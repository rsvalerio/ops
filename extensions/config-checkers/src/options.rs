//! Run configuration shared by both checkers.

use std::path::PathBuf;

use crate::DEFAULT_MAX_BYTES;

/// Options shared by both checkers.
#[derive(Debug, Clone)]
pub struct CheckerOptions {
    /// Directory the discovery walk descends from; every reported failure
    /// path is relative to it.
    pub root: PathBuf,
    /// Selects the candidate set: `true` consults the git index
    /// (`git ls-files`, tracked files only, honouring skip-worktree), while
    /// `false` walks the directory tree honouring gitignore rules — the two
    /// sets differ on untracked and ignored files.
    pub tracked_only: bool,
    /// JSON only: accept JSON5 (a strict superset of JSONC — comments and
    /// trailing commas, plus unquoted keys, single-quoted strings, hex
    /// numbers, etc.).
    pub allow_json5: bool,
    /// Per-file size cap. Files larger than this are skipped without being
    /// parsed, and the cap is enforced on the read itself, not only on a
    /// prior `metadata()` call.
    pub max_bytes: u64,
}

impl CheckerOptions {
    /// Creates options for `root` with `tracked_only` selecting the
    /// candidate set, JSON5 off, and the default byte cap.
    #[must_use]
    pub const fn new(root: PathBuf, tracked_only: bool) -> Self {
        Self {
            root,
            tracked_only,
            allow_json5: false,
            max_bytes: DEFAULT_MAX_BYTES,
        }
    }

    /// Enables JSON5 acceptance (JSON checker only).
    #[must_use]
    pub const fn with_allow_json5(mut self, allow: bool) -> Self {
        self.allow_json5 = allow;
        self
    }

    /// Overrides the per-file byte cap.
    #[must_use]
    pub const fn with_max_bytes(mut self, max_bytes: u64) -> Self {
        self.max_bytes = max_bytes;
        self
    }
}
