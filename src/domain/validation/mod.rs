use thiserror::Error;

mod combined_validation;
mod empty_validation;
mod same_instrument_validation;
mod same_mms_validation;
mod same_timestamp_validation;

pub use combined_validation::validate_all;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("long operation points list is empty")]
    LongOperationPointsEmpty,
    #[error("short operation points list is empty")]
    ShortOperationPointsEmpty,
    #[error("indicators list is empty")]
    IndicatorsEmpty,
    #[error("resampled points list is empty")]
    ResampledPointsEmpty,
    #[error(
        "instruments must match across resampled points, long operation points, and short operation points, found {expected} and {found}"
    )]
    InstrumentMismatch { expected: String, found: String },
    #[error(
        "all operation points must reference money management strategy id {expected}, found {found}"
    )]
    MoneyManagementStrategyMismatch { expected: i32, found: i32 },
    #[error(
        "long and short operation points must share the same timestamps, timestamp {timestamp} is only on one side"
    )]
    LongShortTimestampsDiffer { timestamp: i64 },
    #[error("operation point timestamp {timestamp} has no matching resampled point")]
    TimestampMissingFromResampled { timestamp: i64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_name_the_violated_constraint() {
        let cases = [
            (
                ValidationError::LongOperationPointsEmpty,
                "long operation points list is empty",
            ),
            (
                ValidationError::ShortOperationPointsEmpty,
                "short operation points list is empty",
            ),
            (ValidationError::IndicatorsEmpty, "indicators list is empty"),
            (
                ValidationError::ResampledPointsEmpty,
                "resampled points list is empty",
            ),
            (
                ValidationError::InstrumentMismatch {
                    expected: "EURUSD".into(),
                    found: "USDCAD".into(),
                },
                "instruments must match across resampled points, long operation points, and short operation points, found EURUSD and USDCAD",
            ),
            (
                ValidationError::MoneyManagementStrategyMismatch {
                    expected: 1,
                    found: 2,
                },
                "all operation points must reference money management strategy id 1, found 2",
            ),
            (
                ValidationError::LongShortTimestampsDiffer { timestamp: 4 },
                "long and short operation points must share the same timestamps, timestamp 4 is only on one side",
            ),
            (
                ValidationError::TimestampMissingFromResampled { timestamp: 4 },
                "operation point timestamp 4 has no matching resampled point",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(
                error.to_string(),
                expected,
                "Display mismatch for {error:?}"
            );
        }
    }

    #[test]
    fn validation_error_is_a_std_error_object() {
        let boxed: Box<dyn std::error::Error> = Box::new(ValidationError::IndicatorsEmpty);
        assert_eq!(boxed.to_string(), "indicators list is empty");
        assert!(boxed.source().is_none());
    }
}
