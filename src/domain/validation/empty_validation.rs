use std::error::Error;
use std::fmt;

use crate::domain::indicator::Indicator;
use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmptyRepoValidationError {
    LongOperationPointsEmpty,
    ShortOperationPointsEmpty,
    IndicatorsEmpty,
    ResampledPointsEmpty,
}

impl fmt::Display for EmptyRepoValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LongOperationPointsEmpty => write!(f, "long operation points list is empty"),
            Self::ShortOperationPointsEmpty => write!(f, "short operation points list is empty"),
            Self::IndicatorsEmpty => write!(f, "indicators list is empty"),
            Self::ResampledPointsEmpty => write!(f, "resampled points list is empty"),
        }
    }
}

impl Error for EmptyRepoValidationError {}

pub const fn validate_empty_entities(
    long_ops: &[OperationPoint],
    short_ops: &[OperationPoint],
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
) -> Result<(), EmptyRepoValidationError> {
    if long_ops.is_empty() {
        return Err(EmptyRepoValidationError::LongOperationPointsEmpty);
    }
    if short_ops.is_empty() {
        return Err(EmptyRepoValidationError::ShortOperationPointsEmpty);
    }
    if indicators.is_empty() {
        return Err(EmptyRepoValidationError::IndicatorsEmpty);
    }
    if resampled_points.is_empty() {
        return Err(EmptyRepoValidationError::ResampledPointsEmpty);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::{
        indicators_default, indicators_empty, op_points_default, op_points_empty,
        resampled_points_default, resampled_points_empty,
    };

    #[test]
    fn test_all_empty() {
        assert_eq!(
            validate_empty_entities(
                &op_points_empty(),
                &op_points_empty(),
                &indicators_empty(),
                &resampled_points_empty()
            ),
            Err(EmptyRepoValidationError::LongOperationPointsEmpty)
        );
    }

    #[test]
    fn test_long_empty_only() {
        assert_eq!(
            validate_empty_entities(
                &op_points_empty(),
                &op_points_default(),
                &indicators_default(),
                &resampled_points_default()
            ),
            Err(EmptyRepoValidationError::LongOperationPointsEmpty)
        );
    }

    #[test]
    fn test_short_empty_only() {
        assert_eq!(
            validate_empty_entities(
                &op_points_default(),
                &op_points_empty(),
                &indicators_default(),
                &resampled_points_default()
            ),
            Err(EmptyRepoValidationError::ShortOperationPointsEmpty)
        );
    }

    #[test]
    fn test_indicators_empty_only() {
        assert_eq!(
            validate_empty_entities(
                &op_points_default(),
                &op_points_default(),
                &indicators_empty(),
                &resampled_points_default()
            ),
            Err(EmptyRepoValidationError::IndicatorsEmpty)
        );
    }

    #[test]
    fn test_resampled_empty_only() {
        assert_eq!(
            validate_empty_entities(
                &op_points_default(),
                &op_points_default(),
                &indicators_default(),
                &resampled_points_empty()
            ),
            Err(EmptyRepoValidationError::ResampledPointsEmpty)
        );
    }

    #[test]
    fn test_all_have_data() {
        assert_eq!(
            validate_empty_entities(
                &op_points_default(),
                &op_points_default(),
                &indicators_default(),
                &resampled_points_default()
            ),
            Ok(())
        );
    }

    #[test]
    fn display_messages_name_the_empty_input() {
        let cases = [
            (
                EmptyRepoValidationError::LongOperationPointsEmpty,
                "long operation points list is empty",
            ),
            (
                EmptyRepoValidationError::ShortOperationPointsEmpty,
                "short operation points list is empty",
            ),
            (
                EmptyRepoValidationError::IndicatorsEmpty,
                "indicators list is empty",
            ),
            (
                EmptyRepoValidationError::ResampledPointsEmpty,
                "resampled points list is empty",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(
                error.to_string(),
                expected,
                "Display mismatch for {error:?}"
            );
        }
    }

    #[test]
    fn empty_repo_validation_error_is_a_std_error_object() {
        let boxed: Box<dyn std::error::Error> = Box::new(EmptyRepoValidationError::IndicatorsEmpty);
        assert_eq!(boxed.to_string(), "indicators list is empty");
        assert!(boxed.source().is_none());
    }
}
