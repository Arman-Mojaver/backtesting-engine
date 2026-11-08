use rustc_hash::FxHashSet;

use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;

use super::ValidationError;

pub fn validate_timestamps_match(
    resampled: &[ResampledPointD1],
    long: &[OperationPoint],
    short: &[OperationPoint],
) -> Result<(), ValidationError> {
    let resampled_timestamps: FxHashSet<i64> = resampled.iter().map(|rp| rp.timestamp).collect();
    let long_timestamps: FxHashSet<i64> = long.iter().map(|op| op.timestamp).collect();
    let short_timestamps: FxHashSet<i64> = short.iter().map(|op| op.timestamp).collect();

    if let Some(&timestamp) = long_timestamps
        .symmetric_difference(&short_timestamps)
        .min()
    {
        return Err(ValidationError::LongShortTimestampsDiffer { timestamp });
    }
    if let Some(&timestamp) = long_timestamps.difference(&resampled_timestamps).min() {
        return Err(ValidationError::TimestampMissingFromResampled { timestamp });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::*;

    #[test]
    fn test_all_equal() {
        let resampled = resampled_points_ts_123();
        let long = op_points_ts_123();
        let short = op_points_ts_123();

        assert_eq!(
            validate_timestamps_match(&resampled[..], &long[..], &short[..]),
            Ok(())
        );
    }

    #[test]
    fn test_all_long_and_short_are_subset_of_resampled() {
        let resampled = resampled_points_ts_123();
        let long = op_points_ts_123();
        let short = op_points_ts_123();

        assert_eq!(
            validate_timestamps_match(&resampled[..], &long[..2], &short[..2]),
            Ok(())
        );
    }

    #[test]
    fn test_long_differs_short() {
        let resampled = resampled_points_ts_123();
        let long = op_points_ts_123();
        let short = op_points_ts_124();

        assert_eq!(
            validate_timestamps_match(&resampled[..], &long[..], &short[..]),
            Err(ValidationError::LongShortTimestampsDiffer { timestamp: 3 })
        );
    }

    #[test]
    fn test_long_is_not_subset_of_resampled() {
        let resampled = resampled_points_ts_123();
        let long = op_points_ts_124();
        let short = op_points_ts_124();

        assert_eq!(
            validate_timestamps_match(&resampled[..], &long[..], &short[..]),
            Err(ValidationError::TimestampMissingFromResampled { timestamp: 4 })
        );
    }

    #[test]
    fn test_short_has_timestamp_missing_from_long() {
        let resampled = resampled_points_ts_123();
        let long = op_points_ts_123();
        let short = op_points_ts_124();

        assert_eq!(
            validate_timestamps_match(&resampled[..], &long[..2], &short[..]),
            Err(ValidationError::LongShortTimestampsDiffer { timestamp: 4 })
        );
    }
}
