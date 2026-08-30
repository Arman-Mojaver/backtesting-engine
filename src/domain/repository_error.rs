//! Error type for repository ports.
//!
//! Repository traits (ports) live in the domain layer; adapters fill them in
//! with concrete I/O. This error is the failure channel for those ports: it
//! records *what kind* of operation failed, a human-readable *context*
//! describing the operation, and preserves the underlying *source* error so
//! chains remain intact up to the application boundary (`anyhow`).
//!
//! The domain layer defines only the error shape — it never depends on a
//! concrete storage crate. Adapters construct the errors.

use std::error::Error as StdError;
use std::fmt;

/// Convenience alias for results returned by repository ports.
pub type RepositoryResult<T> = Result<T, RepositoryError>;

/// Why a repository operation failed.
///
/// The distinction is actionable: a [`RepositoryErrorKind::Connection`]
/// failure may be worth retrying, while a [`RepositoryErrorKind::Decode`]
/// failure points at stored data that does not match the expected shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryErrorKind {
    /// The connection to the backing store could not be established.
    Connection,
    /// A query against the backing store failed.
    Query,
    /// A stored row could not be decoded into a domain type.
    Decode,
}

impl fmt::Display for RepositoryErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepositoryErrorKind::Connection => write!(f, "connection"),
            RepositoryErrorKind::Query => write!(f, "query"),
            RepositoryErrorKind::Decode => write!(f, "decode"),
        }
    }
}

/// Error returned by repository ports.
///
/// Built by adapters via the helper constructors ([`RepositoryError::connection`],
/// [`RepositoryError::query`], [`RepositoryError::decode`],
/// [`RepositoryError::invalid_data`]).
#[derive(Debug)]
pub struct RepositoryError {
    kind: RepositoryErrorKind,
    context: String,
    source: Option<Box<dyn StdError + Send + Sync + 'static>>,
}

impl RepositoryError {
    /// The connection to the backing store could not be established.
    pub fn connection(
        context: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::new(
            RepositoryErrorKind::Connection,
            context,
            Some(Box::new(source)),
        )
    }

    /// A query against the backing store failed.
    pub fn query(
        context: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::new(RepositoryErrorKind::Query, context, Some(Box::new(source)))
    }

    /// A stored row could not be decoded, with an underlying error
    /// (e.g. a type mismatch or a parse failure).
    pub fn decode(
        context: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::new(RepositoryErrorKind::Decode, context, Some(Box::new(source)))
    }

    /// Stored data is invalid on its own terms (e.g. a required key is
    /// missing) — there is no underlying error to preserve.
    pub fn invalid_data(context: impl Into<String>) -> Self {
        Self::new(RepositoryErrorKind::Decode, context, None)
    }

    fn new(
        kind: RepositoryErrorKind,
        context: impl Into<String>,
        source: Option<Box<dyn StdError + Send + Sync + 'static>>,
    ) -> Self {
        Self {
            kind,
            context: context.into(),
            source,
        }
    }

    /// The class of failure (connection, query, or decode).
    pub fn kind(&self) -> RepositoryErrorKind {
        self.kind
    }
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "repository {} failure: {}", self.kind, self.context)
    }
}

impl StdError for RepositoryError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|b| &**b as &(dyn StdError + 'static))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    /// A stand-in for a driver error (e.g. `postgres::Error`).
    #[derive(Debug)]
    struct FakeDriverError(&'static str);

    impl fmt::Display for FakeDriverError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "driver error: {}", self.0)
        }
    }

    impl StdError for FakeDriverError {}

    #[test]
    fn display_reports_kind_and_context() {
        let err = RepositoryError::query("fetching indicators", FakeDriverError("boom"));
        assert_eq!(
            err.to_string(),
            "repository query failure: fetching indicators"
        );
    }

    #[test]
    fn source_chain_is_preserved() {
        let err =
            RepositoryError::connection("connecting to PostgreSQL", FakeDriverError("refused"));
        assert_eq!(
            err.source().map(|s| s.to_string()),
            Some("driver error: refused".to_string())
        );
    }

    #[test]
    fn kind_distinguishes_failure_classes() {
        assert_eq!(
            RepositoryError::connection("c", FakeDriverError("x")).kind(),
            RepositoryErrorKind::Connection
        );
        assert_eq!(
            RepositoryError::query("q", FakeDriverError("x")).kind(),
            RepositoryErrorKind::Query
        );
        assert_eq!(
            RepositoryError::decode("d", FakeDriverError("x")).kind(),
            RepositoryErrorKind::Decode
        );
    }

    #[test]
    fn invalid_data_is_a_decode_error_without_source() {
        let err = RepositoryError::invalid_data("unknown indicator type: \"xyz\"");
        assert_eq!(err.kind(), RepositoryErrorKind::Decode);
        assert!(err.source().is_none());
        assert_eq!(
            err.to_string(),
            "repository decode failure: unknown indicator type: \"xyz\""
        );
    }

    #[test]
    fn repository_error_is_send_and_sync() {
        // Required for `anyhow` interop and for use inside `rayon` workers.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RepositoryError>();
    }
}
