//! Ichimoku Kinko Hyo: the Tenkan-sen crossing the Kijun-sen.
//!
//! Tenkan-sen and Kijun-sen are each the midpoint of the highest high and
//! lowest low over their own number of bars. Ported from `Ichimoku.mq5`. The
//! signal is the Tenkan-Kijun cross, so the cloud is not computed: Senkou
//! Span A is only their average plotted ahead, Senkou Span B a third midpoint
//! plotted ahead, and the Chikou span the close plotted back.
use crate::domain::cast;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{CrossoverBuffers, IndicatorValue};
use crate::validate_period;

/// The long buffer is the Tenkan-sen and the short buffer the Kijun-sen.
/// Both start at bar `max(tenkan_period, kijun_period)`, the later of the two
/// lines' `PLOT_DRAW_BEGIN` in `Ichimoku.mq5`.
pub fn compute(
    points: &[ResampledPointD1],
    tenkan_period: i32,
    kijun_period: i32,
) -> Result<CrossoverBuffers, IndicatorError> {
    validate_period!(
        tenkan_period, InvalidIchimokuTenkanPeriod;
        kijun_period, InvalidIchimokuKijunPeriod;
    );
    let tenkan = cast::i32_to_usize(tenkan_period);
    let kijun = cast::i32_to_usize(kijun_period);
    let first = tenkan.max(kijun);

    let mut upcross_long_buffer = Vec::with_capacity(points.len().saturating_sub(first));
    let mut upcross_short_buffer = Vec::with_capacity(points.len().saturating_sub(first));
    for window in points.windows(first.saturating_add(1)) {
        let Some(last) = window.last() else {
            continue;
        };
        upcross_long_buffer.push(IndicatorValue {
            timestamp: last.timestamp,
            value: high_low_midpoint(window, tenkan),
        });
        upcross_short_buffer.push(IndicatorValue {
            timestamp: last.timestamp,
            value: high_low_midpoint(window, kijun),
        });
    }

    Ok(CrossoverBuffers {
        upcross_long_buffer,
        upcross_short_buffer,
    })
}

/// Midpoint of the highest high and lowest low over the last `period` points
/// of the window.
fn high_low_midpoint(window: &[ResampledPointD1], period: usize) -> f64 {
    let mut highest = f64::NEG_INFINITY;
    let mut lowest = f64::INFINITY;
    for point in window.iter().rev().take(period) {
        highest = highest.max(point.high);
        lowest = lowest.min(point.low);
    }
    f64::midpoint(highest, lowest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;

    fn rp(id: i32, ts: i64, high: f64, low: f64) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: low,
            high,
            low,
            close: high,
            volume: 1000,
            timestamp: ts,
        }
    }

    /// Bar `i` spans `[i, i + 1]`.
    fn rising_points(count: i32) -> Vec<ResampledPointD1> {
        (1..=count)
            .map(|i| rp(i, i64::from(i), f64::from(i) + 1.0, f64::from(i)))
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
    fn lines_are_midpoints_of_their_own_windows() {
        // Bar 3 (ts 4) spans [4, 5]. Tenkan 2 covers ts 3..4: (5 + 3) / 2;
        // Kijun 3 covers ts 2..4: (5 + 2) / 2.
        let buffers = compute(&rising_points(5), 2, 3).unwrap();
        assert_eq!(values(&buffers.upcross_long_buffer), vec![4.0, 5.0]);
        assert_eq!(values(&buffers.upcross_short_buffer), vec![3.5, 4.5]);
        assert_eq!(timestamps(&buffers.upcross_long_buffer), vec![4, 5]);
        assert_eq!(timestamps(&buffers.upcross_short_buffer), vec![4, 5]);
    }

    #[test]
    fn window_takes_extreme_high_and_low_from_different_bars() {
        let points = vec![
            rp(1, 1, 10.0, 9.0),
            rp(2, 2, 20.0, 5.0),
            rp(3, 3, 12.0, 8.0),
            rp(4, 4, 11.0, 10.0),
        ];
        // Kijun 3 at bar 3 covers ts 2..4: (20 + 5) / 2. Tenkan 1 is the bar
        // itself: (11 + 10) / 2.
        let buffers = compute(&points, 1, 3).unwrap();
        assert_eq!(values(&buffers.upcross_long_buffer), vec![10.5]);
        assert_eq!(values(&buffers.upcross_short_buffer), vec![12.5]);
    }

    #[test]
    fn tenkan_longer_than_kijun_starts_at_tenkan_period() {
        let buffers = compute(&rising_points(5), 3, 1).unwrap();
        assert_eq!(timestamps(&buffers.upcross_long_buffer), vec![4, 5]);
        assert_eq!(values(&buffers.upcross_long_buffer), vec![3.5, 4.5]);
        assert_eq!(values(&buffers.upcross_short_buffer), vec![4.5, 5.5]);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yield_empty_buffers() {
        let buffers = compute(&[], 9, 26).unwrap();
        assert!(buffers.upcross_long_buffer.is_empty());
        assert!(buffers.upcross_short_buffer.is_empty());
    }

    #[test]
    fn exactly_period_points_yield_empty_buffers() {
        let buffers = compute(&rising_points(3), 2, 3).unwrap();
        assert!(buffers.upcross_long_buffer.is_empty());
        assert!(buffers.upcross_short_buffer.is_empty());
    }

    #[test]
    fn one_point_past_period_yields_single_value_at_last_bar() {
        let buffers = compute(&rising_points(4), 2, 3).unwrap();
        assert_eq!(timestamps(&buffers.upcross_long_buffer), vec![4]);
        assert_eq!(timestamps(&buffers.upcross_short_buffer), vec![4]);
    }

    #[test]
    fn non_positive_tenkan_period_returns_error() {
        let points = rising_points(2);
        assert_eq!(
            compute(&points, 0, 26).unwrap_err(),
            IndicatorError::InvalidIchimokuTenkanPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3, 26).unwrap_err(),
            IndicatorError::InvalidIchimokuTenkanPeriod { n: -3 }
        );
    }

    #[test]
    fn non_positive_kijun_period_returns_error() {
        let points = rising_points(2);
        assert_eq!(
            compute(&points, 9, 0).unwrap_err(),
            IndicatorError::InvalidIchimokuKijunPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, 9, -3).unwrap_err(),
            IndicatorError::InvalidIchimokuKijunPeriod { n: -3 }
        );
    }

    // --- Level 2: real Ichimoku output on real EURUSD data (captured baseline) ---

    /// Baseline captured on the frozen EURUSD fixture with `tenkan_period = 9`
    /// and `kijun_period = 26` (the MQL5 defaults). A direct Python
    /// transcription of the `Ichimoku.mq5` loop agrees bit for bit. Not
    /// checked against MetaTrader output itself.
    #[test]
    fn level2_real_tenkan_kijun_pair_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let buffers = compute(&points, 9, 26).unwrap();
        let long = &buffers.upcross_long_buffer;
        let short = &buffers.upcross_short_buffer;

        // 2400 points -> 2400 - 26 values.
        assert_eq!(long.len(), 2374);
        assert_eq!(short.len(), 2374);

        let last_long = long.last().expect("non-empty");
        let last_short = short.last().expect("non-empty");
        assert_eq!(last_long.timestamp, 1646352000);
        assert_eq!(last_short.timestamp, 1646352000);
        assert_eq!(last_long.value, 1.11261);
        assert_eq!(last_short.value, 1.1190099999999998);

        let expected_first_5: &[(i64, f64, f64)] = &[
            (1357776000, 1.314835, 1.309225),
            (1357862400, 1.318095, 1.312065),
            (1358121600, 1.32003, 1.314),
            (1358208000, 1.32003, 1.31443),
            (1358294400, 1.32003, 1.31657),
        ];
        for (i, (ts, l, s)) in expected_first_5.iter().enumerate() {
            assert_eq!(long[i].timestamp, *ts, "long ts mismatch at {i}");
            assert_eq!(short[i].timestamp, *ts, "short ts mismatch at {i}");
            assert_eq!(long[i].value, *l, "long value mismatch at {i}");
            assert_eq!(short[i].value, *s, "short value mismatch at {i}");
        }
    }
}
