use std::error::Error;
use std::fmt;

use crate::domain::indicator::Indicator;
use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;

use super::empty_validation;
use super::same_instrument_validation;
use super::same_mms_validation;
use super::same_timestamp_validation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CombinedValidationError {
    Empty(empty_validation::EmptyRepoValidationError),
    InstrumentMismatch,
    MoneyManagementStrategyMismatch,
    TimestampMismatch,
}

impl fmt::Display for CombinedValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty(inner) => write!(f, "{inner}"),
            Self::InstrumentMismatch => write!(
                f,
                "instruments must match across resampled points, long operation points, and short operation points"
            ),
            Self::MoneyManagementStrategyMismatch => write!(
                f,
                "all operation points must reference the expected money management strategy id"
            ),
            Self::TimestampMismatch => write!(
                f,
                "operation point timestamps must match each other and be contained in the resampled point timestamps"
            ),
        }
    }
}

impl Error for CombinedValidationError {}

pub fn validate_all(
    long_ops: &[OperationPoint],
    short_ops: &[OperationPoint],
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
    expected_mms_id: i32,
) -> Result<(), CombinedValidationError> {
    empty_validation::validate_empty_entities(long_ops, short_ops, indicators, resampled_points)
        .map_err(CombinedValidationError::Empty)?;

    same_instrument_validation::validate_same_instrument(resampled_points, long_ops, short_ops)
        .map_err(|_| CombinedValidationError::InstrumentMismatch)?;

    same_mms_validation::validate_money_management_strategy_id_matches(
        long_ops,
        short_ops,
        expected_mms_id,
    )
    .map_err(|_| CombinedValidationError::MoneyManagementStrategyMismatch)?;

    same_timestamp_validation::validate_timestamps_match(resampled_points, long_ops, short_ops)
        .map_err(|_| CombinedValidationError::TimestampMismatch)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::*;

    #[test]
    fn test_all_valid() {
        let long = op_points_default();
        let short = op_points_default();
        let indicators = indicators_default();
        let resampled = resampled_points_default();

        assert_eq!(
            validate_all(&long, &short, &indicators, &resampled, 1),
            Ok(())
        );
    }

    #[test]
    fn test_empty_long_fails() {
        let long = op_points_empty();
        let short = op_points_default();
        let indicators = indicators_default();
        let resampled = resampled_points_default();

        assert_eq!(
            validate_all(&long, &short, &indicators, &resampled, 1),
            Err(CombinedValidationError::Empty(
                empty_validation::EmptyRepoValidationError::LongOperationPointsEmpty
            ))
        );
    }

    #[test]
    fn test_empty_short_fails() {
        let long = op_points_default();
        let short = op_points_empty();
        let indicators = indicators_default();
        let resampled = resampled_points_default();

        assert_eq!(
            validate_all(&long, &short, &indicators, &resampled, 1),
            Err(CombinedValidationError::Empty(
                empty_validation::EmptyRepoValidationError::ShortOperationPointsEmpty
            ))
        );
    }

    #[test]
    fn test_instrument_mismatch() {
        let long = op_points_default();
        let short = op_points_default();
        let indicators = indicators_default();
        let resampled = resampled_points_usdcad();

        assert_eq!(
            validate_all(&long, &short, &indicators, &resampled, 1),
            Err(CombinedValidationError::InstrumentMismatch)
        );
    }

    #[test]
    fn test_mms_id_mismatch() {
        let long = op_points_default();
        let short = op_points_default();
        let indicators = indicators_default();
        let resampled = resampled_points_default();

        assert_eq!(
            validate_all(&long, &short, &indicators, &resampled, 2),
            Err(CombinedValidationError::MoneyManagementStrategyMismatch)
        );
    }

    #[test]
    fn test_timestamp_mismatch() {
        let long = op_points_ts_123();
        let short = op_points_ts_124();
        let indicators = indicators_default();
        let resampled = resampled_points_ts_123();

        assert_eq!(
            validate_all(&long, &short, &indicators, &resampled, 1),
            Err(CombinedValidationError::TimestampMismatch)
        );
    }

    #[test]
    fn display_messages_name_the_violated_constraint() {
        let cases = [
            (
                CombinedValidationError::Empty(
                    empty_validation::EmptyRepoValidationError::LongOperationPointsEmpty
                ),
                "long operation points list is empty",
            ),
            (
                CombinedValidationError::InstrumentMismatch,
                "instruments must match across resampled points, long operation points, and short operation points",
            ),
            (
                CombinedValidationError::MoneyManagementStrategyMismatch,
                "all operation points must reference the expected money management strategy id",
            ),
            (
                CombinedValidationError::TimestampMismatch,
                "operation point timestamps must match each other and be contained in the resampled point timestamps",
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
    fn combined_validation_error_is_a_std_error_object() {
        let boxed: Box<dyn std::error::Error> =
            Box::new(CombinedValidationError::TimestampMismatch);
        assert_eq!(
            boxed.to_string(),
            "operation point timestamps must match each other and be contained in the resampled point timestamps"
        );
        assert!(boxed.source().is_none());
    }
}
