use crate::domain::indicator::Indicator;
use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;

use super::empty_validation;
use super::same_instrument_validation;
use super::same_mms_validation;
use super::same_timestamp_validation;
use super::ValidationError;

pub fn validate_all(
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
    long_operation_points: &[OperationPoint],
    short_operation_points: &[OperationPoint],
    expected_mms_id: i32,
) -> Result<(), ValidationError> {
    empty_validation::validate_empty_entities(
        indicators,
        resampled_points,
        long_operation_points,
        short_operation_points,
    )?;

    same_instrument_validation::validate_same_instrument(
        resampled_points,
        long_operation_points,
        short_operation_points,
    )?;

    same_mms_validation::validate_money_management_strategy_id_matches(
        long_operation_points,
        short_operation_points,
        expected_mms_id,
    )?;

    same_timestamp_validation::validate_timestamps_match(
        resampled_points,
        long_operation_points,
        short_operation_points,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::*;

    #[test]
    fn accepts_consistent_inputs() {
        let long = op_points_default();
        let short = op_points_default();
        let indicators = indicators_default();
        let resampled = resampled_points_default();

        assert_eq!(
            validate_all(&indicators, &resampled, &long, &short, 1),
            Ok(())
        );
    }

    #[test]
    fn rejects_empty_long_operation_points() {
        let long = op_points_empty();
        let short = op_points_default();
        let indicators = indicators_default();
        let resampled = resampled_points_default();

        assert_eq!(
            validate_all(&indicators, &resampled, &long, &short, 1),
            Err(ValidationError::LongOperationPointsEmpty)
        );
    }

    #[test]
    fn rejects_empty_short_operation_points() {
        let long = op_points_default();
        let short = op_points_empty();
        let indicators = indicators_default();
        let resampled = resampled_points_default();

        assert_eq!(
            validate_all(&indicators, &resampled, &long, &short, 1),
            Err(ValidationError::ShortOperationPointsEmpty)
        );
    }

    #[test]
    fn rejects_instrument_mismatch() {
        let long = op_points_default();
        let short = op_points_default();
        let indicators = indicators_default();
        let resampled = resampled_points_usdcad();

        assert_eq!(
            validate_all(&indicators, &resampled, &long, &short, 1),
            Err(ValidationError::InstrumentMismatch {
                expected: "USDCAD".into(),
                found: "EURUSD".into(),
            })
        );
    }

    #[test]
    fn rejects_unexpected_mms_id() {
        let long = op_points_default();
        let short = op_points_default();
        let indicators = indicators_default();
        let resampled = resampled_points_default();

        assert_eq!(
            validate_all(&indicators, &resampled, &long, &short, 2),
            Err(ValidationError::MoneyManagementStrategyMismatch {
                expected: 2,
                found: 1,
            })
        );
    }

    #[test]
    fn rejects_long_short_timestamp_mismatch() {
        let long = op_points_ts_123();
        let short = op_points_ts_124();
        let indicators = indicators_default();
        let resampled = resampled_points_ts_123();

        assert_eq!(
            validate_all(&indicators, &resampled, &long, &short, 1),
            Err(ValidationError::LongShortTimestampsDiffer { timestamp: 3 })
        );
    }
}
