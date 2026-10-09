//! Replace a file's contents without ever leaving it short.
//!
//! # Why not `fs::write`
//!
//! `std::fs::write` is `File::create` — `O_WRONLY|O_CREAT|O_TRUNC` — followed
//! by `write_all`. Between the truncate and the completed write the file on
//! disk is empty or partial, and the only copy of the original is a `Vec` in
//! this process. Ctrl-C on a pre-commit hook, an `ENOSPC`, an `EIO`, or a
//! quota refusal in that window leaves the user's source file truncated with
//! no backup and no rollback. On a tool wired into `ops verify` and a commit
//! hook, over the whole worktree, that is silent data loss.
//!
//! [`replace`] instead stages the new content in a temp file created in the
//! *same directory*, `fsync`s it, copies the original's ownership and mode
//! onto it, and `rename(2)`s it over the target. `rename` is atomic on POSIX:
//! a reader sees either the whole old file or the whole new one, never a short
//! one. The parent directory is `fsync`ed afterwards so the new directory
//! entry survives a crash.
//!
//! # What is re-checked before the rename
//!
//! The read side holds a symlink-refusing handle; the write side has only the
//! path, which it resolves a second time. Two checks keep that second
//! resolution honest:
//!
//! - **No directory component of the target is a symlink.** The read refused
//!   a symlink at every component, so one that appears afterwards is a swap,
//!   and staging or renaming through it would land outside the tree the run
//!   was pointed at. It is refused before the stage file is created.
//! - **The target is still the file that was read.** Immediately before the
//!   rename the target is `lstat`ed and its device, inode, length and
//!   modification time are compared with the metadata of the read handle. Any
//!   difference — an editor saving, another step rewriting, the file replaced
//!   by a symlink or deleted — refuses the rewrite, because the staged
//!   content was computed from bytes that are no longer the file's.
//!
//! Both are path-based checks, so the instants between each check and the
//! `rename(2)` it guards remain; they shrink the window from "the whole
//! read-fix-write cycle" to "one syscall gap", they do not close it.
//!
//! # The trade this makes
//!
//! `rename(2)` replaces the *directory entry*, so the target gets a **new
//! inode**. Two properties that truncate-in-place got for free are therefore
//! given up deliberately:
//!
//! - **Hard links are broken.** If the file had other names, they keep the old
//!   inode and the old content; only the path passed here sees the fix.
//! - **Open file descriptors keep reading the old inode.** A process holding
//!   the file open (an editor, a tail) does not observe the rewrite.
//! - **A killed run leaves stage files behind.** `NamedTempFile`'s `Drop`
//!   unlinks the stage on every ordinary error path, but `Drop` does not run
//!   when the process is killed — SIGKILL, or a SIGINT/SIGTERM with no
//!   handler, which is precisely the pre-commit-hook interruption this module
//!   exists for. Each file that was mid-write then keeps one
//!   [`STAGE_PREFIX`]-named sibling in the worktree (visible in `git status`
//!   until deleted). The residue is inert: discovery rejects the prefix in
//!   both walk and tracked modes, so a leftover is
//!   never walked, read, or rewritten as a candidate by a later run.
//!
//! All three are accepted. A whitespace fixer's failure mode has to be "did
//! nothing", never "emptied a source file", and hard-linked source files are
//! rare where interrupted hook runs are not. Mode, uid and gid *are*
//! preserved, so the visible attributes of the file do not change.

use std::fs::{File, Metadata};
use std::io::{self, Write};
use std::path::Path;

/// Stage-file name prefix used by [`replace`].
///
/// Discovery rejects file names starting with this prefix in both walk and
/// tracked modes, so a stage file left behind by a
/// killed run is never a candidate for a subsequent fixer run.
pub const STAGE_PREFIX: &str = ".ops-text-fixers.";

/// Atomically replace the contents of `path` with `contents`, preserving the
/// mode, uid and gid recorded in `original`.
///
/// `original` must be the metadata of the file being replaced, taken from the
/// handle it was read through: it is both the source of the preserved
/// attributes and the identity the target is compared against before the
/// rename.
///
/// # Errors
///
/// - [`io::ErrorKind::InvalidInput`] if a directory component of `path` is a
///   symlink.
/// - An error whose message is [`CHANGED_SINCE_READ`] if the target no longer
///   matches `original` (different device, inode, length or modification
///   time, or no longer a regular file); [`io::ErrorKind::NotFound`] if it is
///   gone.
/// - Any error from creating the temp file in `path`'s directory, writing it,
///   `fsync`ing it, or renaming it over `path`.
///
/// On every error path the target is left exactly as it was and the temp file
/// is unlinked.
pub fn replace(path: &Path, contents: &[u8], original: &Metadata) -> io::Result<()> {
    // A bare filename has an empty parent; stage alongside it in the cwd.
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    refuse_symlinked_directory(parent)?;

    // Same directory, so the rename is within one filesystem (a cross-device
    // rename fails with EXDEV) and a randomised name so two concurrent fixer
    // runs over a shared worktree stage into disjoint paths.
    let mut tmp = tempfile::Builder::new()
        .prefix(STAGE_PREFIX)
        .tempfile_in(parent)?;

    tmp.write_all(contents)?;
    // The content must be durable before the rename, or a crash can leave the
    // directory entry pointing at an inode whose data never reached disk.
    tmp.as_file().sync_data()?;
    preserve_attributes(tmp.as_file(), original)?;

    // Last, so the gap between this comparison and the rename is as short as
    // it can be made without a directory handle.
    ensure_unchanged(path, original)?;

    // `persist` consumes the temp file and renames it over `path`. On failure
    // the inner value falls back into `Drop`, unlinking the stage; `path` is
    // untouched.
    tmp.persist(path).map_err(|e| e.error)?;
    sync_parent_dir(parent);
    Ok(())
}

/// Message of the error [`replace`] returns when the target changed after it
/// was read.
pub const CHANGED_SINCE_READ: &str = "changed on disk since it was read; left untouched";

/// Refuse `dir` if it, or any directory above it in the path as given, is a
/// symlink.
///
/// The check covers the path as spelled, the same scope the read side's
/// component walk has, so a caller that canonicalizes its root once is not
/// refused for a symlink above that root.
fn refuse_symlinked_directory(dir: &Path) -> io::Result<()> {
    for component in dir.ancestors().filter(|a| !a.as_os_str().is_empty()) {
        if std::fs::symlink_metadata(component)?
            .file_type()
            .is_symlink()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "refusing to write through symlinked directory {:?}",
                    component.display()
                ),
            ));
        }
    }
    Ok(())
}

/// Fail unless `path` still names the regular file `original` was taken from.
///
/// `symlink_metadata` rather than `metadata`: a symlink swapped in for the
/// file must be seen as a symlink, not as whatever it points at.
fn ensure_unchanged(path: &Path, original: &Metadata) -> io::Result<()> {
    let current = std::fs::symlink_metadata(path)?;
    if current.file_type().is_file() && is_same_file(&current, original) {
        return Ok(());
    }
    Err(io::Error::other(CHANGED_SINCE_READ))
}

/// Whether two metadata snapshots describe the same, unmodified file.
///
/// Device and inode are compared on Unix only; elsewhere length and
/// modification time are the available evidence.
fn is_same_file(current: &Metadata, original: &Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if current.dev() != original.dev() || current.ino() != original.ino() {
            return false;
        }
    }
    current.len() == original.len() && current.modified().ok() == original.modified().ok()
}

/// Copy mode, uid and gid from the replaced file onto the staged one.
///
/// A `NamedTempFile` is created 0600 and owned by the current user, so
/// without this a 0644 file would come back private and a root-owned file
/// (fixed under `sudo`) would change hands.
fn preserve_attributes(staged: &File, original: &Metadata) -> io::Result<()> {
    staged.set_permissions(original.permissions())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // Only a privileged process may hand a file to another uid. When the
        // fixer runs unprivileged over its own files the ids already match and
        // this is a no-op; when they do not match and the call is refused,
        // refusing the *whole fix* over ownership would be worse than a
        // rewrite that keeps the running user's ownership, which is what
        // `fs::write` did too.
        if let Err(e) =
            std::os::unix::fs::fchown(staged, Some(original.uid()), Some(original.gid()))
        {
            if e.kind() != io::ErrorKind::PermissionDenied {
                return Err(e);
            }
        }
    }
    Ok(())
}

/// Persist the parent directory entry created by the rename.
///
/// Unix-only; Windows does not require the equivalent, and `open(parent)`
/// would fail there anyway. Errors are swallowed because the rewrite has
/// already succeeded and some filesystems do not support directory `fsync` —
/// failing the fix over a durability hint would regress the success path.
fn sync_parent_dir(parent: &Path) {
    #[cfg(not(unix))]
    let _ = parent;
    #[cfg(unix)]
    if let Ok(dir) = File::open(parent) {
        let _ = dir.sync_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tempdir path with its own symlinked prefix resolved (macOS: `/var` ->
    /// `/private/var`), so a refusal in a test is about the fixture.
    fn canon(dir: &tempfile::TempDir) -> std::path::PathBuf {
        dir.path().canonicalize().unwrap()
    }

    fn entry_names(dir: &Path) -> Vec<std::ffi::OsString> {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_file_edited_since_the_read_is_left_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = canon(&dir).join("a.txt");
        std::fs::write(&path, b"old  \n").unwrap();
        let md = std::fs::metadata(&path).unwrap();

        std::fs::write(&path, b"a concurrent edit\n").unwrap();

        let err = replace(&path, b"old\n", &md).unwrap_err();
        assert_eq!(err.to_string(), CHANGED_SINCE_READ);
        assert_eq!(std::fs::read(&path).unwrap(), b"a concurrent edit\n");
        assert_eq!(entry_names(&canon(&dir)), ["a.txt"], "stage unlinked");
    }

    /// Same inode and same length: the modification time alone gives it away.
    #[test]
    fn a_same_length_edit_is_detected_by_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let path = canon(&dir).join("a.txt");
        std::fs::write(&path, b"aaaa\n").unwrap();
        let md = std::fs::metadata(&path).unwrap();

        std::fs::write(&path, b"bbbb\n").unwrap();
        let later = md.modified().unwrap() + std::time::Duration::from_secs(5);
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(later)
            .unwrap();

        let err = replace(&path, b"cccc\n", &md).unwrap_err();
        assert_eq!(err.to_string(), CHANGED_SINCE_READ);
        assert_eq!(std::fs::read(&path).unwrap(), b"bbbb\n");
    }

    #[test]
    fn a_file_replaced_by_another_inode_is_left_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let root = canon(&dir);
        let path = root.join("a.txt");
        std::fs::write(&path, b"old\n").unwrap();
        let md = std::fs::metadata(&path).unwrap();

        // An editor's save-by-rename: same name, same length, new inode.
        std::fs::write(root.join("b.txt"), b"new\n").unwrap();
        std::fs::rename(root.join("b.txt"), &path).unwrap();

        let err = replace(&path, b"fix\n", &md).unwrap_err();
        assert_eq!(err.to_string(), CHANGED_SINCE_READ);
        assert_eq!(std::fs::read(&path).unwrap(), b"new\n");
    }

    #[test]
    fn a_file_deleted_since_the_read_is_not_recreated() {
        let dir = tempfile::tempdir().unwrap();
        let path = canon(&dir).join("a.txt");
        std::fs::write(&path, b"old\n").unwrap();
        let md = std::fs::metadata(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        let err = replace(&path, b"new\n", &md).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(entry_names(&canon(&dir)).is_empty());
    }

    /// The parent directory is swapped for a symlink to a directory holding
    /// the very same inode, so the identity check alone would pass: it is the
    /// directory check that has to refuse.
    #[cfg(unix)]
    #[test]
    fn a_parent_swapped_for_a_symlink_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let root = canon(&dir);
        let path = root.join("sub").join("a.txt");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(&path, b"old\n").unwrap();
        let md = std::fs::metadata(&path).unwrap();

        std::fs::rename(root.join("sub"), root.join("elsewhere")).unwrap();
        std::os::unix::fs::symlink(root.join("elsewhere"), root.join("sub")).unwrap();

        let err = replace(&path, b"new\n", &md).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("symlinked directory"),
            "unexpected error: {err}"
        );
        assert_eq!(
            std::fs::read(root.join("elsewhere").join("a.txt")).unwrap(),
            b"old\n"
        );
        assert_eq!(
            entry_names(&root.join("elsewhere")),
            ["a.txt"],
            "nothing may be staged through the symlink"
        );
    }

    #[test]
    fn replaces_content_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let path = canon(&dir).join("a.txt");
        std::fs::write(&path, b"old\n").unwrap();
        let md = std::fs::metadata(&path).unwrap();

        replace(&path, b"new\n", &md).unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"new\n");
    }

    #[test]
    fn leaves_no_temp_files_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = canon(&dir).join("a.txt");
        std::fs::write(&path, b"old\n").unwrap();
        let md = std::fs::metadata(&path).unwrap();

        replace(&path, b"new\n", &md).unwrap();

        assert_eq!(entry_names(&canon(&dir)), ["a.txt"]);
    }

    #[cfg(unix)]
    #[test]
    fn preserves_mode() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = canon(&dir).join("a.txt");
        std::fs::write(&path, b"old\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        let md = std::fs::metadata(&path).unwrap();

        replace(&path, b"new\n", &md).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o640, "mode must survive the rename");
    }

    #[test]
    fn a_failed_replace_leaves_the_original_intact() {
        let dir = tempfile::tempdir().unwrap();
        let path = canon(&dir).join("sub").join("a.txt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"original\n").unwrap();
        let md = std::fs::metadata(&path).unwrap();

        let Some(guard) = crate::test_support::ReadOnlyDir::new(path.parent().unwrap()) else {
            crate::test_support::skip_precondition(
                "read-only directory fixture",
                "running as root or the chmod did not deny; rollback assertions did not run",
            );
            return;
        };

        let err = replace(&path, b"clobbered\n", &md).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        drop(guard);

        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"original\n",
            "a write that failed part-way must not have touched the target"
        );
    }
}
