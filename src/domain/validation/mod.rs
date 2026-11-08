use thiserror::Error;

pub mod combined_validation;
pub mod empty_validation;
pub mod same_instrument_validation;
pub mod same_mms_validation;
pub mod same_timestamp_validation;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("{0}")]
    Empty(empty_validation::EmptyRepoValidationError),
    #[error("instruments must match across resampled points, long operation points, and short operation points")]
    InstrumentMismatch,
    #[error("all operation points must reference the expected money management strategy id")]
    MoneyManagementStrategyMismatch,
    #[error("operation point timestamps must match each other and be contained in the resampled point timestamps")]
    TimestampMismatch,
}
