use rayon::prelude::*;

use cached::{Cached, cached};
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
    convert = r#"{ (indicator.id, instrument_of(resampled_points).to_owned()) }"#,
    cache_none = true,
    // Intentional: the lines below describe how the cache locks. They stay
    // because the no-rayon-in-the-body rule cannot be expressed in code.
    // Dedupe concurrent first computations of a key. Locks are per bucket, not
    // per key: every call (hit or miss) waits on its bucket while another key
    // there computes, so use enough buckets to keep collisions rare.
    // The bucket lock is held across the body, so the body must not use rayon:
    // a worker waiting inside it can steal another call in the same bucket and
    // deadlock on the lock it already holds.
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
                let instrument = instrument_of(resampled_points);
                // Skip, don't abort: one misconfigured row must not sink the
                // others. The miss is memoized, so this warn fires once per
                // bad indicator, not once per cell.
                warn!(
                    "indicator skipped: {error}; id={}, instrument={instrument}",
                    indicator.id
                );
                return None;
            }
        };
    Some(indicator_output.create_signal_group(indicator.id, resampled_points))
}

pub fn instrument_of(resampled_points: &[ResampledPointD1]) -> &str {
    resampled_points
        .first()
        .map_or("", |point| point.instrument.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::{indicators_rsi_single, resampled_points_for_rsi};
    use crate::domain::indicator::{Indicator, IndicatorParameters, MovingAverageType};
    use crate::domain::resampled_point::ResampledPointD1;

    // Signal behavior is tested through the macro's uncached companion: the
    // memo is process-wide, so cached calls would share entries across tests.

    #[test]
    fn rsi_indicator_produces_a_signal_group_stamped_with_its_id() {
        // 5 points (1,2,1,2,1), n=3 -> diffs [+1,-1,+1,-1], 2 RSI windows.
        // window [+1,-1,+1] -> rsi 67; window [-1,+1,-1] -> rsi 33.
        // Thresholds 70/30: 67 and 33 never cross 70 or 30 -> no signals.
        let indicator = &indicators_rsi_single()[0];
        let points = resampled_points_for_rsi();
        let group = create_signal_group_no_cache(indicator, &points).unwrap();
        assert_eq!(group.indicator_id, 1);
        assert!(group.long_signals.is_empty());
        assert!(group.short_signals.is_empty());
    }

    #[test]
    fn invalid_indicator_is_skipped_not_aborted() {
        let indicators = vec![
            invalid_macd(100),
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
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].indicator_id, 2);
    }

    /// A MACD whose fast period is not below its slow period.
    fn invalid_macd(id: i32) -> Indicator {
        Indicator {
            id,
            parameters: IndicatorParameters::Macd {
                slow_type: MovingAverageType::Ema,
                slow_n: 12,
                fast_type: MovingAverageType::Ema,
                fast_n: 26,
            },
        }
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
        //   diff  = [-8, 9, 0, 0, 0] -> strict - to + cross at ts 11000,
        //   entered on the next resampled point, ts 12000.
        let closes = [
            100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 82.0, 100.0, 100.0,
            100.0, 100.0,
        ];
        let points = points_from_closes(&closes);
        let indicator = Indicator {
            id: 9001,
            parameters: IndicatorParameters::Macd {
                slow_type: MovingAverageType::Sma,
                slow_n: 2,
                fast_type: MovingAverageType::Ema,
                fast_n: 1,
            },
        };
        let group = create_signal_group_no_cache(&indicator, &points).unwrap();
        assert_eq!(group.indicator_id, 9001);
        assert_eq!(group.long_signals, vec![12000]);
        assert!(group.short_signals.is_empty());
    }

    #[test]
    fn macd_crossing_below_signal_line_produces_short_signal() {
        // Mirror of the long test: the spike 100 -> 118 at index 9 and fall
        // back at 10 give
        //   line  = [0,0,0,0,0,0,0,0,9,-9,0,0,0]
        //   signal = [1, 0, 0, 0, 0]
        //   diff  = [8, -9, 0, 0, 0] -> strict + to - cross at ts 11000,
        //   entered on the next resampled point, ts 12000.
        let closes = [
            100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 118.0, 100.0, 100.0,
            100.0, 100.0,
        ];
        let points = points_from_closes(&closes);
        let indicator = Indicator {
            id: 9002,
            parameters: IndicatorParameters::Macd {
                slow_type: MovingAverageType::Sma,
                slow_n: 2,
                fast_type: MovingAverageType::Ema,
                fast_n: 1,
            },
        };
        let group = create_signal_group_no_cache(&indicator, &points).unwrap();
        assert_eq!(group.indicator_id, 9002);
        assert_eq!(group.short_signals, vec![12000]);
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
                slow_type: MovingAverageType::Sma,
                slow_n: 2,
                fast_type: MovingAverageType::Ema,
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
        assert_eq!(eurusd_groups[0].long_signals, vec![12000]);
        assert_eq!(gbpusd_groups.len(), 1);
        assert_eq!(gbpusd_groups[0].instrument, "GBPUSD");
        assert_eq!(gbpusd_groups[0].short_signals, vec![12000]);
    }

    #[test]
    fn empty_resampled_points_yield_no_signal_group() {
        let indicator = &indicators_rsi_single()[0];
        assert!(create_signal_group_no_cache(indicator, &[]).is_none());
    }

    #[test]
    fn signal_groups_follow_indicator_order() {
        // Shuffled ids, enough for rayon to split the work; unique to this
        // test because the call goes through the memo.
        let ids = [
            9207, 9201, 9214, 9203, 9210, 9216, 9202, 9209, 9205, 9213, 9204, 9215, 9208, 9211,
            9206, 9212,
        ];
        let indicators: Vec<Indicator> = ids
            .iter()
            .map(|&id| Indicator {
                id,
                parameters: IndicatorParameters::Rsi {
                    n: 3,
                    high_threshold: 70,
                    low_threshold: 30,
                },
            })
            .collect();
        let points = resampled_points_for_rsi();

        let result = create_signal_groups(&indicators, &points);

        let result_ids: Vec<i32> = result.iter().map(|group| group.indicator_id).collect();
        assert_eq!(result_ids, ids);
    }

    #[test]
    fn no_indicators_yields_no_signal_groups() {
        let points = resampled_points_for_rsi();
        let result = create_signal_groups(&[], &points);
        assert!(result.is_empty());
    }

    #[test]
    fn invalid_indicator_yields_no_signal_group() {
        let indicator = invalid_macd(100);
        let points = resampled_points_for_rsi();
        assert!(create_signal_group_no_cache(&indicator, &points).is_none());
    }

    #[test]
    fn rsi_crossing_thresholds_produces_signals() {
        // Prices 5,4,3,4 with n=2: diffs [-1,-1,+1], RSI windows:
        //   window [-1,-1] -> gain 0, loss 2 -> rs 0 -> rsi 0 (ts 3000)
        //   window [-1,+1] -> gain 1, loss 1 -> rs 1 -> rsi 50 (ts 4000)
        // Thresholds 70/30: prev rsi 0 < low 30, curr rsi 50 > low 30 -> long
        // at ts 4000, entered on the next resampled point, ts 5000.
        let points = points_from_closes(&[5.0, 4.0, 3.0, 4.0, 4.0]);
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
        assert_eq!(group.long_signals, vec![5000]);
        assert!(group.short_signals.is_empty());
    }
}
