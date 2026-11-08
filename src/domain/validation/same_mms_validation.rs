use crate::domain::operation_point::OperationPoint;

use super::ValidationError;

pub fn validate_money_management_strategy_id_matches(
    long_operation_points: &[OperationPoint],
    short_operation_points: &[OperationPoint],
    expected_id: i32,
) -> Result<(), ValidationError> {
    let mismatched_id = long_operation_points
        .iter()
        .chain(short_operation_points)
        .map(|op| op.money_management_strategy_id)
        .find(|&id| id != expected_id);

    if let Some(found) = mismatched_id {
        return Err(ValidationError::MoneyManagementStrategyMismatch {
            expected: expected_id,
            found,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::*;

    const fn mms_mismatch(expected: i32, found: i32) -> Result<(), ValidationError> {
        Err(ValidationError::MoneyManagementStrategyMismatch { expected, found })
    }

    #[test]
    fn accepts_points_with_expected_mms_id() {
        let long = op_points_default();
        let short = op_points_default();

        assert_eq!(
            validate_money_management_strategy_id_matches(&long, &short, 1),
            Ok(())
        );
    }

    #[test]
    fn rejects_points_when_expected_mms_id_differs() {
        let long = op_points_default();
        let short = op_points_default();

        assert_eq!(
            validate_money_management_strategy_id_matches(&long, &short, 2),
            mms_mismatch(2, 1)
        );
    }

    #[test]
    fn rejects_long_points_with_other_mms_id() {
        let long = op_points_mms_id_2();
        let short = op_points_default();

        assert_eq!(
            validate_money_management_strategy_id_matches(&long, &short, 1),
            mms_mismatch(1, 2)
        );
    }

    #[test]
    fn rejects_short_points_with_other_mms_id() {
        let long = op_points_default();
        let short = op_points_mms_id_2();

        assert_eq!(
            validate_money_management_strategy_id_matches(&long, &short, 1),
            mms_mismatch(1, 2)
        );
    }

    #[test]
    fn rejects_single_long_point_with_other_mms_id() {
        let long = op_points_mms_partial_mismatch();
        let short = op_points_default();

        assert_eq!(
            validate_money_management_strategy_id_matches(&long, &short, 1),
            mms_mismatch(1, 2)
        );
    }

    #[test]
    fn rejects_single_short_point_with_other_mms_id() {
        let long = op_points_default();
        let short = op_points_mms_partial_mismatch();

        assert_eq!(
            validate_money_management_strategy_id_matches(&long, &short, 1),
            mms_mismatch(1, 2)
        );
    }
}
