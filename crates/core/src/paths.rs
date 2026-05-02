//! Shared platform path helpers.
//!
//! `home_dir` is the canonical "user home directory" probe used across the
//! crate. Resolution order:
//!
//! 1. `HOME` — present on every Unix shell, and on Windows when running under
//!    Git Bash / WSL / MSYS.
//! 2. `USERPROFILE` — Windows-native fallback (e.g. `C:\Users\X`).
//!
//! This is the *user home* directory. Code that wants the *config base*
//! (XDG / `%APPDATA%` / `$HOME/.config`) should use
//! `config::loader::global_config_path` instead, which layers platform
//! conventions on top of `home_dir`.
//!
//! CL-3 (TASK-0752): consolidates two divergent inline implementations that
//! lived in `expand.rs` (HOME → USERPROFILE) and `config/loader.rs` (HOME-only
//! on Unix, USERPROFILE-only on Windows). Future Windows-native polish should
//! only need to update this single function.

use std::path::PathBuf;

/// Resolve the current user's home directory from the environment.
///
/// Returns `None` if neither `HOME` nor `USERPROFILE` is set.
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}
