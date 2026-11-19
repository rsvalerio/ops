//! Run configuration shared by both fixers.

use std::path::PathBuf;

use crate::discovery::Scope;

// One definition, re-exported — the shared cap lives in
// `ops_core::bounded_read` next to the read that enforces it, so the fixers
// and the config checkers cannot drift on what "too big to hold" means. The
// full rationale (peak resident memory is roughly twice the largest
// candidate) is documented at the definition.
pub use ops_core::bounded_read::DEFAULT_MAX_BYTES;

/// Options for both fixers.
#[derive(Debug, Clone)]
pub struct FixerOptions {
    /// Directory the fixers walk for candidate files. Per-file paths in the
    /// report are rendered relative to it where possible.
    pub root: PathBuf,
    /// Where candidates come from: a walk of `root`, or only the files git
    /// tracks. A [`Scope::Tracked`] run whose git is unavailable, or whose
    /// `root` is not a repository, widens to a full walk and says so in a
    /// fallback notice; see [`crate::discovery`].
    pub scope: Scope,
    /// Per-file size cap; see [`DEFAULT_MAX_BYTES`]. Enforced on the read
    /// itself, not by a preceding `metadata()` call.
    pub max_bytes: u64,
    /// Report the files a fix would change without writing any of them —
    /// the CI mode, where a gate must fail on a dirty tree rather than
    /// repair it. The exit-code contract is unchanged: a file that needs
    /// fixing still lands in [`crate::FixerReport::files_changed`].
    pub check: bool,
}

impl FixerOptions {
    /// Options for fixing the `scope` candidates under `root`, with the
    /// defaults applied: `max_bytes` = [`DEFAULT_MAX_BYTES`] and `check` =
    /// `false`, so files are rewritten.
    #[must_use]
    pub const fn new(root: PathBuf, scope: Scope) -> Self {
        Self { root, scope, max_bytes: DEFAULT_MAX_BYTES, check: false }
    }

    /// Override the per-file size cap; see [`FixerOptions::max_bytes`].
    #[must_use]
    pub const fn with_max_bytes(mut self, max_bytes: u64) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    /// Switch to check-only mode; see [`FixerOptions::check`].
    #[must_use]
    pub const fn with_check(mut self, check: bool) -> Self {
        self.check = check;
        self
    }
}
