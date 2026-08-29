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
        self.0.fmt(f)?;
        // anyhow-style alternate rendering: `{:#}` walks the source chain so
        // the root cause (e.g. "cargo llvm-cov exited with status 101: …")
        // reaches operator logs. Plain `{}` keeps the top-level message only.
        // Without this, callers formatting `DataProviderError` with `{e:#}`
        // saw just the outermost context — thiserror's nested `{0}` does not
        // propagate the alternate flag, and the conversion to
        // `Box<dyn Error>` in `From<anyhow::Error>` flattens anyhow's own
        // alternate-aware Display to its top context. Note the chain may
        // repeat a link whose Display already embeds its own sources
        // (e.g. `DbError::External` renders via `{0:#}`); duplication is
        // cosmetic, lost root causes are not.
        if f.alternate() {
            let mut source = self.0.source();
            while let Some(err) = source {
                write!(f, ": {err}")?;
                source = err.source();
            }
        }
        Ok(())
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
    ///
    /// # Why the message interpolates its own `#[source]`
    ///
    /// ERR-9 / TASK-1889: setting the message and the source to the same
    /// value makes chain-walking printers render each link twice, which is
    /// the textbook shape to avoid. It is kept deliberately, and the
    /// re-derivation was done against the display paths this type actually
    /// reaches in this workspace:
    ///
    /// | Path | Callers | Rendering |
    /// |---|---|---|
    /// | `{e:#}` in `tracing::warn!` | `extensions/about/src/providers.rs` (`warm_providers`), `extensions/about/src/lib.rs` (six enrichment sites) | needs the whole chain in one line |
    /// | `{e}` / `to_string()` | this crate's tests; any operator log that forgets the `#` | needs the whole chain in one line |
    /// | `{e:?}` via `anyhow::Error` | `extensions/create-review-tasks/src/lib.rs` (`fetch_review_targets`), `providers::load_or_default` | walks `source()` itself |
    ///
    /// The first two are the majority and they are the constraint: thiserror
    /// generates `write!(f, "…: {}", self.0)` for a plain `{0}`, which does
    /// **not** propagate the alternate flag to `SharedError`'s
    /// chain-walking Display — so dropping the `#` loses everything past the
    /// outermost context on every `{e:#}` and `{e}` site above. An
    /// alternate-aware `SharedError` (`error.rs`) does not change that: the
    /// flag never reaches it. The cost of keeping `{0:#}` is that
    /// `anyhow`'s `{:?}` repeats the chain under `Caused by:`. Duplication in
    /// one debug-formatted report is cheaper than a lost root cause in every
    /// operator warning, so `{0:#}` stays. `tests.rs` pins the rendering of
    /// all three paths.
    #[error("data computation failed: {0:#}")]
    ComputationFailed(#[source] SharedError),
    /// ERR-2 / TASK-1887: a computation failure described only by a message.
    ///
    /// [`DataProviderError::computation_failed`] used to build a
    /// `std::io::Error` purely as a container for its string, which put a
    /// false claim into the error chain: the value was indistinguishable —
    /// by type and by `ErrorKind` — from a real filesystem or process
    /// failure, so a caller doing
    /// `err.source().and_then(|s| s.downcast_ref::<std::io::Error>())` got a
    /// hit for an error that never touched a file descriptor. This variant
    /// carries the message directly, with no source at all. Its `Display`
    /// output matches [`DataProviderError::ComputationFailed`]'s, so the
    /// change is invisible to log readers.
    #[error("data computation failed: {0}")]
    ComputationMessage(String),
    /// Returned when a provider produced a value whose JSON shape could not
    /// be parsed back into the caller-expected struct (typically via
    /// `serde_json::from_value(...)`), or when constructing a JSON value
    /// itself failed.
    /// Mirrors [`DataProviderError::ComputationFailed`]'s `{0:#}` chain
    /// rendering — see that variant for the ERR-9 / TASK-1889 re-derivation
    /// — so serialization root causes stay visible in logs too.
    #[error("data serialization error: {0:#}")]
    Serialization(#[source] SharedError),
    /// SEC-38 / TASK-0744: returned when [`crate::Context::get_or_provide`]
    /// detects a re-entrant request for a key whose provider is still
    /// in-flight on the same context. A misconfigured or hostile extension
    /// that registers circular provider dependencies (A → B → A) would
    /// otherwise recurse until stack overflow.
    #[error("data provider cycle detected: {key}")]
    Cycle {
        /// The key whose provider re-entered itself transitively.
        key: String,
    },
}

impl DataProviderError {
    #[must_use]
    pub fn not_found(name: &str) -> Self {
        Self::NotFound(name.to_string())
    }

    /// Create a computation failure from a string message.
    ///
    /// ERR-2 / TASK-1887: produces [`DataProviderError::ComputationMessage`],
    /// which holds the message and nothing else. Reach for
    /// [`DataProviderError::computation_error`] instead whenever a real
    /// source error is available — that is what preserves a chain worth
    /// walking.
    #[must_use]
    pub fn computation_failed(msg: impl Into<String>) -> Self {
        Self::ComputationMessage(msg.into())
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
