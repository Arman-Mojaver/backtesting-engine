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
}
