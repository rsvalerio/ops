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
    pub root: PathBuf,
    pub tracked_only: bool,
    /// Per-file size cap; see [`DEFAULT_MAX_BYTES`]. Enforced on the read
    /// itself, not by a preceding `metadata()` call.
    pub max_bytes: u64,
}

impl FixerOptions {
    #[must_use]
    pub const fn new(root: PathBuf, tracked_only: bool) -> Self {
        Self {
            root,
            tracked_only,
            max_bytes: DEFAULT_MAX_BYTES,
        }
    }

    #[must_use]
    pub const fn with_max_bytes(mut self, max_bytes: u64) -> Self {
        self.max_bytes = max_bytes;
        self
    }
}
