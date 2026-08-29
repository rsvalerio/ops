//! Permission-based test fixtures, shared by the module tests.
//!
//! Every helper here returns `None` when the current process can defeat the
//! permission it is trying to set — i.e. when the tests run as root, which is
//! the default in many CI containers. Callers treat `None` as "this hazard
//! cannot be simulated here" and skip, rather than asserting something the
//! environment cannot make true.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Create a git repository at `dir`.
///
/// Returns `false` when git is unusable in this environment, so the caller
/// skips rather than failing on something it is not testing.
pub fn git_init(dir: &Path) -> bool {
    git(dir, &["init", "-q"])
}

/// Stage `paths` in the repository at `dir`.
pub fn git_add(dir: &Path, paths: &[&Path]) -> bool {
    let mut args: Vec<&std::ffi::OsStr> = vec!["add".as_ref(), "--".as_ref()];
    args.extend(paths.iter().map(|p| p.as_os_str()));
    git_os(dir, &args)
}

/// Whether a usable `git` is on `PATH` at all.
///
/// [`is_inside_repo`] cannot distinguish "not a repository" from "git is
/// missing" — both are `false` — so fixtures that assert on a git-derived
/// outcome need this second guard before they run.
pub fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Whether `dir` is inside a git worktree. Used to skip the
/// "not a repository" fixtures when `TMPDIR` happens to live inside one.
pub fn is_inside_repo(dir: &Path) -> bool {
    git(dir, &["rev-parse", "--is-inside-work-tree"])
}

fn git(dir: &Path, args: &[&str]) -> bool {
    let args: Vec<&std::ffi::OsStr> = args.iter().map(AsRef::as_ref).collect();
    git_os(dir, &args)
}

fn git_os(dir: &Path, args: &[&std::ffi::OsStr]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        // Keep the fixture repo independent of the developer's global config
        // (templates, hooks, `core.excludesFile`).
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .args(args)
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Makes a directory unwritable for the lifetime of the guard.
pub struct ReadOnlyDir {
    path: PathBuf,
    original: std::fs::Permissions,
}

impl ReadOnlyDir {
    /// Returns `None` if the process can still create files inside `path`
    /// after the chmod (root ignores the mode bits).
    #[cfg(unix)]
    pub fn new(path: &Path) -> Option<Self> {
        use std::os::unix::fs::PermissionsExt;

        let original = std::fs::metadata(path).ok()?.permissions();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o555)).ok()?;
        let guard = Self {
            path: path.to_path_buf(),
            original,
        };

        let probe = path.join(".ops-permission-probe");
        if std::fs::write(&probe, b"").is_ok() {
            let _ = std::fs::remove_file(&probe);
            return None;
        }
        Some(guard)
    }

    #[cfg(not(unix))]
    pub fn new(_path: &Path) -> Option<Self> {
        None
    }
}

impl Drop for ReadOnlyDir {
    fn drop(&mut self) {
        // Restore, or the tempdir cannot be cleaned up.
        let _ = std::fs::set_permissions(&self.path, self.original.clone());
    }
}

/// Makes a file unreadable for the lifetime of the guard.
pub struct UnreadableFile {
    path: PathBuf,
    original: std::fs::Permissions,
}

impl UnreadableFile {
    /// Returns `None` if the process can still read `path` after the chmod.
    #[cfg(unix)]
    pub fn new(path: &Path) -> Option<Self> {
        use std::os::unix::fs::PermissionsExt;

        let original = std::fs::metadata(path).ok()?.permissions();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).ok()?;
        let guard = Self {
            path: path.to_path_buf(),
            original,
        };
        if std::fs::read(path).is_ok() {
            return None;
        }
        Some(guard)
    }

    #[cfg(not(unix))]
    pub fn new(_path: &Path) -> Option<Self> {
        None
    }
}

impl Drop for UnreadableFile {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.path, self.original.clone());
    }
}

/// Makes a directory unsearchable (and unreadable) for the lifetime of the
/// guard, so a walk of its parent hits a per-entry error.
pub struct UnsearchableDir {
    path: PathBuf,
    original: std::fs::Permissions,
}

impl UnsearchableDir {
    /// Returns `None` if the process can still list `path` after the chmod.
    #[cfg(unix)]
    pub fn new(path: &Path) -> Option<Self> {
        use std::os::unix::fs::PermissionsExt;

        let original = std::fs::metadata(path).ok()?.permissions();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).ok()?;
        let guard = Self {
            path: path.to_path_buf(),
            original,
        };
        if std::fs::read_dir(path).is_ok() {
            return None;
        }
        Some(guard)
    }

    #[cfg(not(unix))]
    pub fn new(_path: &Path) -> Option<Self> {
        None
    }
}

impl Drop for UnsearchableDir {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.path, self.original.clone());
    }
}
