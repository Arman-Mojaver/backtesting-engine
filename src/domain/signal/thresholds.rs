use crate::domain::signal::ThresholdSeries;
use crate::domain::signal_group::SignalGroup;

pub fn create(buffer: &ThresholdSeries, indicator_id: i32) -> SignalGroup {
    let values = &buffer.items;
    let n = values.len();
    let high_threshold = buffer.high_threshold;
    let low_threshold = buffer.low_threshold;

    let mut long_signals: Vec<i64> = Vec::with_capacity(n / 2);
    let mut short_signals: Vec<i64> = Vec::with_capacity(n / 2);

    if n == 0 {
        return SignalGroup {
            indicator_id,
            long_signals,
            short_signals,
        };
    }

    let mut prev = &values[0];
    for curr in &values[1..] {
        if prev.value < low_threshold && curr.value > low_threshold {
            long_signals.push(curr.timestamp);
        } else if prev.value > high_threshold && curr.value < high_threshold {
            short_signals.push(curr.timestamp);
        }
        prev = curr;
    }

    SignalGroup {
        indicator_id,
        long_signals,
        short_signals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::signal::IndicatorValue;

    const ID: i32 = 1;

    fn iv(timestamp: i64, value: f64) -> IndicatorValue {
        IndicatorValue { timestamp, value }
    }

    fn buf(items: &[IndicatorValue], high: f64, low: f64) -> ThresholdSeries {
        ThresholdSeries {
            items: items.to_vec(),
            high_threshold: high,
            low_threshold: low,
        }
    }

    fn expected(longs: &[i64], shorts: &[i64]) -> SignalGroup {
        SignalGroup {
            indicator_id: ID,
            long_signals: longs.to_vec(),
            short_signals: shorts.to_vec(),
        }
    }

    #[test]
    fn empty_buffer_yields_no_signals() {
        let result = create(&buf(&[], 1.0, -1.0), ID);
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn two_values_between_thresholds_no_cross() {
        let result = create(
            &buf(&[iv(1700006400, 0.5), iv(1700092800, 0.7)], 1.0, -1.0),
            ID,
        );
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn two_values_above_high_no_down_cross() {
        let result = create(
            &buf(&[iv(1703808000, 3.0), iv(1703894400, 3.1)], 2.0, -1.0),
            ID,
        );
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn two_values_below_low_no_up_cross() {
        let result = create(
            &buf(&[iv(1703980800, -2.5), iv(1704067200, -2.0)], 1.0, -1.0),
            ID,
        );
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn value_on_low_threshold_strict_no_long() {
        // low=-1.5: prev=-2.0 < -1.5, curr=-1.5 > -1.5 is false (strict) -> no long.
        let result = create(
            &buf(
                &[
                    iv(1701043200, -2.0),
                    iv(1701129600, -1.5),
                    iv(1701388800, -1.9),
                ],
                1.0,
                -1.5,
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn value_on_low_threshold_strict_no_long_v2() {
        // low=-2.0: prev=-1.0 < -2.0 false; prev=-2.0 < -2.0 false -> no long.
        let result = create(
            &buf(
                &[
                    iv(1701216000, -1.0),
                    iv(1701302400, -2.0),
                    iv(1701388800, -1.2),
                ],
                1.0,
                -2.0,
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn value_on_high_threshold_strict_no_short() {
        // high=2.0: prev=2.5 > 2.0, curr=2.0 < 2.0 false -> no short.
        let result = create(
            &buf(
                &[
                    iv(1701216000, 2.5),
                    iv(1701302400, 2.0),
                    iv(1701388800, 2.2),
                ],
                2.0,
                -1.0,
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn value_on_high_threshold_strict_no_short_v2() {
        // high=1.5: prev=1.0 > 1.5 false; prev=1.5 > 1.5 false -> no short.
        let result = create(
            &buf(
                &[
                    iv(1701216000, 1.0),
                    iv(1701302400, 1.5),
                    iv(1701388800, 1.2),
                ],
                1.5,
                -1.0,
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn cross_above_low_is_long() {
        let result = create(
            &buf(&[iv(1700006400, -2.0), iv(1700092800, 0.2)], 1.5, -1.0),
            ID,
        );
        assert_eq!(result, expected(&[1700092800], &[]));
    }

    #[test]
    fn cross_below_high_is_short() {
        let result = create(
            &buf(&[iv(1700179200, 2.5), iv(1700265600, 1.0)], 2.0, -1.5),
            ID,
        );
        assert_eq!(result, expected(&[], &[1700265600]));
    }

    #[test]
    fn two_long_crosses_through_zero() {
        let result = create(
            &buf(
                &[
                    iv(1700352000, -3.0),
                    iv(1700438400, 0.0),
                    iv(1700524800, -2.5),
                    iv(1700611200, -0.2),
                ],
                2.0,
                -1.0,
            ),
            ID,
        );
        assert_eq!(result, expected(&[1700438400, 1700611200], &[]));
    }

    #[test]
    fn two_short_crosses() {
        let result = create(
            &buf(
                &[
                    iv(1700697600, 3.5),
                    iv(1700784000, 1.8),
                    iv(1700870400, 3.0),
                    iv(1700956800, 1.4),
                ],
                2.5,
                -2.0,
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[1700784000, 1700956800]));
    }

    #[test]
    fn long_and_short_in_sequence() {
        let result = create(
            &buf(
                &[
                    iv(1701043200, -2.5),
                    iv(1701129600, 0.1),
                    iv(1701216000, 2.8),
                    iv(1701302400, 1.5),
                ],
                2.0,
                -1.0,
            ),
            ID,
        );
        assert_eq!(result, expected(&[1701129600], &[1701302400]));
    }

    #[test]
    fn two_longs_one_short_six_points() {
        let result = create(
            &buf(
                &[
                    iv(1702080000, -2.5),
                    iv(1702166400, 0.5),
                    iv(1702252800, 3.2),
                    iv(1702339200, 1.4),
                    iv(1702425600, -2.2),
                    iv(1702512000, 0.3),
                ],
                2.5,
                -1.0,
            ),
            ID,
        );
        assert_eq!(result, expected(&[1702166400, 1702512000], &[1702339200]));
    }

    #[test]
    fn one_long_two_shorts_six_points() {
        let result = create(
            &buf(
                &[
                    iv(1702598400, 2.8),
                    iv(1702684800, 1.7),
                    iv(1702771200, -2.0),
                    iv(1702857600, 0.1),
                    iv(1702944000, 3.0),
                    iv(1703030400, 1.9),
                ],
                2.5,
                -1.5,
            ),
            ID,
        );
        assert_eq!(result, expected(&[1702857600], &[1702684800, 1703030400]));
    }

    #[test]
    fn two_longs_two_shorts_eight_points() {
        let result = create(
            &buf(
                &[
                    iv(1703116800, -3.0),
                    iv(1703203200, 0.1),
                    iv(1703289600, -3.1),
                    iv(1703376000, 0.2),
                    iv(1703462400, 3.0),
                    iv(1703548800, 1.7),
                    iv(1703635200, 2.9),
                    iv(1703721600, 1.5),
                ],
                2.5,
                -1.0,
            ),
            ID,
        );
        assert_eq!(
            result,
            expected(&[1703203200, 1703376000], &[1703548800, 1703721600])
        );
    }

    // --- edge cases required by the PRD ---

    #[test]
    fn value_exactly_on_threshold_no_signal_strict_inequality() {
        // high=2.0, low=-1.0. prev=2.0 (not > high), curr=1.9 -> no short.
        // prev=-1.0 (not < low), curr=0.0 -> no long.
        let result = create(
            &buf(
                &[iv(100, 2.0), iv(101, 1.9), iv(102, -1.0), iv(103, 0.0)],
                2.0,
                -1.0,
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn single_element_yields_no_signals() {
        let result = create(&buf(&[iv(100, 5.0)], 2.0, -2.0), ID);
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn stamps_indicator_id_on_result() {
        let result = create(&buf(&[iv(10, -2.0), iv(11, 0.5)], 1.0, -1.0), 7);
        assert_eq!(result.indicator_id, 7);
        assert_eq!(result.long_signals, vec![11]);
    }
}
