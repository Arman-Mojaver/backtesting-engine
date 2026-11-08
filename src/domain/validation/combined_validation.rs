use crate::domain::indicator::Indicator;
use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;

use super::empty_validation;
use super::same_instrument_validation;
use super::same_mms_validation;
use super::same_timestamp_validation;
use super::ValidationError;

pub fn validate_all(
    long_ops: &[OperationPoint],
    short_ops: &[OperationPoint],
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
    expected_mms_id: i32,
) -> Result<(), ValidationError> {
    empty_validation::validate_empty_entities(long_ops, short_ops, indicators, resampled_points)?;

    same_instrument_validation::validate_same_instrument(resampled_points, long_ops, short_ops)?;

    same_mms_validation::validate_money_management_strategy_id_matches(
        long_ops,
        short_ops,
        expected_mms_id,
    )?;

    same_timestamp_validation::validate_timestamps_match(resampled_points, long_ops, short_ops)?;

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
            Err(ValidationError::LongOperationPointsEmpty)
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
            Err(ValidationError::ShortOperationPointsEmpty)
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
            Err(ValidationError::InstrumentMismatch {
                expected: "USDCAD".into(),
                found: "EURUSD".into(),
            })
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
            Err(ValidationError::MoneyManagementStrategyMismatch)
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
            Err(ValidationError::TimestampMismatch)
        );
    }
}
