use crate::domain::signal::OneBuffer;
use crate::domain::signal_group::SignalGroup;

/// Zero-crossing signal rule.
///
/// Long signal when the value crosses upward through zero
/// (`prev < 0 && curr > 0`); short signal when it crosses downward
/// (`prev > 0 && curr < 0`). Strict inequalities mean a value that is
/// exactly zero produces no signal (documented known-edge).
pub fn generate(buffer: &OneBuffer, indicator_id: i32) -> SignalGroup {
    let values = &buffer.items;
    let n = values.len();

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
        if prev.value < 0.0 && curr.value > 0.0 {
            long_signals.push(curr.timestamp);
        } else if prev.value > 0.0 && curr.value < 0.0 {
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

    fn buf(items: &[IndicatorValue]) -> OneBuffer {
        OneBuffer {
            items: items.to_vec(),
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
        let result = generate(&buf(&[]), ID);
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn single_positive_yields_no_signals() {
        let result = generate(&buf(&[iv(1704067200, 1.2)]), ID);
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn single_negative_yields_no_signals() {
        let result = generate(&buf(&[iv(1704153600, -1.5)]), ID);
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn two_positives_no_cross() {
        let result = generate(&buf(&[iv(1704758400, 1.5), iv(1704844800, 2.3)]), ID);
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn two_negatives_no_cross() {
        let result = generate(&buf(&[iv(1704931200, -0.6), iv(1705017600, -1.2)]), ID);
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn negative_to_positive_is_long() {
        let result = generate(&buf(&[iv(1704240000, -2.0), iv(1704326400, 2.0)]), ID);
        assert_eq!(result, expected(&[1704326400], &[]));
    }

    #[test]
    fn positive_to_negative_is_short() {
        let result = generate(&buf(&[iv(1704412800, 0.7), iv(1704672000, -0.9)]), ID);
        assert_eq!(result, expected(&[], &[1704672000]));
    }

    #[test]
    fn up_cross_then_stay_positive_yields_long_only() {
        let result = generate(
            &buf(&[
                iv(1705276800, -1.1),
                iv(1705363200, 1.0),
                iv(1705449600, 0.5),
            ]),
            ID,
        );
        assert_eq!(result, expected(&[1705363200], &[]));
    }

    #[test]
    fn alternating_crosses_long_then_short_then_long() {
        let result = generate(
            &buf(&[
                iv(1705536000, -0.4),
                iv(1705622400, 0.3),
                iv(1705881600, -0.8),
                iv(1705968000, 0.9),
            ]),
            ID,
        );
        assert_eq!(result, expected(&[1705622400, 1705968000], &[1705881600]));
    }

    #[test]
    fn positive_then_down_cross_then_up_cross() {
        let result = generate(
            &buf(&[
                iv(1706054400, 0.6),
                iv(1706140800, 1.2),
                iv(1706227200, -1.4),
                iv(1706486400, 0.5),
            ]),
            ID,
        );
        assert_eq!(result, expected(&[1706486400], &[1706227200]));
    }

    #[test]
    fn negatives_then_up_cross_only() {
        let result = generate(
            &buf(&[
                iv(1706572800, -2.2),
                iv(1706659200, -1.1),
                iv(1706745600, 2.5),
                iv(1706832000, 0.4),
            ]),
            ID,
        );
        assert_eq!(result, expected(&[1706745600], &[]));
    }

    #[test]
    fn alternating_four_crosses() {
        let result = generate(
            &buf(&[
                iv(1707091200, 1.1),
                iv(1707177600, -1.3),
                iv(1707264000, 1.6),
                iv(1707350400, -1.9),
            ]),
            ID,
        );
        assert_eq!(result, expected(&[1707264000], &[1707177600, 1707350400]));
    }

    #[test]
    fn positive_decline_single_short() {
        let result = generate(
            &buf(&[
                iv(1707436800, 2.0),
                iv(1707696000, 1.8),
                iv(1707782400, -0.5),
                iv(1707868800, -0.2),
            ]),
            ID,
        );
        assert_eq!(result, expected(&[], &[1707782400]));
    }

    #[test]
    fn two_up_crosses_one_down_cross() {
        let result = generate(
            &buf(&[
                iv(1707955200, -1.7),
                iv(1708041600, 1.9),
                iv(1708300800, -0.6),
                iv(1708387200, 0.7),
            ]),
            ID,
        );
        assert_eq!(result, expected(&[1708041600, 1708387200], &[1708300800]));
    }

    #[test]
    fn positive_negative_negative_positive() {
        let result = generate(
            &buf(&[
                iv(1708473600, 1.2),
                iv(1708560000, -1.2),
                iv(1708646400, -0.9),
                iv(1708905600, 0.4),
            ]),
            ID,
        );
        assert_eq!(result, expected(&[1708905600], &[1708560000]));
    }

    #[test]
    fn up_then_down_at_end() {
        let result = generate(
            &buf(&[
                iv(1708992000, -0.3),
                iv(1709078400, 1.5),
                iv(1709164800, 0.2),
                iv(1709251200, -1.0),
            ]),
            ID,
        );
        assert_eq!(result, expected(&[1709078400], &[1709251200]));
    }

    #[test]
    fn five_points_two_up_one_down() {
        let result = generate(
            &buf(&[
                iv(1709510400, -0.8),
                iv(1709596800, 0.9),
                iv(1709683200, 1.7),
                iv(1709769600, -1.5),
                iv(1709856000, 1.4),
            ]),
            ID,
        );
        assert_eq!(result, expected(&[1709596800, 1709856000], &[1709769600]));
    }

    #[test]
    fn five_points_alternating_long_short() {
        let result = generate(
            &buf(&[
                iv(1710115200, 0.3),
                iv(1710201600, -0.4),
                iv(1710288000, 0.5),
                iv(1710374400, -0.6),
                iv(1710460800, 0.7),
            ]),
            ID,
        );
        assert_eq!(
            result,
            expected(&[1710288000, 1710460800], &[1710201600, 1710374400])
        );
    }

    #[test]
    fn five_points_one_up_one_down() {
        let result = generate(
            &buf(&[
                iv(1710720000, -1.6),
                iv(1710806400, -0.3),
                iv(1710892800, 1.1),
                iv(1710979200, 0.6),
                iv(1711065600, -0.1),
            ]),
            ID,
        );
        assert_eq!(result, expected(&[1710892800], &[1711065600]));
    }

    #[test]
    fn five_points_two_up_two_down() {
        let result = generate(
            &buf(&[
                iv(1711324800, 2.1),
                iv(1711411200, -2.1),
                iv(1711497600, 2.2),
                iv(1711584000, -2.2),
                iv(1711670400, 2.3),
            ]),
            ID,
        );
        assert_eq!(
            result,
            expected(&[1711497600, 1711670400], &[1711411200, 1711584000])
        );
    }

    // --- edge cases not in the Python suite but required by the PRD ---

    #[test]
    fn value_exactly_zero_produces_no_signal_known_edge() {
        // prev > 0, curr == 0: strict `curr < 0` is false -> no short signal.
        // prev == 0, curr > 0: strict `prev < 0` is false -> no long signal.
        let result = generate(&buf(&[iv(100, 1.0), iv(101, 0.0), iv(102, 1.0)]), ID);
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn equal_adjacent_values_no_signal() {
        let result = generate(&buf(&[iv(100, 0.5), iv(101, 0.5)]), ID);
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn stamps_indicator_id_on_result() {
        let result = generate(&buf(&[iv(10, -1.0), iv(11, 1.0)]), 42);
        assert_eq!(result.indicator_id, 42);
        assert_eq!(result.long_signals, vec![11]);
    }
}
