//! `$PATH` walking, executable detection, and index cache.

use std::collections::HashSet;
use std::ffi::OsString;

/// Cached index of executable basenames found on `$PATH`.
///
/// PERF-3 / TASK-1046: `collect_tools` previously fell through to
/// [`check_binary_installed`] for every Cargo-source tool that did not appear
/// in `cargo --list` (any tool installed standalone via `cargo install`,
/// e.g. `tokei`, `bacon`). Each fallback re-walked the entire `$PATH`.
/// Capturing the index once amortises the walk into a single pass.
///
/// CONC-7 / TASK-1249: Windows filesystems are case-insensitive but
/// `OsString` equality is case-sensitive. The index normalises basenames
/// to lowercase under `cfg(windows)` at both insert and lookup time.
pub type PathIndex = HashSet<OsString>;

/// CONC-7 / TASK-1249: normalise an `OsString` basename to the index key
/// form. Lowercase on Windows; verbatim on Unix.
pub(crate) fn index_key(name: OsString) -> OsString {
    if cfg!(windows) {
        OsString::from(name.to_string_lossy().to_lowercase())
    } else {
        name
    }
}

/// Build a one-shot index of executable basenames present on `$PATH`.
pub fn capture_path_index() -> Option<PathIndex> {
    let path = std::env::var_os("PATH")?;
    Some(capture_path_index_from(&path))
}

pub(crate) fn capture_path_index_from(path_var: &std::ffi::OsStr) -> PathIndex {
    let mut set: PathIndex = HashSet::new();
    for dir in std::env::split_paths(path_var) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!(
                        path = %dir.display(),
                        error = %e,
                        "PATH entry unreadable while building path index; skipping"
                    );
                }
                continue;
            }
        };
        for entry in entries.flatten() {
            let candidate = entry.path();
            if matches!(check_executable(&candidate), ExecCheck::Yes) {
                set.insert(index_key(entry.file_name()));
            }
        }
    }
    set
}

/// Look up `name` in a precomputed [`PathIndex`].
pub(crate) fn is_in_path_index(index: &PathIndex, name: &str) -> bool {
    if index.contains(&index_key(OsString::from(name))) {
        return true;
    }
    if cfg!(windows) {
        for ext in pathext_suffixes() {
            let mut candidate = OsString::from(name);
            candidate.push(&ext);
            if index.contains(&index_key(candidate)) {
                return true;
            }
        }
    }
    false
}

/// PERF-3 / TASK-1046: variant that consults a precomputed [`PathIndex`]
/// when supplied, falling back to the per-call `$PATH` walk when `index`
/// is `None`.
pub fn check_binary_installed_with(name: &str, index: Option<&PathIndex>) -> bool {
    match index {
        Some(idx) => is_in_path_index(idx, name),
        None => check_binary_installed(name),
    }
}

/// SEC-13: walk `PATH` directly instead of shelling out to `which`.
pub fn check_binary_installed(name: &str) -> bool {
    find_on_path(name).is_some()
}

pub(crate) fn find_on_path(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    find_on_path_in(name, &path)
}

pub(crate) fn find_on_path_in(
    name: &str,
    path_var: &std::ffi::OsStr,
) -> Option<std::path::PathBuf> {
    for dir in std::env::split_paths(path_var) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(name);
        match check_executable(&candidate) {
            ExecCheck::Yes => return Some(candidate),
            ExecCheck::BrokenSymlink => {
                tracing::warn!(
                    path = %candidate.display(),
                    "PATH entry is a broken symlink; skipping"
                );
            }
            ExecCheck::NotExec | ExecCheck::Missing => {}
        }
        if cfg!(windows) {
            for ext in pathext_suffixes() {
                let mut with_ext = candidate.clone().into_os_string();
                with_ext.push(&ext);
                let p = std::path::PathBuf::from(with_ext);
                if matches!(check_executable(&p), ExecCheck::Yes) {
                    return Some(p);
                }
            }
        }
    }
    None
}

#[cfg(windows)]
fn pathext_suffixes() -> Vec<std::ffi::OsString> {
    let raw = std::env::var_os("PATHEXT")
        .unwrap_or_else(|| std::ffi::OsString::from(".COM;.EXE;.BAT;.CMD"));
    std::env::split_paths(&raw)
        .map(std::path::PathBuf::into_os_string)
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(not(windows))]
fn pathext_suffixes() -> Vec<std::ffi::OsString> {
    Vec::new()
}

/// ERR-1 / TASK-0607: distinguish "not executable", "broken symlink", and
/// "missing" so the PATH walk keeps looking while surfacing the
/// broken-symlink case to operators.
enum ExecCheck {
    Yes,
    NotExec,
    BrokenSymlink,
    Missing,
}

#[cfg(unix)]
fn check_executable(path: &std::path::Path) -> ExecCheck {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(m) if m.is_file() && m.permissions().mode() & 0o111 != 0 => ExecCheck::Yes,
        Ok(_) => ExecCheck::NotExec,
        Err(_) => match std::fs::symlink_metadata(path) {
            Ok(m) if m.file_type().is_symlink() => ExecCheck::BrokenSymlink,
            _ => ExecCheck::Missing,
        },
    }
}

#[cfg(not(unix))]
fn check_executable(path: &std::path::Path) -> ExecCheck {
    match std::fs::metadata(path) {
        Ok(m) if m.is_file() => ExecCheck::Yes,
        Ok(_) => ExecCheck::NotExec,
        Err(_) => match std::fs::symlink_metadata(path) {
            Ok(m) if m.file_type().is_symlink() => ExecCheck::BrokenSymlink,
            _ => ExecCheck::Missing,
        },
    }
}

#[cfg(all(test, windows))]
mod path_index_case_tests {
    use super::*;

    /// CONC-7 / TASK-1249: Windows lookup is case-insensitive in both directions.
    #[test]
    fn windows_lookup_matches_mixed_case_basename() {
        let mut idx: PathIndex = HashSet::new();
        idx.insert(index_key(OsString::from("Tokei.EXE")));
        assert!(
            is_in_path_index(&idx, "tokei"),
            "Windows lookup must be case-insensitive in both directions"
        );
        idx.insert(index_key(OsString::from("ripgrep.exe")));
        assert!(is_in_path_index(&idx, "RipGrep"));
    }
}

#[cfg(all(test, unix))]
mod path_index_unix_tests {
    use super::*;

    /// CONC-7 / TASK-1249: Unix lookup remains case-sensitive.
    #[test]
    fn unix_lookup_remains_case_sensitive() {
        let mut idx: PathIndex = HashSet::new();
        idx.insert(index_key(OsString::from("Tokei")));
        assert!(
            !is_in_path_index(&idx, "tokei"),
            "Unix lookup must stay case-sensitive: `tokei` and `Tokei` are distinct"
        );
        assert!(is_in_path_index(&idx, "Tokei"));
    }
}
