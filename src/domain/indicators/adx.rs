//! ADX: the +DI line crossing the -DI line.
//!
//! Each bar's directional move is `high - high[1]` up and `low[1] - low`
//! down; only the larger one counts, both drop to 0 on a tie, and the kept
//! move is scaled to `100 * move / TR` with the true range of the bar.
//! +DI and -DI smooth those with an EMA of factor `2 / (period + 1)` seeded
//! with 0 at bar 0, as `ExponentialMA` does in `ADX.mq5`. The ADX line itself,
//! the smoothed spread of the two, only measures trend strength and has no
//! direction, so the signal is the DI cross.
use crate::domain::cast;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{CrossoverBuffers, IndicatorValue};
use crate::validate_period;

#[derive(Debug, Clone, Copy, PartialEq)]
struct DirectionalMovement {
    plus: f64,
    minus: f64,
}

/// The long buffer is +DI and the short buffer -DI. Both start at bar
/// `period`, the `PLOT_DRAW_BEGIN` of the DI lines in `ADX.mq5`.
pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
) -> Result<CrossoverBuffers, IndicatorError> {
    validate_period!(period, InvalidAdxPeriod);
    let first = cast::i32_to_usize(period);
    let smoothing = 2.0 / (f64::from(period) + 1.0);

    let mut plus_di = 0.0;
    let mut minus_di = 0.0;
    let mut upcross_long_buffer = Vec::with_capacity(points.len().saturating_sub(first));
    let mut upcross_short_buffer = Vec::with_capacity(points.len().saturating_sub(first));
    for (pair, bar) in points.windows(2).zip(1_usize..) {
        let [prev, curr] = pair else {
            continue;
        };
        let movement = directional_movement(prev, curr);
        plus_di = smoothing.mul_add(movement.plus - plus_di, plus_di);
        minus_di = smoothing.mul_add(movement.minus - minus_di, minus_di);
        if bar < first {
            continue;
        }
        upcross_long_buffer.push(IndicatorValue {
            timestamp: curr.timestamp,
            value: plus_di,
        });
        upcross_short_buffer.push(IndicatorValue {
            timestamp: curr.timestamp,
            value: minus_di,
        });
    }

    Ok(CrossoverBuffers {
        upcross_long_buffer,
        upcross_short_buffer,
    })
}

fn directional_movement(prev: &ResampledPointD1, curr: &ResampledPointD1) -> DirectionalMovement {
    let up = (curr.high - prev.high).max(0.0);
    let down = (prev.low - curr.low).max(0.0);
    let true_range = (curr.high - curr.low)
        .abs()
        .max((curr.high - prev.close).abs())
        .max((curr.low - prev.close).abs());
    if true_range == 0.0 || up == down {
        return DirectionalMovement {
            plus: 0.0,
            minus: 0.0,
        };
    }
    if up > down {
        DirectionalMovement {
            plus: 100.0 * up / true_range,
            minus: 0.0,
        }
    } else {
        DirectionalMovement {
            plus: 0.0,
            minus: 100.0 * down / true_range,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;

    fn rp(id: i32, ts: i64, high: f64, low: f64, close: f64) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: close,
            high,
            low,
            close,
            volume: 1000,
            timestamp: ts,
        }
    }

    /// Each bar sits one higher than the last and closes at its high: every
    /// bar moves up by exactly its true range.
    fn rising_points(count: i32) -> Vec<ResampledPointD1> {
        (1..=count)
            .map(|i| {
                let low = f64::from(i);
                rp(i, i64::from(i), low + 1.0, low, low + 1.0)
            })
            .collect()
    }

    fn values(buffer: &[IndicatorValue]) -> Vec<f64> {
        buffer.iter().map(|v| v.value).collect()
    }

    fn timestamps(buffer: &[IndicatorValue]) -> Vec<i64> {
        buffer.iter().map(|v| v.timestamp).collect()
    }

    fn movement(plus: f64, minus: f64) -> DirectionalMovement {
        DirectionalMovement { plus, minus }
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn steady_rise_with_period_one_yields_full_plus_di() {
        let buffers = compute(&rising_points(3), 1).unwrap();
        assert_eq!(values(&buffers.upcross_long_buffer), vec![100.0, 100.0]);
        assert_eq!(values(&buffers.upcross_short_buffer), vec![0.0, 0.0]);
    }

    #[test]
    fn di_smoothing_starts_from_zero() {
        // Period 3 -> factor 1/2 on +DM 100 every bar from 0: bar 1 50,
        // bar 2 75, bar 3 87.5, bar 4 93.75; output starts at bar 3.
        let buffers = compute(&rising_points(5), 3).unwrap();
        assert_eq!(values(&buffers.upcross_long_buffer), vec![87.5, 93.75]);
        assert_eq!(values(&buffers.upcross_short_buffer), vec![0.0, 0.0]);
    }

    #[test]
    fn steady_fall_yields_full_minus_di() {
        let points = vec![rp(1, 1, 3.0, 2.0, 2.0), rp(2, 2, 2.0, 1.0, 1.0)];
        let buffers = compute(&points, 1).unwrap();
        assert_eq!(values(&buffers.upcross_long_buffer), vec![0.0]);
        assert_eq!(values(&buffers.upcross_short_buffer), vec![100.0]);
    }

    #[test]
    fn true_range_reaches_back_to_previous_close() {
        // Gap up: high - low is 2 but high - prev close is 4, so the up move
        // of 3 is 75% of the true range.
        let prev = rp(1, 1, 2.0, 0.0, 1.0);
        let curr = rp(2, 2, 5.0, 3.0, 4.0);
        assert_eq!(directional_movement(&prev, &curr), movement(75.0, 0.0));
    }

    #[test]
    fn equal_up_and_down_moves_cancel() {
        let prev = rp(1, 1, 2.0, 1.0, 1.5);
        let curr = rp(2, 2, 3.0, 0.0, 1.5);
        assert_eq!(directional_movement(&prev, &curr), movement(0.0, 0.0));
    }

    #[test]
    fn only_the_larger_move_counts() {
        // Up 2, down 1 over a range of 4: only the up move is kept.
        let prev = rp(1, 1, 2.0, 1.0, 1.5);
        let curr = rp(2, 2, 4.0, 0.0, 2.0);
        assert_eq!(directional_movement(&prev, &curr), movement(50.0, 0.0));
    }

    #[test]
    fn inside_bar_has_no_directional_movement() {
        let prev = rp(1, 1, 4.0, 0.0, 2.0);
        let curr = rp(2, 2, 3.0, 1.0, 2.0);
        assert_eq!(directional_movement(&prev, &curr), movement(0.0, 0.0));
    }

    #[test]
    fn flat_market_has_no_directional_movement() {
        let prev = rp(1, 1, 1.0, 1.0, 1.0);
        let curr = rp(2, 2, 1.0, 1.0, 1.0);
        assert_eq!(directional_movement(&prev, &curr), movement(0.0, 0.0));
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yield_empty_buffers() {
        let buffers = compute(&[], 14).unwrap();
        assert!(buffers.upcross_long_buffer.is_empty());
        assert!(buffers.upcross_short_buffer.is_empty());
    }

    #[test]
    fn exactly_period_points_yield_empty_buffers() {
        let buffers = compute(&rising_points(3), 3).unwrap();
        assert!(buffers.upcross_long_buffer.is_empty());
        assert!(buffers.upcross_short_buffer.is_empty());
    }

    #[test]
    fn one_point_past_period_yields_single_value_at_last_bar() {
        let buffers = compute(&rising_points(4), 3).unwrap();
        assert_eq!(timestamps(&buffers.upcross_long_buffer), vec![4]);
        assert_eq!(timestamps(&buffers.upcross_short_buffer), vec![4]);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = rising_points(2);
        assert_eq!(
            compute(&points, 0).unwrap_err(),
            IndicatorError::InvalidAdxPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3).unwrap_err(),
            IndicatorError::InvalidAdxPeriod { n: -3 }
        );
    }

    // --- Level 2: real ADX output on real EURUSD data (captured baseline) ---

    /// Baseline captured on the frozen EURUSD fixture with `period = 14` (the
    /// MQL5 default). A direct Python transcription of the `ADX.mq5` loop
    /// agrees to within 2e-14: `mul_add` rounds once where MQL5's
    /// `a * k + b * (1 - k)` rounds several times. Not checked against
    /// MetaTrader output itself.
    #[test]
    fn level2_real_di_pair_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let buffers = compute(&points, 14).unwrap();
        let long = &buffers.upcross_long_buffer;
        let short = &buffers.upcross_short_buffer;

        // 2400 points -> 2400 - 14 values.
        assert_eq!(long.len(), 2386);
        assert_eq!(short.len(), 2386);

        let last_long = long.last().expect("non-empty");
        let last_short = short.last().expect("non-empty");
        assert_eq!(last_long.timestamp, 1646352000);
        assert_eq!(last_short.timestamp, 1646352000);
        assert_eq!(last_long.value, 4.3793446634245035);
        assert_eq!(last_short.value, 32.75517736066793);

        let expected_first_5: &[(i64, f64, f64)] = &[
            (1356307200, 23.02475376605015, 9.464614854417938),
            (1356480000, 23.87192407370314, 8.202666207162213),
            (1356566400, 25.556725731600917, 7.108977379540585),
            (1356652800, 22.149162300720796, 11.447457341269732),
            (1356912000, 19.19594066062469, 9.921129695767101),
        ];
        for (i, (ts, l, s)) in expected_first_5.iter().enumerate() {
            assert_eq!(long[i].timestamp, *ts, "long ts mismatch at {i}");
            assert_eq!(short[i].timestamp, *ts, "short ts mismatch at {i}");
            assert_eq!(long[i].value, *l, "long value mismatch at {i}");
            assert_eq!(short[i].value, *s, "short value mismatch at {i}");
        }
    }
}
