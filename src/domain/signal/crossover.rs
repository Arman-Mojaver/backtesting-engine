use crate::domain::signal::oscillator;
use crate::domain::signal::{CrossoverPair, IndicatorValue, OscillatorSeries};
use crate::domain::signal_group::SignalGroup;

pub fn create(buffers: &CrossoverPair, indicator_id: i32) -> SignalGroup {
    let diff_buffer: Vec<IndicatorValue> = buffers
        .upcross_long_buffer
        .iter()
        .zip(buffers.upcross_short_buffer.iter())
        .map(|(long, short)| IndicatorValue {
            timestamp: long.timestamp,
            value: long.value - short.value,
        })
        .collect();

    oscillator::create(&OscillatorSeries { items: diff_buffer }, indicator_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::signal::IndicatorValue;

    const ID: i32 = 1;

    fn iv(timestamp: i64, value: f64) -> IndicatorValue {
        IndicatorValue { timestamp, value }
    }

    fn bufs(long: &[IndicatorValue], short: &[IndicatorValue]) -> CrossoverPair {
        CrossoverPair {
            upcross_long_buffer: long.to_vec(),
            upcross_short_buffer: short.to_vec(),
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
    fn empty_buffers_yields_no_signals() {
        let result = create(&bufs(&[], &[]), ID);
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn single_pair_no_cross() {
        let result = create(&bufs(&[iv(1708300800, 1.0)], &[iv(1708300800, 2.0)]), ID);
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn long_above_short_both_positive_no_cross() {
        let result = create(
            &bufs(
                &[iv(1704153600, 2.0), iv(1704240000, 3.0)],
                &[iv(1704153600, 1.0), iv(1704240000, 1.5)],
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn long_below_short_both_negative_no_cross() {
        let result = create(
            &bufs(
                &[iv(1704326400, 0.5), iv(1704412800, 0.2)],
                &[iv(1704326400, 1.0), iv(1704412800, 2.0)],
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn identical_buffers_zero_diff_no_signal() {
        let result = create(
            &bufs(
                &[iv(1705881600, 0.5), iv(1705968000, -0.7)],
                &[iv(1705881600, 0.5), iv(1705968000, -0.7)],
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn equal_diff_no_cross() {
        let result = create(
            &bufs(
                &[iv(1712777600, -2.0), iv(1712864000, -1.5)],
                &[iv(1712777600, -1.0), iv(1712864000, -0.5)],
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn long_overtakes_short_up_cross() {
        let result = create(
            &bufs(
                &[iv(1704499200, 0.8), iv(1704585600, 1.2)],
                &[iv(1704499200, 1.0), iv(1704585600, 0.9)],
            ),
            ID,
        );
        assert_eq!(result, expected(&[1704585600], &[]));
    }

    #[test]
    fn long_falls_below_short_down_cross() {
        let result = create(
            &bufs(
                &[iv(1704672000, 1.5), iv(1704758400, 0.7)],
                &[iv(1704672000, 1.0), iv(1704758400, 1.2)],
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[1704758400]));
    }

    #[test]
    fn up_then_down_three_points() {
        let result = create(
            &bufs(
                &[
                    iv(1705017600, 0.5),
                    iv(1705104000, 1.5),
                    iv(1705190400, 0.8),
                ],
                &[
                    iv(1705017600, 1.0),
                    iv(1705104000, 1.2),
                    iv(1705190400, 1.0),
                ],
            ),
            ID,
        );
        assert_eq!(result, expected(&[1705104000], &[1705190400]));
    }

    #[test]
    fn down_then_up_three_points() {
        let result = create(
            &bufs(
                &[
                    iv(1705276800, 2.0),
                    iv(1705363200, 1.0),
                    iv(1705449600, 1.5),
                ],
                &[
                    iv(1705276800, 1.0),
                    iv(1705363200, 1.2),
                    iv(1705449600, -2.0),
                ],
            ),
            ID,
        );
        assert_eq!(result, expected(&[1705449600], &[1705363200]));
    }

    #[test]
    fn up_down_up_four_points() {
        let result = create(
            &bufs(
                &[
                    iv(1705536000, 0.0),
                    iv(1705622400, 2.0),
                    iv(1705708800, -1.0),
                    iv(1705795200, 1.0),
                ],
                &[
                    iv(1705536000, 1.0),
                    iv(1705622400, 1.5),
                    iv(1705708800, 1.2),
                    iv(1705795200, 0.5),
                ],
            ),
            ID,
        );
        assert_eq!(result, expected(&[1705622400, 1705795200], &[1705708800]));
    }

    #[test]
    fn up_cross_from_negative_two_points() {
        let result = create(
            &bufs(
                &[iv(1712000000, -2.0), iv(1712086400, 0.5)],
                &[iv(1712000000, -1.0), iv(1712086400, -0.2)],
            ),
            ID,
        );
        assert_eq!(result, expected(&[1712086400], &[]));
    }

    #[test]
    fn down_cross_to_negative_two_points() {
        let result = create(
            &bufs(
                &[iv(1712172800, 1.0), iv(1712259200, -0.7)],
                &[iv(1712172800, 0.5), iv(1712259200, 0.0)],
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[1712259200]));
    }

    #[test]
    fn down_cross_small_diff_two_points() {
        let result = create(
            &bufs(
                &[iv(1712345600, -1.5), iv(1712432000, -1.1)],
                &[iv(1712345600, -2.0), iv(1712432000, -1.0)],
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[1712432000]));
    }

    #[test]
    fn down_cross_negative_to_negative() {
        let result = create(
            &bufs(
                &[iv(1712518400, -0.5), iv(1712604800, -1.2)],
                &[iv(1712518400, -1.0), iv(1712604800, -0.8)],
            ),
            ID,
        );
        assert_eq!(result, expected(&[], &[1712604800]));
    }

    #[test]
    fn alternating_five_points_two_long_two_short() {
        let result = create(
            &bufs(
                &[
                    iv(1713036800, 0.0),
                    iv(1713123200, 2.0),
                    iv(1713209600, -1.0),
                    iv(1713296000, 1.5),
                    iv(1713382400, -0.5),
                ],
                &[
                    iv(1713036800, 1.0),
                    iv(1713123200, 1.5),
                    iv(1713209600, 0.0),
                    iv(1713296000, 1.0),
                    iv(1713382400, 0.0),
                ],
            ),
            ID,
        );
        assert_eq!(
            result,
            expected(&[1713123200, 1713296000], &[1713209600, 1713382400])
        );
    }

    // --- edge cases required by the PRD ---

    #[test]
    fn unequal_lengths_long_longer_tail_ignored() {
        // long has 3, short has 2 -> zip stops at 2 diffs.
        let result = create(
            &bufs(
                &[iv(1, -1.0), iv(2, 1.0), iv(3, -1.0)],
                &[iv(1, 0.0), iv(2, 0.0)],
            ),
            ID,
        );
        assert_eq!(result, expected(&[2], &[]));
    }

    #[test]
    fn equal_adjacent_diffs_no_cross() {
        let result = create(
            &bufs(&[iv(10, 1.0), iv(11, 2.0)], &[iv(10, 0.0), iv(11, 1.0)]),
            ID,
        );
        // diff: 1.0, 1.0 -> no cross
        assert_eq!(result, expected(&[], &[]));
    }

    #[test]
    fn stamps_indicator_id_on_result() {
        let result = create(
            &bufs(&[iv(10, 0.0), iv(11, 2.0)], &[iv(10, 1.0), iv(11, 1.0)]),
            99,
        );
        assert_eq!(result.indicator_id, 99);
        assert!(result.short_signals.is_empty());
        assert_eq!(result.long_signals, vec![11]);
    }
}
