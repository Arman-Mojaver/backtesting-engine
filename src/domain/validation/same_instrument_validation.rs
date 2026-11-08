use itertools::{AllEqualValueError, Itertools};

use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;

use super::ValidationError;

pub fn validate_same_instrument(
    resampled_points: &[ResampledPointD1],
    long_operation_points: &[OperationPoint],
    short_operation_points: &[OperationPoint],
) -> Result<(), ValidationError> {
    let mut instruments = resampled_points
        .iter()
        .map(|resampled_point| &resampled_point.instrument)
        .chain(long_operation_points.iter().map(|op| &op.instrument))
        .chain(short_operation_points.iter().map(|op| &op.instrument));

    if let Err(AllEqualValueError(Some([expected, found]))) = instruments.all_equal_value() {
        return Err(ValidationError::InstrumentMismatch {
            expected: expected.clone(),
            found: found.clone(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::*;

    fn instrument_mismatch(expected: &str, found: &str) -> Result<(), ValidationError> {
        Err(ValidationError::InstrumentMismatch {
            expected: expected.into(),
            found: found.into(),
        })
    }

    #[test]
    fn accepts_points_sharing_one_instrument() {
        let resampled = resampled_points_default();
        let long = op_points_default();
        let short = op_points_default();

        assert_eq!(validate_same_instrument(&resampled, &long, &short), Ok(()));
    }

    #[test]
    fn rejects_resampled_points_on_other_instrument() {
        let resampled = resampled_points_usdcad();
        let long = op_points_default();
        let short = op_points_default();

        assert_eq!(
            validate_same_instrument(&resampled, &long, &short),
            instrument_mismatch("USDCAD", "EURUSD")
        );
    }

    #[test]
    fn rejects_long_points_on_other_instrument() {
        let resampled = resampled_points_default();
        let long = op_points_default_usdcad();
        let short = op_points_default();

        assert_eq!(
            validate_same_instrument(&resampled, &long, &short),
            instrument_mismatch("EURUSD", "USDCAD")
        );
    }

    #[test]
    fn rejects_short_points_on_other_instrument() {
        let resampled = resampled_points_default();
        let long = op_points_default();
        let short = op_points_default_usdcad();

        assert_eq!(
            validate_same_instrument(&resampled, &long, &short),
            instrument_mismatch("EURUSD", "USDCAD")
        );
    }

    #[test]
    fn rejects_single_resampled_point_on_other_instrument() {
        let resampled = resampled_points_partial_mismatch();
        let long = op_points_default();
        let short = op_points_default();

        assert_eq!(
            validate_same_instrument(&resampled, &long, &short),
            instrument_mismatch("EURUSD", "USDCAD")
        );
    }

    #[test]
    fn rejects_single_long_point_on_other_instrument() {
        let resampled = resampled_points_default();
        let long = op_points_partial_mismatch();
        let short = op_points_default();

        assert_eq!(
            validate_same_instrument(&resampled, &long, &short),
            instrument_mismatch("EURUSD", "USDCAD")
        );
    }

    #[test]
    fn rejects_single_short_point_on_other_instrument() {
        let resampled = resampled_points_default();
        let long = op_points_default();
        let short = op_points_partial_mismatch();

        assert_eq!(
            validate_same_instrument(&resampled, &long, &short),
            instrument_mismatch("EURUSD", "USDCAD")
        );
    }
}
