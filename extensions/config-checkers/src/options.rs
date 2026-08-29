//! Run configuration shared by both checkers.

use std::path::PathBuf;

use crate::DEFAULT_MAX_BYTES;

/// Options shared by both checkers.
#[derive(Debug, Clone)]
pub struct CheckerOptions {
    pub root: PathBuf,
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
    #[must_use]
    pub const fn new(root: PathBuf, tracked_only: bool) -> Self {
        Self {
            root,
            tracked_only,
            allow_json5: false,
            max_bytes: DEFAULT_MAX_BYTES,
        }
    }

    #[must_use]
    pub const fn with_allow_json5(mut self, allow: bool) -> Self {
        self.allow_json5 = allow;
        self
    }

    #[must_use]
    pub const fn with_max_bytes(mut self, max_bytes: u64) -> Self {
        self.max_bytes = max_bytes;
        self
    }
}
