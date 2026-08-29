//! Error types for the extension framework.

use std::sync::Arc;

/// Cloneable wrapper for error sources, preserving the full error chain.
///
/// EFF-002: `Arc` enables `Clone` on `DataProviderError` without discarding the
/// original error's cause chain and Display output.
#[derive(Debug, Clone)]
pub struct SharedError(Inner);

/// ERR-1 / TASK-2024: an `anyhow::Error` is kept as itself rather than
/// flattened into `Arc<dyn Error>`.
///
/// `anyhow::Error` converts into `Box<dyn Error + Send + Sync>` by boxing its
/// own internal `ErrorImpl<E>` wrapper, not the `E` it was built from. That
/// box renders and chains correctly, but it is a *different concrete type*, so
/// `downcast_ref::<E>()` on it — and on every link a chain walk reaches
/// through it — misses. Storing the `anyhow::Error` lets `source()` hand out
/// `AsRef::<dyn Error>::as_ref`, which is the original `E` erased and is
/// downcastable, so typed-error classification through `DataProviderError`
/// works for the anyhow-built errors that make up most of this workspace's
/// provider failures.
#[derive(Debug, Clone)]
enum Inner {
    Std(Arc<dyn std::error::Error + Send + Sync>),
    Anyhow(Arc<anyhow::Error>),
}

impl SharedError {
    /// Wrap a concrete error. Use [`SharedError::from`] for an
    /// `anyhow::Error`, which needs the representation above.
    pub(crate) fn new(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(Inner::Std(Arc::new(err)))
    }

    /// EFF-002: whether two handles share one allocation — the observable
    /// signal that `Clone` reuses the wrapped error instead of rewrapping it.
    /// Test-facing; the representation is private.
    #[cfg(test)]
    pub(crate) fn shares_allocation_with(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Inner::Std(a), Inner::Std(b)) => Arc::ptr_eq(a, b),
            (Inner::Anyhow(a), Inner::Anyhow(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }

    /// The wrapped error erased to `&dyn Error` — the first link of the chain
    /// and what [`std::error::Error::source`] hands out.
    fn as_error(&self) -> &(dyn std::error::Error + 'static) {
        match &self.0 {
            Inner::Std(e) => &**e,
            // `anyhow::Error: AsRef<dyn Error + Send + Sync>` yields the
            // originating error itself, not anyhow's wrapper.
            Inner::Anyhow(e) => e.as_ref().as_ref(),
        }
    }
}

impl std::fmt::Display for SharedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The anyhow representation already renders its own chain under `{:#}`
        // with exactly the `: `-joined shape the manual walk below produces,
        // so it is delegated wholesale rather than walked a second time —
        // walking it here would print its first link twice.
        if let Inner::Anyhow(err) = &self.0 {
            return if f.alternate() {
                write!(f, "{err:#}")
            } else {
                write!(f, "{err}")
            };
        }
        let inner = self.as_error();
        std::fmt::Display::fmt(inner, f)?;
        // anyhow-style alternate rendering: `{:#}` walks the source chain so
        // the root cause (e.g. "cargo llvm-cov exited with status 101: …")
        // reaches operator logs. Plain `{}` keeps the top-level message only.
        // Without this, callers formatting `DataProviderError` with `{e:#}`
        // saw just the outermost context — thiserror's nested `{0}` does not
        // propagate the alternate flag. Note the chain may repeat a link whose
        // Display already embeds its own sources (e.g. `DbError::External`
        // renders via `{0:#}`); duplication is cosmetic, lost root causes are
        // not.
        if f.alternate() {
            let mut source = inner.source();
            while let Some(err) = source {
                write!(f, ": {err}")?;
                source = err.source();
            }
        }
        Ok(())
    }
}

impl std::error::Error for SharedError {
    /// ERR-1 / TASK-2024: yields the **wrapped error itself**, not the wrapped
    /// error's own source.
    ///
    /// This used to return `self.0.source()`, which skipped a link: the error
    /// this type exists to preserve never appeared in the chain at all. Every
    /// caller doing the standard typed-error classification —
    /// `err.source().and_then(|s| s.downcast_ref::<T>())`, or a walk over
    /// `source()` — therefore missed on the one object it was looking for.
    /// `extensions-rust/about`'s `is_manifest_missing` is the concrete
    /// casualty: it looks for `FindWorkspaceRootError::NotFound` to tell "this
    /// is not a Rust project" from "the manifest failed to read", and returned
    /// `false` for both, so every non-Rust directory produced a `warn`.
    ///
    /// The mirror-image `Display` impl above already printed the wrapped error
    /// as its first link and then walked *that* error's sources, so it stays
    /// as it was: fixing `source()` neither duplicates nor drops anything in
    /// `{:#}` output, it only makes a chain walk see what the message was
    /// showing all along.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.as_error())
    }
}

impl From<anyhow::Error> for SharedError {
    fn from(err: anyhow::Error) -> Self {
        // ERR-1 / TASK-2024: deliberately *not* `Box<dyn Error>`. That
        // conversion hands back anyhow's own `ErrorImpl<E>` wrapper, which
        // renders correctly but makes the originating `E` undowncastable, so
        // every typed-error classification downstream missed. See `Inner`.
        Self(Inner::Anyhow(Arc::new(err)))
    }
}

impl From<serde_json::Error> for SharedError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(err)
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
    /// SEC-33 / TASK-2017: returned when a dispatched provider ran past the
    /// wall-clock budget carried by the [`crate::Context`].
    ///
    /// The budget is installed by [`crate::DataRegistry::provide`] for the
    /// outermost provider of a traversal and inherited by everything that
    /// provider composes, so the bound covers the whole dispatch rather than
    /// resetting at each level. Providers doing long, chunkable work
    /// (directory walks, repeated external commands) poll
    /// [`crate::Context::check_deadline`] and return this variant themselves;
    /// providers that never poll still surface it, because the dispatch point
    /// converts an over-budget return into this variant instead of handing
    /// the caller a value produced after the deadline. Either way the failure
    /// names the provider, so an operator log identifies which one stalled.
    #[error("data provider timed out: {provider} exceeded its {budget:?} budget")]
    TimedOut {
        /// The provider that owned the budget that was exceeded.
        provider: String,
        /// The wall-clock budget it was given.
        budget: std::time::Duration,
    },
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
        Self::ComputationFailed(SharedError::new(err))
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
