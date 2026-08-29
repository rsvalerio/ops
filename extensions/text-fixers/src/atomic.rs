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
//!
//! Both are accepted. A whitespace fixer's failure mode has to be "did
//! nothing", never "emptied a source file", and hard-linked source files are
//! rare where interrupted hook runs are not. Mode, uid and gid *are*
//! preserved, so the visible attributes of the file do not change.

use std::fs::{File, Metadata};
use std::io::{self, Write};
use std::path::Path;

/// Atomically replace the contents of `path` with `contents`, preserving the
/// mode, uid and gid recorded in `original`.
///
/// `original` must be the metadata of the file being replaced, taken from the
/// handle it was read through.
///
/// # Errors
///
/// If the temp file cannot be created in `path`'s directory, written,
/// `fsync`ed, or renamed over `path`. On every error path the target is left
/// exactly as it was and the temp file is unlinked.
pub fn replace(path: &Path, contents: &[u8], original: &Metadata) -> io::Result<()> {
    // A bare filename has an empty parent; stage alongside it in the cwd.
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    // Same directory, so the rename is within one filesystem (a cross-device
    // rename fails with EXDEV) and a randomised name so two concurrent fixer
    // runs over a shared worktree stage into disjoint paths.
    let mut tmp = tempfile::Builder::new()
        .prefix(".ops-text-fixers.")
        .tempfile_in(parent)?;

    tmp.write_all(contents)?;
    // The content must be durable before the rename, or a crash can leave the
    // directory entry pointing at an inode whose data never reached disk.
    tmp.as_file().sync_data()?;
    preserve_attributes(tmp.as_file(), original)?;

    // `persist` consumes the temp file and renames it over `path`. On failure
    // the inner value falls back into `Drop`, unlinking the stage; `path` is
    // untouched.
    tmp.persist(path).map_err(|e| e.error)?;
    sync_parent_dir(parent);
    Ok(())
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

    #[test]
    fn replaces_content_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"old\n").unwrap();
        let md = std::fs::metadata(&path).unwrap();

        replace(&path, b"new\n", &md).unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"new\n");
    }

    #[test]
    fn leaves_no_temp_files_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"old\n").unwrap();
        let md = std::fs::metadata(&path).unwrap();

        replace(&path, b"new\n", &md).unwrap();

        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("a.txt")]);
    }

    #[cfg(unix)]
    #[test]
    fn preserves_mode() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
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
        let path = dir.path().join("sub").join("a.txt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"original\n").unwrap();
        let md = std::fs::metadata(&path).unwrap();

        let Some(guard) = crate::test_support::ReadOnlyDir::new(path.parent().unwrap()) else {
            return; // running as root: the directory is writable regardless.
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
