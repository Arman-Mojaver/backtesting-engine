use rayon::prelude::*;

use cached::{cached, Cached};
use log::warn;

use crate::domain::indicator::Indicator;
use crate::domain::indicators::create_indicator_output;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::IndicatorOutput;
use crate::domain::signal_group::SignalGroup;

pub fn create_signal_groups(
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
) -> Vec<SignalGroup> {
    indicators
        .par_iter()
        .filter_map(|indicator| create_signal_group(indicator, resampled_points))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalGroupCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
}

pub fn signal_group_cache_stats() -> SignalGroupCacheStats {
    let cache = SIGNAL_GROUP_CACHE.read();
    SignalGroupCacheStats {
        // `UnboundCache` always tracks metrics, so these are never `None`.
        hits: cache.cache_hits().unwrap_or_default(),
        misses: cache.cache_misses().unwrap_or_default(),
        entries: cache.cache_size(),
    }
}

/// Memoized on `(indicator_id, instrument)`: parameters are immutable per
/// id and an instrument's resampled points never change, so the identity is
/// a stable key without hashing the large inputs.
#[cached(
    name = "SIGNAL_GROUP_CACHE",
    key = "(i32, String)",
    convert = r#"{ (indicator.id, resampled_points.first().map(|p| p.instrument.clone()).unwrap_or_default()) }"#,
    cache_none = true,
    // Dedupe concurrent first computations of a key. Locks are per bucket, not
    // per key: every call (hit or miss) waits on its bucket while another key
    // there computes, so use enough buckets to keep collisions rare.
    sync_writes = "by_key",
    sync_writes_buckets = 1024,
    // Hits take a shared read lock instead of the exclusive write lock.
    unsync_reads = true
)]
fn create_signal_group(
    indicator: &Indicator,
    resampled_points: &[ResampledPointD1],
) -> Option<SignalGroup> {
    let indicator_output: IndicatorOutput =
        match create_indicator_output(&indicator.parameters, resampled_points) {
            Ok(output) => output,
            Err(error) => {
                let instrument = resampled_points
                    .first()
                    .map_or("", |point| point.instrument.as_str());
                // Skip, don't abort: one misconfigured row must not sink the
                // others. The miss is memoized, so this warn fires once per
                // bad indicator, not once per cell.
                warn!("indicator {} ({instrument}) skipped: {error}", indicator.id);
                return None;
            }
        };
    Some(indicator_output.create_signal_group(indicator.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::{indicators_rsi_single, resampled_points_for_rsi};
    use crate::domain::indicator::{Indicator, IndicatorParameters};
    use crate::domain::resampled_point::ResampledPointD1;

    // Signal behavior is tested through the macro's uncached companion: the
    // memo is process-wide, so cached calls would share entries across tests.

    #[test]
    fn rsi_indicator_produces_a_signal_group_stamped_with_its_id() {
        // 5 points (1,2,1,2,1), n=3 -> diffs [+1,-1,+1,-1], 2 RSI windows.
        // window [+1,-1,+1] -> rsi 67; window [-1,+1,-1] -> rsi 33.
        // Thresholds 70/30: 67 and 33 never cross 70 or 30 -> no signals.
        // So this verifies the path runs and returns a group with no signals.
        let indicator = &indicators_rsi_single()[0];
        let points = resampled_points_for_rsi();
        let group = create_signal_group_no_cache(indicator, &points).unwrap();
        assert_eq!(group.indicator_id, 1);
        // RSI 67/33 sit between 30 and 70 -> no threshold crossing -> no signals.
        assert!(group.long_signals.is_empty());
        assert!(group.short_signals.is_empty());
    }

    #[test]
    fn invalid_indicator_is_skipped_not_aborted() {
        // A MACD with an unknown moving-average type errors -> produces no
        // group, but the RSI indicator still produces one.
        let indicators = vec![
            Indicator {
                id: 100,
                parameters: IndicatorParameters::Macd {
                    slow_type: "ema".to_string(),
                    slow_n: 26,
                    fast_type: "wma".to_string(),
                    fast_n: 12,
                },
            },
            Indicator {
                id: 2,
                parameters: IndicatorParameters::Rsi {
                    n: 3,
                    high_threshold: 70,
                    low_threshold: 30,
                },
            },
        ];
        let points = resampled_points_for_rsi();
        let result = create_signal_groups(&indicators, &points);
        // Only the RSI indicator yields a group.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].indicator_id, 2);
    }

    /// Build points from an explicit close series, ts = (i + 1) * 1000.
    fn points_from_closes(closes: &[f64]) -> Vec<ResampledPointD1> {
        closes
            .iter()
            .enumerate()
            .map(|(i, close)| {
                let i3 = i32::try_from(i).unwrap();
                ResampledPointD1 {
                    id: i3.saturating_add(1),
                    instrument: "EURUSD".into(),
                    open: *close,
                    high: *close,
                    low: *close,
                    close: *close,
                    volume: 1000,
                    timestamp: i64::from(i3.saturating_add(1)).saturating_mul(1000),
                }
            })
            .collect()
    }

    #[test]
    fn macd_crossing_above_signal_line_produces_long_signal() {
        // fast EMA(1) (identity), slow SMA(2): the MACD line is half the
        // bar-to-bar close change, evaluated at the later bar. The dip
        // 100 -> 82 at index 9 and recovery at 10 give
        //   line  = [0,0,0,0,0,0,0,0,-9,9,0,0,0] (ts 2000..14000)
        //   signal = SMA(line, 9) = [-1, 0, 0, 0, 0]
        //   diff  = [-8, 9, 0, 0, 0] -> strict - to + cross at ts 11000.
        let closes = [
            100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 82.0, 100.0, 100.0,
            100.0, 100.0,
        ];
        let points = points_from_closes(&closes);
        let indicator = Indicator {
            id: 9001,
            parameters: IndicatorParameters::Macd {
                slow_type: "sma".to_string(),
                slow_n: 2,
                fast_type: "ema".to_string(),
                fast_n: 1,
            },
        };
        let group = create_signal_group_no_cache(&indicator, &points).unwrap();
        assert_eq!(group.indicator_id, 9001);
        assert_eq!(group.long_signals, vec![11000]);
        assert!(group.short_signals.is_empty());
    }

    #[test]
    fn macd_crossing_below_signal_line_produces_short_signal() {
        // Mirror of the long test: the spike 100 -> 118 at index 9 and fall
        // back at 10 give
        //   line  = [0,0,0,0,0,0,0,0,9,-9,0,0,0]
        //   signal = [1, 0, 0, 0, 0]
        //   diff  = [8, -9, 0, 0, 0] -> strict + to - cross at ts 11000.
        let closes = [
            100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 118.0, 100.0, 100.0,
            100.0, 100.0,
        ];
        let points = points_from_closes(&closes);
        let indicator = Indicator {
            id: 9002,
            parameters: IndicatorParameters::Macd {
                slow_type: "sma".to_string(),
                slow_n: 2,
                fast_type: "ema".to_string(),
                fast_n: 1,
            },
        };
        let group = create_signal_group_no_cache(&indicator, &points).unwrap();
        assert_eq!(group.indicator_id, 9002);
        assert_eq!(group.short_signals, vec![11000]);
        assert!(group.long_signals.is_empty());
    }

    #[test]
    fn same_indicator_on_different_instruments_yields_distinct_groups() {
        // Goes through the memo on purpose, with an id no other cached test
        // uses: a key without the instrument would return EURUSD's group for
        // GBPUSD.
        let indicators = vec![Indicator {
            id: 9101,
            parameters: IndicatorParameters::Macd {
                slow_type: "sma".to_string(),
                slow_n: 2,
                fast_type: "ema".to_string(),
                fast_n: 1,
            },
        }];
        let dip = [
            100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 82.0, 100.0, 100.0,
            100.0, 100.0,
        ];
        let spike = [
            100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 118.0, 100.0, 100.0,
            100.0, 100.0,
        ];
        let eurusd = points_from_closes(&dip);
        let gbpusd: Vec<ResampledPointD1> = points_from_closes(&spike)
            .into_iter()
            .map(|point| ResampledPointD1 {
                instrument: "GBPUSD".into(),
                ..point
            })
            .collect();

        let eurusd_groups = create_signal_groups(&indicators, &eurusd);
        let gbpusd_groups = create_signal_groups(&indicators, &gbpusd);

        assert_eq!(eurusd_groups.len(), 1);
        assert_eq!(eurusd_groups[0].instrument, "EURUSD");
        assert_eq!(eurusd_groups[0].long_signals, vec![11000]);
        assert_eq!(gbpusd_groups.len(), 1);
        assert_eq!(gbpusd_groups[0].instrument, "GBPUSD");
        assert_eq!(gbpusd_groups[0].short_signals, vec![11000]);
    }

    #[test]
    fn no_indicators_yields_no_signal_groups() {
        let points = resampled_points_for_rsi();
        let result = create_signal_groups(&[], &points);
        assert!(result.is_empty());
    }

    #[test]
    fn invalid_indicator_yields_no_signal_group() {
        // An unknown moving-average type is a configuration error.
        let indicator = Indicator {
            id: 100,
            parameters: IndicatorParameters::Macd {
                slow_type: "ema".to_string(),
                slow_n: 26,
                fast_type: "wma".to_string(),
                fast_n: 12,
            },
        };
        let points = resampled_points_for_rsi();
        assert!(create_signal_group_no_cache(&indicator, &points).is_none());
    }

    #[test]
    fn rsi_crossing_thresholds_produces_signals() {
        // Prices 5,4,3,4 with n=2: diffs [-1,-1,+1], RSI windows:
        //   window [-1,-1] -> gain 0, loss 2 -> rs 0 -> rsi 0 (ts 3000)
        //   window [-1,+1] -> gain 1, loss 1 -> rs 1 -> rsi 50 (ts 4000)
        // Thresholds 70/30: prev rsi 0 < low 30, curr rsi 50 > low 30 -> long.
        let points = vec![
            ResampledPointD1 {
                id: 1,
                instrument: "EURUSD".into(),
                open: 5.0,
                high: 5.1,
                low: 4.9,
                close: 5.0,
                volume: 1000,
                timestamp: 1000,
            },
            ResampledPointD1 {
                id: 2,
                instrument: "EURUSD".into(),
                open: 4.0,
                high: 4.1,
                low: 3.9,
                close: 4.0,
                volume: 1000,
                timestamp: 2000,
            },
            ResampledPointD1 {
                id: 3,
                instrument: "EURUSD".into(),
                open: 3.0,
                high: 3.1,
                low: 2.9,
                close: 3.0,
                volume: 1000,
                timestamp: 3000,
            },
            ResampledPointD1 {
                id: 4,
                instrument: "EURUSD".into(),
                open: 4.0,
                high: 4.1,
                low: 3.9,
                close: 4.0,
                volume: 1000,
                timestamp: 4000,
            },
        ];
        let indicator = Indicator {
            id: 7001,
            parameters: IndicatorParameters::Rsi {
                n: 2,
                high_threshold: 70,
                low_threshold: 30,
            },
        };
        let group = create_signal_group_no_cache(&indicator, &points).unwrap();
        assert_eq!(group.indicator_id, 7001);
        // RSI 0 -> 50 crosses above low threshold 30 -> long signal at ts 4000.
        assert_eq!(group.long_signals, vec![4000]);
        assert!(group.short_signals.is_empty());
    }
}
