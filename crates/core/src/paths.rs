//! Shared platform path helpers.
//!
//! `home_dir` is the canonical "user home directory" probe used across the
//! crate. Resolution order is platform-gated:
//!
//! - **Unix (`cfg(unix)`):** `HOME` only. ARCH-2 / TASK-0891: a polluted
//!   `USERPROFILE` (attacker- or shell-rc-supplied) must NOT be honored
//!   as `$HOME` — `home_dir` is now the single source of truth for `~`
//!   expansion in argv / cwd / env values, so a fallback there would
//!   redirect every tilde-resolved path.
//! - **Windows / non-Unix:** `HOME` first (set by Git Bash / WSL / MSYS),
//!   then `USERPROFILE` (`C:\Users\X`) as the Windows-native fallback.
//!
//! READ-1 / TASK-1434: the **HOME-first precedence on non-Unix is
//! deliberate**. Cross-platform tooling that ops users typically already
//! have installed (Git Bash, WSL, MSYS, Cygwin) sets `HOME` to a Unix-style
//! path that points at the user's *intended* home for tooling — e.g.
//! `/c/Users/X` rather than `C:\Users\X`. Honouring that first keeps `~`
//! expansion consistent across the user's bash and PowerShell sessions.
//! `USERPROFILE` is the Windows-native fallback for the rare native-only
//! shell that has not been touched by any cross-platform tooling. The
//! WSL/MSYS leakage trade-off (a process inheriting Unix-style `HOME` from
//! a polluted parent shell) is the same one
//! [`crate::config::loader::global_config_path`] documents for
//! `XDG_CONFIG_HOME`; both surfaces accept the same trade-off so a user
//! moving between shells does not see config silently load from one
//! directory and `~` expand to another.
//!
//! This is the *user home* directory. Code that wants the *config base*
//! (XDG / `%APPDATA%` / `$HOME/.config`) should use
//! `config::loader::global_config_path` instead, which layers platform
//! conventions on top of `home_dir` — so the HOME-vs-USERPROFILE precedence
//! defined here is the single source of truth shared by both surfaces.
//!
//! CL-3 (TASK-0752): consolidates two divergent inline implementations that
//! lived in `expand.rs` (HOME → USERPROFILE) and `config/loader.rs` (HOME-only
//! on Unix, USERPROFILE-only on Windows). Future Windows-native polish should
//! only need to update this single function.

use std::path::PathBuf;

/// Resolve the current user's home directory from the environment.
///
/// Returns `None` if no platform-appropriate variable is set:
/// - on Unix, when `HOME` is unset;
/// - on Windows, when neither `HOME` nor `USERPROFILE` is set.
pub fn home_dir() -> Option<PathBuf> {
    let from_home = std::env::var_os("HOME");
    #[cfg(not(unix))]
    {
        from_home
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    }
    #[cfg(unix)]
    {
        from_home.map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// ARCH-2 / TASK-0891: on non-Windows targets `USERPROFILE` must be
    /// ignored even when `HOME` is unset — the previous unconditional
    /// chain let a polluted `USERPROFILE` redirect every `~` expansion.
    #[cfg(unix)]
    #[test]
    #[serial]
    fn home_dir_ignores_userprofile_on_unix() {
        let saved_home = std::env::var_os("HOME");
        let saved_up = std::env::var_os("USERPROFILE");
        // SAFETY: serial-gated mutation of process env.
        unsafe {
            std::env::remove_var("HOME");
            std::env::set_var("USERPROFILE", "/should/not/be/used");
        }
        let resolved = home_dir();
        // SAFETY: restore.
        unsafe {
            match saved_home {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match saved_up {
                Some(v) => std::env::set_var("USERPROFILE", v),
                None => std::env::remove_var("USERPROFILE"),
            }
        }
        assert!(
            resolved.is_none(),
            "USERPROFILE must NOT be honored on Unix; got {resolved:?}"
        );
    }
}
