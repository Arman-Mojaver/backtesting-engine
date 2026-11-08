use crate::domain::indicator::Indicator;
use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;

use super::ValidationError;

pub const fn validate_empty_entities(
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
    long_operation_points: &[OperationPoint],
    short_operation_points: &[OperationPoint],
) -> Result<(), ValidationError> {
    if long_operation_points.is_empty() {
        return Err(ValidationError::LongOperationPointsEmpty);
    }
    if short_operation_points.is_empty() {
        return Err(ValidationError::ShortOperationPointsEmpty);
    }
    if indicators.is_empty() {
        return Err(ValidationError::IndicatorsEmpty);
    }
    if resampled_points.is_empty() {
        return Err(ValidationError::ResampledPointsEmpty);
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
    fn reports_long_operation_points_first_when_all_empty() {
        assert_eq!(
            validate_empty_entities(
                &indicators_empty(),
                &resampled_points_empty(),
                &op_points_empty(),
                &op_points_empty()
            ),
            Err(ValidationError::LongOperationPointsEmpty)
        );
    }

    #[test]
    fn rejects_empty_long_operation_points() {
        assert_eq!(
            validate_empty_entities(
                &indicators_default(),
                &resampled_points_default(),
                &op_points_empty(),
                &op_points_default()
            ),
            Err(ValidationError::LongOperationPointsEmpty)
        );
    }

    #[test]
    fn rejects_empty_short_operation_points() {
        assert_eq!(
            validate_empty_entities(
                &indicators_default(),
                &resampled_points_default(),
                &op_points_default(),
                &op_points_empty()
            ),
            Err(ValidationError::ShortOperationPointsEmpty)
        );
    }

    #[test]
    fn rejects_empty_indicators() {
        assert_eq!(
            validate_empty_entities(
                &indicators_empty(),
                &resampled_points_default(),
                &op_points_default(),
                &op_points_default()
            ),
            Err(ValidationError::IndicatorsEmpty)
        );
    }

    #[test]
    fn rejects_empty_resampled_points() {
        assert_eq!(
            validate_empty_entities(
                &indicators_default(),
                &resampled_points_empty(),
                &op_points_default(),
                &op_points_default()
            ),
            Err(ValidationError::ResampledPointsEmpty)
        );
    }

    #[test]
    fn accepts_non_empty_inputs() {
        assert_eq!(
            validate_empty_entities(
                &indicators_default(),
                &resampled_points_default(),
                &op_points_default(),
                &op_points_default()
            ),
            Ok(())
        );
    }
}
