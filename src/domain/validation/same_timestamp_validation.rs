use rustc_hash::FxHashSet;

use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;

use super::ValidationError;

pub fn validate_timestamps_match(
    resampled_points: &[ResampledPointD1],
    long_operation_points: &[OperationPoint],
    short_operation_points: &[OperationPoint],
) -> Result<(), ValidationError> {
    let resampled_timestamps: FxHashSet<i64> =
        resampled_points.iter().map(|rp| rp.timestamp).collect();
    let long_timestamps: FxHashSet<i64> = long_operation_points
        .iter()
        .map(|op| op.timestamp)
        .collect();
    let short_timestamps: FxHashSet<i64> = short_operation_points
        .iter()
        .map(|op| op.timestamp)
        .collect();

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
    fn accepts_identical_timestamps_everywhere() {
        let resampled = resampled_points_ts_123();
        let long = op_points_ts_123();
        let short = op_points_ts_123();

        assert_eq!(validate_timestamps_match(&resampled, &long, &short), Ok(()));
    }

    #[test]
    fn accepts_operation_timestamps_within_resampled() {
        let resampled = resampled_points_ts_123();
        let long = op_points_ts_123();
        let short = op_points_ts_123();

        assert_eq!(
            validate_timestamps_match(&resampled, &long[..2], &short[..2]),
            Ok(())
        );
    }

    #[test]
    fn rejects_long_and_short_with_different_timestamps() {
        let resampled = resampled_points_ts_123();
        let long = op_points_ts_123();
        let short = op_points_ts_124();

        assert_eq!(
            validate_timestamps_match(&resampled, &long, &short),
            Err(ValidationError::LongShortTimestampsDiffer { timestamp: 3000 })
        );
    }

    #[test]
    fn rejects_timestamp_missing_from_resampled() {
        let resampled = resampled_points_ts_123();
        let long = op_points_ts_124();
        let short = op_points_ts_124();

        assert_eq!(
            validate_timestamps_match(&resampled, &long, &short),
            Err(ValidationError::TimestampMissingFromResampled { timestamp: 4000 })
        );
    }

    #[test]
    fn rejects_short_timestamp_missing_from_long() {
        let resampled = resampled_points_ts_123();
        let long = op_points_ts_123();
        let short = op_points_ts_124();

        assert_eq!(
            validate_timestamps_match(&resampled, &long[..2], &short),
            Err(ValidationError::LongShortTimestampsDiffer { timestamp: 4000 })
        );
    }
}
