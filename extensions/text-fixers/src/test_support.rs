//! Permission-based test fixtures, shared by the module tests.
//!
//! Every helper here returns `None` when the current process can defeat the
//! permission it is trying to set — i.e. when the tests run as root, which is
//! the default in many CI containers. Callers treat `None` as "this hazard
//! cannot be simulated here" and skip, rather than asserting something the
//! environment cannot make true.
//!
//! No skip here is silent. Bail-outs surface a `skip:` line via
//! [`skip_precondition`] (re-exported below), and the git helpers distinguish
//! "git is absent here" (surfaced skip) from "git ran and refused" (panic — a
//! broken fixture, not a missing capability), so a green run cannot hide that
//! its safety assertions never executed.

use std::path::{Path, PathBuf};

use ops_core::test_utils::{git_fixture, git_fixture_os, GitFixtureError};

pub use ops_core::test_utils::skip_precondition;

/// Create a git repository at `dir`.
///
/// `false` means exactly one thing — this environment
/// has no git binary — and the skip has already been surfaced on stderr, so
/// a caller's `return` is visible in the test output instead of a vacuous
/// pass. A git that *runs* and fails panics here.
pub fn git_init(dir: &Path) -> bool {
    fixture("git fixture", dir, &["init", "-q"])
}

/// Stage `paths` in the repository at `dir`.
///
/// Same contract as [`git_init`]; on success there is nothing to assert —
/// staging either worked or this helper already failed the test.
pub fn git_add(dir: &Path, paths: &[&Path]) {
    let mut args: Vec<&std::ffi::OsStr> = vec!["add".as_ref(), "--".as_ref()];
    args.extend(paths.iter().map(|p| p.as_os_str()));
    fixture_os("git add fixture", dir, &args);
}

/// Whether a usable `git` is on `PATH` at all.
///
/// [`is_inside_repo`] cannot distinguish "not a repository" from "git is
/// missing" — both are `false` — so fixtures that assert on a git-derived
/// outcome need this second guard before they run. `false` is returned only
/// for an absent binary (surfaced as a skip); a `git --version` that runs and
/// fails panics.
pub fn git_available() -> bool {
    fixture("git availability probe", Path::new("."), &["--version"])
}

/// Whether `dir` is inside a git worktree. Used to skip the
/// "not a repository" fixtures when `TMPDIR` happens to live inside one.
///
/// A non-zero `rev-parse` is its expected "not a repository" answer, so —
/// unlike the mutating helpers — a refused command is `false`, not a panic;
/// callers that need the distinction guard with [`git_available`] first.
pub fn is_inside_repo(dir: &Path) -> bool {
    git_fixture(dir, &["rev-parse", "--is-inside-work-tree"]).is_ok()
}

fn fixture(what: &str, dir: &Path, args: &[&str]) -> bool {
    match git_fixture(dir, args) {
        Ok(()) => true,
        Err(GitFixtureError::BinaryAbsent) => {
            skip_precondition(what, "git is not on PATH; git-mode assertions did not run");
            false
        }
        Err(e @ GitFixtureError::CommandFailed { .. }) => {
            panic!("{what} broke: {e}");
        }
    }
}

fn fixture_os(what: &str, dir: &Path, args: &[&std::ffi::OsStr]) {
    match git_fixture_os(dir, args) {
        Ok(()) => {}
        Err(GitFixtureError::BinaryAbsent) => {
            skip_precondition(what, "git is not on PATH; git-mode assertions did not run");
        }
        Err(e @ GitFixtureError::CommandFailed { .. }) => {
            panic!("{what} broke: {e}");
        }
    }
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
