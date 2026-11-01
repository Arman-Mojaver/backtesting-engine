use itertools::Itertools;
use thiserror::Error;

use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("instruments must match across resampled points, long operation points, and short operation points")]
    InstrumentMismatch,
}

pub fn validate_same_instrument(
    resampled_points: &[ResampledPointD1],
    long_operation_points: &[OperationPoint],
    short_operation_points: &[OperationPoint],
) -> Result<(), ValidationError> {
    let all_match = resampled_points
        .iter()
        .map(|resampled_point| &resampled_point.instrument)
        .chain(long_operation_points.iter().map(|op| &op.instrument))
        .chain(short_operation_points.iter().map(|op| &op.instrument))
        .all_equal();

    if !all_match {
        return Err(ValidationError::InstrumentMismatch);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::*;

    #[test]
    fn test_all_equal() {
        let resampled = resampled_points_default();
        let long = op_points_default();
        let short = op_points_default();

        assert_eq!(validate_same_instrument(&resampled, &long, &short), Ok(()));
    }

    #[test]
    fn test_resampled_different() {
        let resampled = resampled_points_usdcad();
        let long = op_points_default();
        let short = op_points_default();

        assert_eq!(
            validate_same_instrument(&resampled, &long, &short),
            Err(ValidationError::InstrumentMismatch)
        );
    }

    #[test]
    fn test_long_different() {
        let resampled = resampled_points_default();
        let long = op_points_default_usdcad();
        let short = op_points_default();

        assert_eq!(
            validate_same_instrument(&resampled, &long, &short),
            Err(ValidationError::InstrumentMismatch)
        );
    }

    #[test]
    fn test_short_different() {
        let resampled = resampled_points_default();
        let long = op_points_default();
        let short = op_points_default_usdcad();

        assert_eq!(
            validate_same_instrument(&resampled, &long, &short),
            Err(ValidationError::InstrumentMismatch)
        );
    }

    #[test]
    fn test_resampled_partial_mismatch() {
        let resampled = resampled_points_partial_mismatch();
        let long = op_points_default();
        let short = op_points_default();

        assert_eq!(
            validate_same_instrument(&resampled, &long, &short),
            Err(ValidationError::InstrumentMismatch)
        );
    }

    #[test]
    fn test_long_partial_mismatch() {
        let resampled = resampled_points_default();
        let long = op_points_partial_mismatch();
        let short = op_points_default();

        assert_eq!(
            validate_same_instrument(&resampled, &long, &short),
            Err(ValidationError::InstrumentMismatch)
        );
    }

    #[test]
    fn test_short_partial_mismatch() {
        let resampled = resampled_points_default();
        let long = op_points_default();
        let short = op_points_partial_mismatch();

        assert_eq!(
            validate_same_instrument(&resampled, &long, &short),
            Err(ValidationError::InstrumentMismatch)
        );
    }

    #[test]
    fn display_message_names_the_violated_constraint() {
        assert_eq!(
            ValidationError::InstrumentMismatch.to_string(),
            "instruments must match across resampled points, long operation points, and short operation points"
        );
    }
}
