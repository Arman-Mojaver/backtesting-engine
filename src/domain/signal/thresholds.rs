use itertools::Itertools;

use crate::domain::signal::{IndicatorValue, ThresholdBuffer};

pub fn detect_signals(buffer: &ThresholdBuffer) -> (Vec<i64>, Vec<i64>) {
    level_crosses(&buffer.items, buffer.low_threshold, buffer.high_threshold)
}

pub fn level_crosses(
    items: &[IndicatorValue],
    long_level: f64,
    short_level: f64,
) -> (Vec<i64>, Vec<i64>) {
    (
        crosses_above(items, long_level),
        crosses_below(items, short_level),
    )
}

fn crosses_above(items: &[IndicatorValue], level: f64) -> Vec<i64> {
    items
        .iter()
        .filter(|item| item.value != level)
        .tuple_windows()
        .filter(|(prev, curr)| prev.value < level && curr.value > level)
        .map(|(_, curr)| curr.timestamp)
        .collect()
}

fn crosses_below(items: &[IndicatorValue], level: f64) -> Vec<i64> {
    items
        .iter()
        .filter(|item| item.value != level)
        .tuple_windows()
        .filter(|(prev, curr)| prev.value > level && curr.value < level)
        .map(|(_, curr)| curr.timestamp)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::iv;

    fn buf(items: &[IndicatorValue], high: f64, low: f64) -> ThresholdBuffer {
        ThresholdBuffer {
            items: items.to_vec(),
            high_threshold: high,
            low_threshold: low,
        }
    }

    fn expected(longs: &[i64], shorts: &[i64]) -> (Vec<i64>, Vec<i64>) {
        (longs.to_vec(), shorts.to_vec())
    }

    #[test]
    fn empty_buffer_yields_no_signals() {
        let result = detect_signals(&buf(&[], 70.0, 30.0));
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn negative_levels_detect_zone_exits_like_wpr() {
        // WPR's -80 oversold and -20 overbought levels on its -100..0 scale.
        let result = detect_signals(&buf(
            &[iv(1, -90.0), iv(2, -70.0), iv(3, -10.0), iv(4, -30.0)],
            -20.0,
            -80.0,
        ));
        assert_eq!(result, expected(&[2], &[4]));
    }

    #[test]
    fn two_values_between_thresholds_no_cross() {
        let result = detect_signals(&buf(
            &[iv(1700006400, 50.0), iv(1700092800, 55.0)],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn two_values_above_high_no_down_cross() {
        let result = detect_signals(&buf(
            &[iv(1703808000, 80.0), iv(1703894400, 85.0)],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn two_values_below_low_no_up_cross() {
        let result = detect_signals(&buf(
            &[iv(1703980800, 20.0), iv(1704067200, 25.0)],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn above_low_touches_low_and_rises_back_no_signal() {
        let result = detect_signals(&buf(
            &[
                iv(1701216000, 40.0),
                iv(1701302400, 30.0),
                iv(1701388800, 35.0),
            ],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn below_high_touches_high_and_falls_back_no_signal() {
        let result = detect_signals(&buf(
            &[
                iv(1701216000, 60.0),
                iv(1701302400, 70.0),
                iv(1701388800, 65.0),
            ],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn cross_above_low_is_long() {
        let result = detect_signals(&buf(
            &[iv(1700006400, 20.0), iv(1700092800, 40.0)],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[1700092800], &[]));
    }

    #[test]
    fn cross_below_high_is_short() {
        let result = detect_signals(&buf(
            &[iv(1700179200, 80.0), iv(1700265600, 60.0)],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[], &[1700265600]));
    }

    #[test]
    fn two_long_crosses() {
        let result = detect_signals(&buf(
            &[
                iv(1700352000, 10.0),
                iv(1700438400, 50.0),
                iv(1700524800, 15.0),
                iv(1700611200, 40.0),
            ],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[1700438400, 1700611200], &[]));
    }

    #[test]
    fn two_short_crosses() {
        let result = detect_signals(&buf(
            &[
                iv(1700697600, 90.0),
                iv(1700784000, 60.0),
                iv(1700870400, 85.0),
                iv(1700956800, 55.0),
            ],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[], &[1700784000, 1700956800]));
    }

    #[test]
    fn long_and_short_in_sequence() {
        let result = detect_signals(&buf(
            &[
                iv(1701043200, 20.0),
                iv(1701129600, 50.0),
                iv(1701216000, 80.0),
                iv(1701302400, 60.0),
            ],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[1701129600], &[1701302400]));
    }

    #[test]
    fn two_longs_one_short_six_points() {
        let result = detect_signals(&buf(
            &[
                iv(1702080000, 20.0),
                iv(1702166400, 50.0),
                iv(1702252800, 80.0),
                iv(1702339200, 60.0),
                iv(1702425600, 20.0),
                iv(1702512000, 40.0),
            ],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[1702166400, 1702512000], &[1702339200]));
    }

    #[test]
    fn one_long_two_shorts_six_points() {
        let result = detect_signals(&buf(
            &[
                iv(1702598400, 80.0),
                iv(1702684800, 60.0),
                iv(1702771200, 20.0),
                iv(1702857600, 40.0),
                iv(1702944000, 80.0),
                iv(1703030400, 60.0),
            ],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[1702857600], &[1702684800, 1703030400]));
    }

    #[test]
    fn two_longs_two_shorts_eight_points() {
        let result = detect_signals(&buf(
            &[
                iv(1703116800, 10.0),
                iv(1703203200, 40.0),
                iv(1703289600, 10.0),
                iv(1703376000, 40.0),
                iv(1703462400, 80.0),
                iv(1703548800, 60.0),
                iv(1703635200, 75.0),
                iv(1703721600, 65.0),
            ],
            70.0,
            30.0,
        ));
        assert_eq!(
            result,
            expected(&[1703203200, 1703376000], &[1703548800, 1703721600])
        );
    }

    #[test]
    fn value_exactly_on_threshold_no_signal_strict_inequality() {
        let result = detect_signals(&buf(
            &[iv(100, 70.0), iv(101, 65.0), iv(102, 30.0), iv(103, 40.0)],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn single_element_yields_no_signals() {
        let result = detect_signals(&buf(&[iv(100, 50.0)], 70.0, 30.0));
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn below_low_through_low_to_above_low_is_long() {
        let result = detect_signals(&buf(
            &[iv(100, 20.0), iv(101, 30.0), iv(102, 40.0)],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[102], &[]));
    }

    #[test]
    fn below_low_touches_low_and_falls_back_no_signal() {
        let result = detect_signals(&buf(
            &[iv(100, 20.0), iv(101, 30.0), iv(102, 20.0)],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn above_high_through_high_to_below_high_is_short() {
        let result = detect_signals(&buf(
            &[iv(100, 80.0), iv(101, 70.0), iv(102, 60.0)],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[], &[102]));
    }

    #[test]
    fn above_high_touches_high_and_rises_back_no_signal() {
        let result = detect_signals(&buf(
            &[iv(100, 80.0), iv(101, 70.0), iv(102, 80.0)],
            70.0,
            30.0,
        ));
        assert_eq!(result, expected(&[], &[]));
    }
}
