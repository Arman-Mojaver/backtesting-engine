use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    TimestampMismatch,
}

pub fn validate_timestamps_match(
    resampled: &[ResampledPointD1],
    long: &[OperationPoint],
    short: &[OperationPoint],
) -> Result<(), ValidationError> {
    let resampled_timestamps: HashSet<i32> = resampled.iter().map(|rp| rp.timestamp).collect();
    let long_timestamps: HashSet<i32> = long.iter().map(|op| op.timestamp).collect();
    let short_timestamps: HashSet<i32> = short.iter().map(|op| op.timestamp).collect();

    if long_timestamps != short_timestamps || !long_timestamps.is_subset(&resampled_timestamps) {
        return Err(ValidationError::TimestampMismatch);
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
            Err(ValidationError::TimestampMismatch)
        );
    }

    #[test]
    fn test_long_is_not_subset_of_resampled() {
        let resampled = resampled_points_ts_123();
        let long = op_points_ts_124();
        let short = op_points_ts_123();

        assert_eq!(
            validate_timestamps_match(&resampled[..], &long[..], &short[..]),
            Err(ValidationError::TimestampMismatch)
        );
    }

    #[test]
    fn test_short_is_not_subset_of_resampled() {
        let resampled = resampled_points_ts_123();
        let long = op_points_ts_124();
        let short = op_points_ts_123();

        assert_eq!(
            validate_timestamps_match(&resampled[..], &long[..], &short[..]),
            Err(ValidationError::TimestampMismatch)
        );
    }
}
