//! Run configuration shared by both fixers.

use std::path::PathBuf;

/// Default per-file size cap.
///
/// Files larger than this are skipped and reported rather than read. Both
/// fixers hold the whole file in memory and `fix_trailing` allocates a second
/// buffer of the same size, so peak resident memory is roughly twice the
/// largest candidate — and the candidate set is repository-controlled. A
/// multi-gigabyte NUL-free file (a CSV export, an ndjson dump, a `.sql` seed,
/// a minified bundle) is ordinary in a repo and would otherwise OOM-kill a
/// `git commit`.
///
/// 16 MiB matches `ops-config-checkers`' `DEFAULT_MAX_BYTES` so the two
/// file-walking extensions agree on what "too big to hold" means. Nothing a
/// whitespace fixer should be editing comes close to it.
pub const DEFAULT_MAX_BYTES: u64 = 16 * 1024 * 1024;

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
