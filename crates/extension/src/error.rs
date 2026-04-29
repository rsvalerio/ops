//! Error types for the extension framework.

use std::sync::Arc;

/// Cloneable wrapper for error sources, preserving the full error chain.
///
/// EFF-002: `Arc` enables `Clone` on `DataProviderError` without discarding the
/// original error's cause chain and Display output.
#[derive(Debug, Clone)]
pub struct SharedError(pub(crate) Arc<dyn std::error::Error + Send + Sync>);

impl std::fmt::Display for SharedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for SharedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl From<anyhow::Error> for SharedError {
    fn from(err: anyhow::Error) -> Self {
        // anyhow::Error → Box<dyn Error + Send + Sync> preserves the full source chain
        // via anyhow's std Into impl.
        let boxed: Box<dyn std::error::Error + Send + Sync> = err.into();
        Self(Arc::from(boxed))
    }
}

impl From<serde_json::Error> for SharedError {
    fn from(err: serde_json::Error) -> Self {
        Self(Arc::new(err))
    }
}

/// Error type for data provider operations.
///
/// EFF-002: Uses `SharedError` (Arc-wrapped) for `ComputationFailed` and
/// `Serialization` variants to preserve the full error chain while keeping
/// `Clone`. The `#[source]` attribute enables `Error::source()` traversal.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum DataProviderError {
    /// Returned when the requested provider name is not registered in the
    /// `DataRegistry`.
    ///
    /// Callers that warm up multiple providers (e.g. `let _ =
    /// ctx.get_or_provide("optional", reg)`) typically *expect* this variant
    /// for providers that are not part of the active stack and should
    /// silently ignore it.
    #[error("data provider not found: {0}")]
    NotFound(String),
    /// Returned when a registered provider's `provide(...)` method failed —
    /// e.g. an external command returned non-zero, an SQL query errored, or
    /// a filesystem read failed.
    ///
    /// The wrapped [`SharedError`] preserves the full source chain;
    /// `std::error::Error::source()` walks through to the originating cause.
    /// Use this variant to surface real failures (log + re-raise rather
    /// than swallow).
    #[error("data computation failed: {0}")]
    ComputationFailed(#[source] SharedError),
    /// Returned when a provider produced a value whose JSON shape could not
    /// be parsed back into the caller-expected struct (typically via
    /// `serde_json::from_value(...)`), or when constructing a JSON value
    /// itself failed.
    #[error("data serialization error: {0}")]
    Serialization(#[source] SharedError),
}

impl DataProviderError {
    pub fn not_found(name: &str) -> Self {
        Self::NotFound(name.to_string())
    }

    /// Create a computation failure from a string message.
    pub fn computation_failed(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        Self::ComputationFailed(SharedError(Arc::new(std::io::Error::other(msg))))
    }

    /// Create a computation failure from a source error, preserving the error chain.
    pub fn computation_error(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::ComputationFailed(SharedError(Arc::new(err)))
    }
}

impl From<anyhow::Error> for DataProviderError {
    fn from(err: anyhow::Error) -> Self {
        Self::ComputationFailed(SharedError::from(err))
    }
}

impl From<serde_json::Error> for DataProviderError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(SharedError::from(err))
    }
}
