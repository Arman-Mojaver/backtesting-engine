use crate::domain::indicator::Indicator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    IndicatorTypeMismatch,
}

pub fn validate_same_indicator_types(indicators: &[Indicator]) -> Result<(), ValidationError> {
    let reference = &indicators[0].r#type;

    if indicators
        .iter()
        .any(|indicator| &indicator.r#type != reference)
    {
        return Err(ValidationError::IndicatorTypeMismatch);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::*;

    #[test]
    fn test_all_equal() {
        let indicators = indicators_default();

        assert_eq!(validate_same_indicator_types(&indicators), Ok(()));
    }

    #[test]
    fn test_types_are_different() {
        let indicators = indicators_different_type();

        assert_eq!(
            validate_same_indicator_types(&indicators),
            Err(ValidationError::IndicatorTypeMismatch)
        );
    }
}
