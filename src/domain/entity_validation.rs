use crate::domain::indicator::Indicator;
use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;

#[derive(Debug, Clone, PartialEq)]
pub enum EmptyValidationError {
    LongOperationPointsEmpty,
    ShortOperationPointsEmpty,
    IndicatorsEmpty,
    ResampledPointsEmpty,
}

fn ensure_not_empty<T>(
    data: &[T],
    error: EmptyValidationError,
) -> Result<(), EmptyValidationError> {
    if data.is_empty() {
        Err(error)
    } else {
        Ok(())
    }
}

pub fn validate_entities(
    long_ops: &[OperationPoint],
    short_ops: &[OperationPoint],
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
) -> Result<(), EmptyValidationError> {
    ensure_not_empty(long_ops, EmptyValidationError::LongOperationPointsEmpty)?;
    ensure_not_empty(short_ops, EmptyValidationError::ShortOperationPointsEmpty)?;
    ensure_not_empty(indicators, EmptyValidationError::IndicatorsEmpty)?;
    ensure_not_empty(resampled_points, EmptyValidationError::ResampledPointsEmpty)?;

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
            validate_entities(
                &op_points_empty(),
                &op_points_empty(),
                &indicators_empty(),
                &resampled_points_empty()
            ),
            Err(EmptyValidationError::LongOperationPointsEmpty)
        );
    }

    #[test]
    fn test_long_empty_only() {
        assert_eq!(
            validate_entities(
                &op_points_empty(),
                &op_points_default(),
                &indicators_default(),
                &resampled_points_default()
            ),
            Err(EmptyValidationError::LongOperationPointsEmpty)
        );
    }

    #[test]
    fn test_short_empty_only() {
        assert_eq!(
            validate_entities(
                &op_points_default(),
                &op_points_empty(),
                &indicators_default(),
                &resampled_points_default()
            ),
            Err(EmptyValidationError::ShortOperationPointsEmpty)
        );
    }

    #[test]
    fn test_indicators_empty_only() {
        assert_eq!(
            validate_entities(
                &op_points_default(),
                &op_points_default(),
                &indicators_empty(),
                &resampled_points_default()
            ),
            Err(EmptyValidationError::IndicatorsEmpty)
        );
    }

    #[test]
    fn test_resampled_empty_only() {
        assert_eq!(
            validate_entities(
                &op_points_default(),
                &op_points_default(),
                &indicators_default(),
                &resampled_points_empty()
            ),
            Err(EmptyValidationError::ResampledPointsEmpty)
        );
    }

    #[test]
    fn test_all_have_data() {
        assert_eq!(
            validate_entities(
                &op_points_default(),
                &op_points_default(),
                &indicators_default(),
                &resampled_points_default()
            ),
            Ok(())
        );
    }
}
