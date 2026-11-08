use crate::domain::operation_point::OperationPoint;

use super::ValidationError;

pub fn validate_money_management_strategy_id_matches(
    long_points: &[OperationPoint],
    short_points: &[OperationPoint],
    expected_id: i32,
) -> Result<(), ValidationError> {
    if long_points
        .iter()
        .map(|op| op.money_management_strategy_id)
        .chain(
            short_points
                .iter()
                .map(|op| op.money_management_strategy_id),
        )
        .any(|money_management_strategy_id| money_management_strategy_id != expected_id)
    {
        return Err(ValidationError::MoneyManagementStrategyMismatch);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::*;

    #[test]
    fn test_all_equal() {
        let long = op_points_default();
        let short = op_points_default();

        assert_eq!(
            validate_money_management_strategy_id_matches(&long, &short, 1),
            Ok(())
        );
    }

    #[test]
    fn test_expected_id_different() {
        let long = op_points_default();
        let short = op_points_default();

        assert_eq!(
            validate_money_management_strategy_id_matches(&long, &short, 2),
            Err(ValidationError::MoneyManagementStrategyMismatch)
        );
    }

    #[test]
    fn test_long_mismatch() {
        let long = op_points_mms_id_2();
        let short = op_points_default();

        assert_eq!(
            validate_money_management_strategy_id_matches(&long, &short, 1),
            Err(ValidationError::MoneyManagementStrategyMismatch)
        );
    }

    #[test]
    fn test_short_mismatch() {
        let long = op_points_default();
        let short = op_points_mms_id_2();

        assert_eq!(
            validate_money_management_strategy_id_matches(&long, &short, 1),
            Err(ValidationError::MoneyManagementStrategyMismatch)
        );
    }

    #[test]
    fn test_long_partial_mismatch() {
        let long = op_points_mms_partial_mismatch();
        let short = op_points_default();

        assert_eq!(
            validate_money_management_strategy_id_matches(&long, &short, 1),
            Err(ValidationError::MoneyManagementStrategyMismatch)
        );
    }

    #[test]
    fn test_short_partial_mismatch() {
        let long = op_points_default();
        let short = op_points_mms_partial_mismatch();

        assert_eq!(
            validate_money_management_strategy_id_matches(&long, &short, 1),
            Err(ValidationError::MoneyManagementStrategyMismatch)
        );
    }
}
