//! Run configuration shared by both fixers.

use std::path::PathBuf;

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
    /// Select only the files git tracks, via `git ls-files` from `root`.
    /// When git is unavailable or `root` is not a repository, discovery
    /// silently widens to a full filesystem walk and records the reason in
    /// the report's fallback notice; see [`crate::discovery`].
    pub tracked_only: bool,
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
    #[must_use]
    pub const fn new(root: PathBuf, tracked_only: bool) -> Self {
        Self {
            root,
            tracked_only,
            max_bytes: DEFAULT_MAX_BYTES,
            check: false,
        }
    }

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
