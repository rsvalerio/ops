//! Error types for the extension framework.

use std::sync::Arc;

/// Cloneable wrapper for error sources, preserving the full error chain.
///
/// The `Arc` is what lets `DataProviderError` be `Clone` without discarding
/// the original error's cause chain and `Display` output.
#[derive(Debug, Clone)]
pub struct SharedError(Inner);

/// An `anyhow::Error` is kept as itself rather than flattened into
/// `Arc<dyn Error>`.
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

    /// Whether two handles share one allocation — the observable signal that
    /// `Clone` reuses the wrapped error instead of rewrapping it. Test-facing;
    /// the representation is private.
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
    /// Yields the **wrapped error itself**, not the wrapped error's own
    /// source.
    ///
    /// The wrapped error is the whole point of this type, so it must appear
    /// in the chain. Returning `self.0.source()` would skip that link, and
    /// every caller doing the standard typed-error classification —
    /// `err.source().and_then(|s| s.downcast_ref::<T>())`, or a walk over
    /// `source()` — would miss the one object it was looking for.
    /// `extensions-rust/about`'s `is_manifest_missing` depends on this: it
    /// looks for `FindWorkspaceRootError::NotFound` to tell "this is not a
    /// Rust project" from "the manifest failed to read", and without the
    /// wrapped error in the chain both answer `false`, turning every non-Rust
    /// directory into a `warn`.
    ///
    /// The `Display` impl above is the mirror image: it prints the wrapped
    /// error as its first link and then walks *that* error's sources, so
    /// `{:#}` output neither duplicates nor drops a link relative to a
    /// `source()` walk.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.as_error())
    }
}

impl From<anyhow::Error> for SharedError {
    fn from(err: anyhow::Error) -> Self {
        // Deliberately *not* `Box<dyn Error>`: that conversion hands back
        // anyhow's own `ErrorImpl<E>` wrapper, which renders correctly but
        // makes the originating `E` undowncastable, defeating every
        // typed-error classification downstream. See `Inner`.
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
/// The `ComputationFailed` and `Serialization` variants wrap an
/// `Arc`-backed [`SharedError`], which preserves the full error chain while
/// keeping the enum `Clone`. Their `#[source]` attribute enables
/// `Error::source()` traversal.
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
    /// Setting the message and the source to the same value makes
    /// chain-walking printers render each link twice — the textbook shape
    /// to avoid — but it is kept deliberately. This error reaches
    /// operators predominantly through `{e:#}` in `tracing::warn!` and
    /// plain `{e}` / `to_string()`, and thiserror generates
    /// `write!(f, "…: {}", self.0)` for a plain `{0}`, which does **not**
    /// propagate the alternate flag to `SharedError`'s chain-walking
    /// Display — so dropping the `#` would lose everything past the
    /// outermost context in every one of those logs. The cost of keeping
    /// `{0:#}` is that `anyhow`'s `{:?}` repeats the chain under
    /// `Caused by:`. Duplication in one debug-formatted report is cheaper
    /// than a lost root cause in every operator warning, so `{0:#}`
    /// stays. `tests.rs` pins the rendering of all three paths.
    #[error("data computation failed: {0:#}")]
    ComputationFailed(#[source] SharedError),
    /// A computation failure described only by a message.
    ///
    /// The message is carried directly and the variant has **no source at
    /// all**, so nothing false enters the error chain. Manufacturing a
    /// carrier error — a `std::io::Error` built purely to hold the string,
    /// say — would be indistinguishable by type and by `ErrorKind` from a
    /// real filesystem or process failure, and a caller doing
    /// `err.source().and_then(|s| s.downcast_ref::<std::io::Error>())` would
    /// get a hit for an error that never touched a file descriptor.
    ///
    /// Its `Display` output matches
    /// [`DataProviderError::ComputationFailed`]'s, so log readers see no
    /// difference between the two.
    #[error("data computation failed: {0}")]
    ComputationMessage(String),
    /// Returned when a provider produced a value whose JSON shape could not
    /// be parsed back into the caller-expected struct (typically via
    /// `serde_json::from_value(...)`), or when constructing a JSON value
    /// itself failed.
    /// Mirrors [`DataProviderError::ComputationFailed`]'s `{0:#}` chain
    /// rendering (see that variant for why) so serialization root causes
    /// stay visible in logs too.
    #[error("data serialization error: {0:#}")]
    Serialization(#[source] SharedError),
    /// Returned when a dispatched provider ran past the wall-clock budget
    /// carried by the [`crate::Context`].
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
    /// Returned when [`crate::Context::get_or_provide`] detects a
    /// re-entrant request for a key whose provider is still
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
    /// Produces [`DataProviderError::ComputationMessage`], which holds the
    /// message and nothing else. Reach for
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

/// Unwraps a `DataProviderError` that merely travelled inside an
/// `anyhow::Error`, rather than re-wrapping it as
/// [`DataProviderError::ComputationFailed`].
///
/// Several provider entry points are `anyhow`-typed free functions —
/// `collect_tokei`, `collect_rust_loc`, everything reached through
/// `ops_sqlite::try_provide_from_db`'s fallback closure — so a deadline check
/// inside one of them can only propagate its [`DataProviderError::TimedOut`]
/// by boxing it into `anyhow`. Without this downcast the round trip would
/// degrade a *typed* timeout into an opaque computation failure: the message
/// survives, but nothing can match on the variant, so a caller cannot tell a
/// stall from a broken provider.
///
/// **`anyhow::Error::downcast` searches the whole cause chain**, so this also
/// unwraps a `DataProviderError` that picked up `.context(..)` on the way out
/// — and that context is then dropped, since the variants carry no free-text
/// field to hold it. That is deliberate but narrow: do not `.context(..)` a
/// `DataProviderError` you intend to send back through this conversion.
/// Nothing in-tree does; the crate's own error type is the *outer* one
/// everywhere, and a provider that wants to add wording should return
/// [`DataProviderError::computation_failed`] with it instead. The alternative
/// — matching only an unwrapped error, which anyhow cannot express — would
/// leave the common case (a walker's bare `?`) opaque in order to protect a
/// layering mistake.
impl From<anyhow::Error> for DataProviderError {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast::<Self>() {
            Ok(typed) => typed,
            Err(other) => Self::ComputationFailed(SharedError::from(other)),
        }
    }
}

impl From<serde_json::Error> for DataProviderError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(SharedError::from(err))
    }
}

/// These tests stay beside `error.rs` because they need private access —
/// `SharedError::new` and `SharedError::shares_allocation_with` — which the
/// integration suite in `tests/public_api.rs` cannot reach. Every other
/// error-type test lives there.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_error_display_shows_inner_message() {
        let inner = std::io::Error::other("disk full");
        let shared = SharedError::new(inner);
        assert_eq!(shared.to_string(), "disk full");
    }

    #[test]
    fn shared_error_source_chain_preserved() {
        use std::error::Error;
        // A custom error with a source
        #[derive(Debug)]
        struct Outer(std::io::Error);
        impl std::fmt::Display for Outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "outer")
            }
        }
        impl std::error::Error for Outer {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }
        let outer = Outer(std::io::Error::other("root cause"));
        let shared = SharedError::new(outer);

        // ERR-1 / TASK-2024: the first link is the *wrapped error itself*. This
        // test previously asserted that `source()` skipped straight to "root
        // cause", which is exactly the missing link the fix restores — the
        // wrapped `Outer` was unreachable by any chain walk or downcast.
        let first = shared
            .source()
            .expect("the wrapped error is the first link");
        assert_eq!(first.to_string(), "outer");
        assert!(
            first.downcast_ref::<Outer>().is_some(),
            "the wrapped error must be downcastable through the chain"
        );

        // …and the rest of the chain still follows from there.
        let root = first.source().expect("the wrapped error's own source");
        assert!(root.to_string().contains("root cause"), "got: {root}");
    }

    /// A sourceless error renders identically with and without the alternate
    /// flag — the chain walk must not append separators to nothing.
    #[test]
    fn shared_error_alternate_display_matches_plain_when_no_sources() {
        let shared = SharedError::new(std::io::Error::other("disk full"));
        assert_eq!(shared.to_string(), "disk full");
        assert_eq!(format!("{shared:#}"), "disk full");
    }

    /// `SharedError`'s alternate rendering walks `self.0.source()` after
    /// printing `self.0`, independently of the `Error::source()` impl, so
    /// `{:#}` prints no link twice and drops none even though `source()`
    /// yields the wrapped error itself.
    #[test]
    fn source_fix_leaves_the_alternate_display_unchanged() {
        #[derive(Debug)]
        struct Layered(std::io::Error);
        impl std::fmt::Display for Layered {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("outer context")
            }
        }
        impl std::error::Error for Layered {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }

        let shared = SharedError::new(Layered(std::io::Error::other("root cause")));
        assert_eq!(format!("{shared}"), "outer context");
        assert_eq!(format!("{shared:#}"), "outer context: root cause");

        let e = DataProviderError::ComputationFailed(shared);
        assert_eq!(
            e.to_string(),
            "data computation failed: outer context: root cause"
        );
    }

    #[test]
    fn data_provider_error_is_clone() {
        #[derive(Debug)]
        struct WithSource(std::io::Error);
        impl std::fmt::Display for WithSource {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("outer")
            }
        }
        impl std::error::Error for WithSource {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }

        let err = DataProviderError::computation_error(WithSource(std::io::Error::other("inner")));
        let cloned = err.clone();

        assert_eq!(err.to_string(), cloned.to_string());
        assert!(matches!(cloned, DataProviderError::ComputationFailed(_)));
        // Source chain survives the clone.
        assert!(std::error::Error::source(&cloned).is_some());

        // EFF-002: Clone reuses the inner Arc rather than rewrapping the error.
        let (
            DataProviderError::ComputationFailed(orig),
            DataProviderError::ComputationFailed(copy),
        ) = (&err, &cloned)
        else {
            panic!("expected ComputationFailed variants");
        };
        assert!(orig.shares_allocation_with(copy));
    }
}
