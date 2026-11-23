//! VIDYA: the close crossing Chande's variable index dynamic average.
//!
//! `CMO = (sum_up - sum_down) / (sum_up + sum_down)` over the last
//! `cmo_period` close-to-close moves, `f = 2 / (ema_period + 1)`, and
//! `VIDYA = close * f * |CMO| + VIDYA[1] * (1 - f * |CMO|)`, seeded with the
//! close at bar `ema_period + cmo_period - 2`. Ported from `VIDYA.mq5`
//! (`PRICE_CLOSE`). VIDYA is a price overlay, so its signal is the close
//! crossing the VIDYA line.
use crate::domain::cast;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{CrossoverBuffers, IndicatorValue};
use crate::empty_crossover;
use crate::validate_period;

/// The long buffer is the close; the short buffer is the VIDYA line. Both
/// start at bar `ema_period + cmo_period - 1`, where `VIDYA.mq5` starts.
pub fn compute(
    points: &[ResampledPointD1],
    cmo_period: i32,
    ema_period: i32,
) -> Result<CrossoverBuffers, IndicatorError> {
    validate_period!(cmo_period, InvalidVidyaCmoPeriod; ema_period, InvalidVidyaEmaPeriod);
    let cmo_len = cast::i32_to_usize(cmo_period);
    let ema_len = cast::i32_to_usize(ema_period);
    let first = ema_len.saturating_add(cmo_len).saturating_sub(1);
    let smoothing = 2.0 / (f64::from(ema_period) + 1.0);

    let closes: Vec<f64> = points.iter().map(|pt| pt.close).collect();
    let Some(&seed) = closes.get(first.saturating_sub(1)) else {
        empty_crossover!();
    };

    let mut vidya = seed;
    let mut upcross_long_buffer = Vec::with_capacity(points.len().saturating_sub(first));
    let mut upcross_short_buffer = Vec::with_capacity(points.len().saturating_sub(first));
    for (window, point) in closes
        .windows(cmo_len.saturating_add(1))
        .skip(ema_len.saturating_sub(1))
        .zip(points.iter().skip(first))
    {
        let alpha = smoothing * chande_momentum(window).abs();
        vidya = alpha.mul_add(point.close - vidya, vidya);
        upcross_long_buffer.push(IndicatorValue {
            timestamp: point.timestamp,
            value: point.close,
        });
        upcross_short_buffer.push(IndicatorValue {
            timestamp: point.timestamp,
            value: vidya,
        });
    }

    Ok(CrossoverBuffers {
        upcross_long_buffer,
        upcross_short_buffer,
    })
}

/// Net move over total path, signed: 1 for a straight rise, -1 for a straight
/// fall, 0 for a flat window.
fn chande_momentum(window: &[f64]) -> f64 {
    let (sum_up, sum_down) = window.iter().zip(window.iter().skip(1)).fold(
        (0.0, 0.0),
        |(sum_up, sum_down), (prev, curr)| {
            let diff = curr - prev;
            if diff > 0.0 {
                (sum_up + diff, sum_down)
            } else {
                (sum_up, sum_down - diff)
            }
        },
    );
    let total = sum_up + sum_down;
    if total == 0.0 {
        return 0.0;
    }
    (sum_up - sum_down) / total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;

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
        }
    }

    fn points_with_closes(closes: &[f64]) -> Vec<ResampledPointD1> {
        closes
            .iter()
            .zip(1..)
            .map(|(&close, i)| rp(i, i64::from(i), close))
            .collect()
    }

    fn values(buffer: &[IndicatorValue]) -> Vec<f64> {
        buffer.iter().map(|v| v.value).collect()
    }

    fn timestamps(buffer: &[IndicatorValue]) -> Vec<i64> {
        buffer.iter().map(|v| v.timestamp).collect()
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn constant_close_yields_vidya_equal_to_close() {
        let points = points_with_closes(&[10.0; 6]);
        let result = compute(&points, 2, 3).unwrap();
        assert_eq!(values(&result.upcross_short_buffer), vec![10.0; 2]);
    }

    #[test]
    fn straight_rise_uses_full_ema_factor() {
        // closes [1, 2, 3], cmo 2, ema 1: |CMO| = 1 and f = 1, so VIDYA = close.
        let points = points_with_closes(&[1.0, 2.0, 3.0]);
        let result = compute(&points, 2, 1).unwrap();
        assert_eq!(values(&result.upcross_short_buffer), vec![3.0]);
    }

    #[test]
    fn straight_fall_uses_full_ema_factor_too() {
        // closes [3, 2, 2, 1], cmo 1, ema 3: the last move is a fall, so
        // |CMO| = 1, f = 1/2. Seed 2 at bar 3: VIDYA = 2 + 1/2 * (1 - 2) = 1.5.
        let points = points_with_closes(&[3.0, 2.0, 2.0, 1.0]);
        let result = compute(&points, 1, 3).unwrap();
        assert_eq!(values(&result.upcross_short_buffer), vec![1.5]);
    }

    #[test]
    fn partial_momentum_scales_the_ema_factor() {
        // closes [1, 3, 2], cmo 2, ema 1: sum_up 2, sum_down 1, CMO = 1/3,
        // f = 1. Seed 3: VIDYA = 3 + 1/3 * (2 - 3) = 8/3.
        let points = points_with_closes(&[1.0, 3.0, 2.0]);
        let result = compute(&points, 2, 1).unwrap();
        let vidya = values(&result.upcross_short_buffer);
        assert_eq!(vidya.len(), 1);
        assert!((vidya[0] - 8.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn flat_window_holds_the_previous_value() {
        // closes [1, 5, 5, 5], cmo 2, ema 2: seed 5 at bar 2, then CMO over
        // [5, 5, 5] is 0, so VIDYA stays 5 while the window is flat.
        let points = points_with_closes(&[1.0, 5.0, 5.0, 5.0]);
        let result = compute(&points, 2, 2).unwrap();
        assert_eq!(values(&result.upcross_short_buffer), vec![5.0]);
    }

    #[test]
    fn long_buffer_is_the_close() {
        let points = points_with_closes(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let result = compute(&points, 2, 2).unwrap();
        assert_eq!(values(&result.upcross_long_buffer), vec![4.0, 5.0]);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffers() {
        let result = compute(&[], 9, 12).unwrap();
        assert!(result.upcross_long_buffer.is_empty());
        assert!(result.upcross_short_buffer.is_empty());
    }

    #[test]
    fn exactly_warm_up_points_yields_empty_buffers() {
        let points = points_with_closes(&[10.0; 4]);
        let result = compute(&points, 2, 3).unwrap();
        assert!(result.upcross_long_buffer.is_empty());
        assert!(result.upcross_short_buffer.is_empty());
    }

    #[test]
    fn one_point_past_warm_up_yields_single_value_at_last_bar() {
        let points = points_with_closes(&[10.0; 5]);
        let result = compute(&points, 2, 3).unwrap();
        assert_eq!(timestamps(&result.upcross_short_buffer), vec![5]);
        assert_eq!(timestamps(&result.upcross_long_buffer), vec![5]);
    }

    #[test]
    fn non_positive_periods_return_errors() {
        let points = points_with_closes(&[1.0, 2.0]);
        assert_eq!(
            compute(&points, 0, 12).unwrap_err(),
            IndicatorError::InvalidVidyaCmoPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, 9, -1).unwrap_err(),
            IndicatorError::InvalidVidyaEmaPeriod { n: -1 }
        );
    }

    // --- Level 2: real VIDYA output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured by running `vidya::compute` on the frozen
    /// EURUSD fixture with `cmo_period = 9, ema_period = 12` (the MQL5
    /// defaults). Not ground truth: a regression baseline to compare later
    /// against an external source.
    #[test]
    fn level2_real_vidya_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points, 9, 12).unwrap();
        let vidya = &result.upcross_short_buffer;

        // 2400 points -> 2400 - (12 + 9 - 1) values.
        assert_eq!(vidya.len(), 2380);
        assert_eq!(result.upcross_long_buffer.len(), 2380);

        assert_eq!(vidya[0].timestamp, 1357084800);
        assert_eq!(vidya.last().expect("non-empty").timestamp, 1646352000);
        assert_eq!(vidya.last().expect("non-empty").value, 1.1233669528388128);

        let expected_first_5: &[(i64, f64)] = &[
            (1357084800, 1.320518493560094),
            (1357171200, 1.3188345797372143),
            (1357257600, 1.318083305541124),
            (1357516800, 1.317932695639932),
            (1357603200, 1.3173439474536741),
        ];
        for (i, (ts, val)) in expected_first_5.iter().enumerate() {
            assert_eq!(vidya[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(vidya[i].value, *val, "value mismatch at index {i}");
        }
    }
}
