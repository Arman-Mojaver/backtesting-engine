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
    fn test_all_empty() {
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
    fn test_long_empty_only() {
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
    fn test_short_empty_only() {
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
    fn test_indicators_empty_only() {
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
    fn test_resampled_empty_only() {
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
    fn test_all_have_data() {
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
