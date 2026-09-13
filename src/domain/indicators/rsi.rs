use crate::domain::indicators::{IndicatorError, PriceField};
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, ThresholdSeries};
use crate::validate_period;

/// A price diff tagged with the timestamp of the newer point.
#[derive(Debug, Clone, Copy)]
struct PriceDiff {
    timestamp: i64,
    diff: f64,
}

/// Compute RSI over the resampled points for the given price field.
///
/// Mirrors the source algorithm: build per-step price diffs, then for each
/// window of `n` consecutive diffs sum the gains and losses, compute
/// `rs = gain / loss` (with `+∞` when loss is 0 and `0` when gain is 0),
/// `raw_rsi = 100 - 100 / (1 + rs)`, and round to the nearest integer.
pub fn compute(
    points: &[ResampledPointD1],
    field: PriceField,
    n: i32,
    high_threshold: f64,
    low_threshold: f64,
) -> Result<ThresholdSeries, IndicatorError> {
    validate_period!(n, InvalidRsiPeriod);
    let period = n as usize;

    // Build per-step price diffs: diff[i] = price[i+1] - price[i].
    let diffs: Vec<PriceDiff> = points
        .windows(2)
        .map(|w| {
            let prev = &w[0];
            let cur = &w[1];
            PriceDiff {
                timestamp: cur.timestamp,
                diff: field.select(cur) - field.select(prev),
            }
        })
        .collect();

    // Each RSI point needs `period` consecutive diffs.
    let indicator_values: Vec<IndicatorValue> = if diffs.len() >= period {
        diffs.windows(period).map(compute_rsi_point).collect()
    } else {
        Vec::new()
    };

    Ok(ThresholdSeries {
        items: indicator_values,
        high_threshold,
        low_threshold,
    })
}

/// Compute one RSI value for a window of price diffs.
///
/// The window comes from `windows(period)` with `period >= 1` (validated),
/// so it is never empty; the loop leaves `timestamp` at the last diff's.
fn compute_rsi_point(window: &[PriceDiff]) -> IndicatorValue {
    let mut gain: f64 = 0.0;
    let mut loss: f64 = 0.0;
    let mut timestamp: i64 = 0;
    for d in window {
        if d.diff > 0.0 {
            gain += d.diff;
        } else if d.diff < 0.0 {
            loss += d.diff.abs();
        }
        timestamp = d.timestamp;
    }

    let rs = if loss == 0.0 {
        f64::INFINITY
    } else if gain == 0.0 {
        0.0
    } else {
        gain / loss
    };

    let raw_rsi = 100.0 - (100.0 / (1.0 + rs));
    let value = raw_rsi.round();

    IndicatorValue { timestamp, value }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1};
    use crate::indicator_values;

    fn rp(id: i32, ts: i64, close: f64) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 1000,
            timestamp: ts,
            high_low_order: HighLowOrder::HighFirst,
        }
    }

    #[test]
    fn all_gains_yield_max_rsi() {
        // Prices strictly rising -> all diffs positive -> loss=0 -> rs=inf -> rsi=100.
        let points = vec![
            rp(1, 1, 1.0),
            rp(2, 2, 2.0),
            rp(3, 3, 3.0),
            rp(4, 4, 4.0),
            rp(5, 5, 5.0),
        ];
        let result = compute(&points, PriceField::Close, 3, 70.0, 30.0).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].timestamp, 4);
        assert_eq!(vals[0].value, 100.0);
        assert_eq!(vals[1].timestamp, 5);
        assert_eq!(vals[1].value, 100.0);
    }

    #[test]
    fn all_losses_yield_min_rsi() {
        // Prices strictly falling -> all diffs negative -> gain=0 -> rs=0 -> rsi=0.
        let points = vec![
            rp(1, 1, 5.0),
            rp(2, 2, 4.0),
            rp(3, 3, 3.0),
            rp(4, 4, 2.0),
            rp(5, 5, 1.0),
        ];
        let result = compute(&points, PriceField::Close, 3, 70.0, 30.0).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].timestamp, 4);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[1].timestamp, 5);
        assert_eq!(vals[1].value, 0.0);
    }

    #[test]
    fn alternating_gains_losses_round_correctly() {
        // diffs: +1, -1, +1, -1 (n=3)
        // window [+1,-1,+1]: gain=2, loss=1, rs=2, rsi=100-100/3=66.67 -> 67
        // window [-1,+1,-1]: gain=1, loss=2, rs=0.5, rsi=100-100/1.5=33.33 -> 33
        let points = vec![
            rp(1, 1, 1.0),
            rp(2, 2, 2.0),
            rp(3, 3, 1.0),
            rp(4, 4, 2.0),
            rp(5, 5, 1.0),
        ];
        let result = compute(&points, PriceField::Close, 3, 70.0, 30.0).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].timestamp, 4);
        assert_eq!(vals[0].value, 67.0);
        assert_eq!(vals[1].timestamp, 5);
        assert_eq!(vals[1].value, 33.0);
    }

    #[test]
    fn timestamps_track_newer_point_in_window() {
        // The RSI point timestamp is the last (newest) diff's timestamp.
        let points = vec![
            rp(1, 100, 1.0),
            rp(2, 200, 2.0),
            rp(3, 300, 3.0),
            rp(4, 400, 4.0),
        ];
        let result = compute(&points, PriceField::Close, 2, 70.0, 30.0).unwrap();
        let vals = indicator_values!(result);
        // n=2, diffs at ts 200,300,400 -> windows: [200,300]->ts300, [300,400]->ts400
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].timestamp, 300);
        assert_eq!(vals[1].timestamp, 400);
    }

    #[test]
    fn thresholds_carried_through_unchanged() {
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0), rp(3, 3, 3.0), rp(4, 4, 4.0)];
        let result = compute(&points, PriceField::Close, 2, 75.0, 25.0).unwrap();
        assert_eq!(result.high_threshold, 75.0);
        assert_eq!(result.low_threshold, 25.0);
    }

    #[test]
    fn not_enough_diffs_yields_empty_items() {
        // 3 points -> 2 diffs. n=3 needs 3 diffs -> empty result, no error.
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0), rp(3, 3, 3.0)];
        let result = compute(&points, PriceField::Close, 3, 70.0, 30.0).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn empty_points_yields_empty_items() {
        let result = compute(&[], PriceField::Close, 3, 70.0, 30.0).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_one_window() {
        // 4 points -> 3 diffs -> n=3 yields exactly 1 window.
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0), rp(3, 3, 3.0), rp(4, 4, 4.0)];
        let result = compute(&points, PriceField::Close, 3, 70.0, 30.0).unwrap();
        assert_eq!(indicator_values!(result).len(), 1);
        assert_eq!(indicator_values!(result)[0].value, 100.0);
    }

    #[test]
    fn invalid_period_zero_returns_error() {
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = compute(&points, PriceField::Close, 0, 70.0, 30.0).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidRsiPeriod { n: 0 });
    }

    #[test]
    fn invalid_period_negative_returns_error() {
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = compute(&points, PriceField::Close, -1, 70.0, 30.0).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidRsiPeriod { n: -1 });
    }

    #[test]
    fn price_field_open_uses_open_not_close() {
        // open differs from close; using Open should pick open diffs.
        let mut p1 = rp(1, 1, 10.0);
        p1.open = 1.0;
        let mut p2 = rp(2, 2, 10.0);
        p2.open = 3.0;
        let mut p3 = rp(3, 3, 10.0);
        p3.open = 5.0;
        let mut p4 = rp(4, 4, 10.0);
        p4.open = 7.0;
        // Open diffs: +2,+2,+2 -> all gains -> rsi 100
        let result = compute(&[p1, p2, p3, p4], PriceField::Open, 2, 70.0, 30.0).unwrap();
        assert_eq!(indicator_values!(result).len(), 2);
        assert_eq!(indicator_values!(result)[0].value, 100.0);
    }
}
