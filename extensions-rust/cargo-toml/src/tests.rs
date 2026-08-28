//! Tests for `cargo_toml` extension.

use super::*;

/// TEST-18 / TASK-1802: helper for the two tests that make DAC permission
/// bits the mechanism under test.
///
/// Both problems those tests had are handled here:
///
/// 1. **Panic-safe cleanup.** [`PermGuard`] restores the original mode from
///    `Drop`, so a panic anywhere in the test body still leaves the
///    `TempDir` removable instead of leaking an undeletable 0o000 directory
///    into the temp filesystem for every subsequent run.
/// 2. **Environment-dependent outcome.** `CAP_DAC_OVERRIDE` (uid 0 — the
///    default in most CI container images) bypasses 0o000 entirely, and some
///    mounts ignore mode bits altogether. [`PermGuard::deny_all`] returns
///    `None` when the environment cannot express the condition, so the
///    caller skips rather than asserting an outcome the environment cannot
///    produce.
#[cfg(unix)]
pub struct PermGuard {
    path: std::path::PathBuf,
    restore_mode: u32,
}

#[cfg(unix)]
impl PermGuard {
    /// Set `path` to 0o000 and verify the restriction actually took effect by
    /// probing it with `probe`.
    ///
    /// Returns `None` — with the original mode already restored — when the
    /// probe still succeeds, meaning the caller is running as root or on a
    /// filesystem that ignores mode bits.
    pub fn deny_all(
        path: &std::path::Path,
        probe: impl Fn(&std::path::Path) -> std::io::Result<()>,
    ) -> Option<Self> {
        use std::os::unix::fs::PermissionsExt as _;

        let restore_mode = std::fs::metadata(path)
            .expect("stat target before chmod")
            .permissions()
            .mode();
        // Not `.ok()`: the test's correctness depends on this having taken
        // effect, so a failure must surface rather than be swallowed.
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000))
            .expect("chmod 0o000 must succeed");
        let guard = Self {
            path: path.to_path_buf(),
            restore_mode,
        };

        if probe(path).is_ok() {
            // Dropping `guard` restores the mode.
            return None;
        }
        Some(guard)
    }
}

#[cfg(unix)]
impl Drop for PermGuard {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt as _;
        // Best-effort: a failure here cannot be reported from `Drop`, and
        // panicking during unwind would abort the test process.
        let _ = std::fs::set_permissions(
            &self.path,
            std::fs::Permissions::from_mode(self.restore_mode),
        );
    }
}

/// TEST-18 / TASK-1802: uniform message for the skip path, so a green run on
/// a root/CI container is self-explaining rather than silently degraded.
#[cfg(unix)]
pub fn skip_no_dac_enforcement(test: &str) {
    eprintln!(
        "skipping {test}: this environment does not enforce 0o000 (running as uid 0, or a \
         filesystem that ignores mode bits)"
    );
}

mod extension;
mod find_root;
mod inheritance;
mod parse_edge;
mod provider;
mod types;
