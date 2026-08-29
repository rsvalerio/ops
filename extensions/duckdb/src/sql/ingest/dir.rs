//! Ingest directory layout, hardening, checksums, and external-error helpers.

use crate::{DbError, DbResult, DuckDb};
use std::path::{Path, PathBuf};

/// Compute the ingest data directory from a DB path (appends `.ingest`).
///
/// READ-5 / TASK-1867: `DuckDb::open_in_memory` stores the `DuckDB`
/// connection string `:memory:` as its path. Appending `.ingest` to that
/// sentinel yielded the *relative* path `:memory:.ingest`, which the ingest
/// pipeline then created — with staged JSON inside it — in whatever the
/// process working directory happened to be: the user's project root under
/// `ops`, or the crate directory under `cargo test` (where the debris was
/// once committed to this repository). An in-memory handle has no staging
/// area, so it is rejected instead of silently redirected.
///
/// # Errors
///
/// [`DbError::NotFileBacked`] if `db_path` is the in-memory sentinel.
pub fn data_dir_for_db(db_path: &Path) -> DbResult<PathBuf> {
    if db_path == Path::new(crate::connection::IN_MEMORY_PATH) {
        return Err(DbError::NotFileBacked(db_path.to_path_buf()));
    }
    let mut path = db_path.as_os_str().to_os_string();
    path.push(".ingest");
    Ok(PathBuf::from(path))
}

/// Create the ingest data directory with restrictive permissions.
///
/// SEC-25 / TASK-0787: the ingest dir holds workspace-root sidecars and
/// JSON staging files that the database trusts on load. On Unix we create
/// it with mode 0o700 (and re-stamp the mode when the dir pre-exists with
/// a more permissive default umask) so a co-tenant on a multi-user system
/// cannot tamper with staged data between collect and load. Non-Unix
/// platforms have no portable mode to stamp, so they get the rejection half
/// only: a pre-existing symlink or reparse point at `data_dir` is refused
/// there too (see [`reject_untrusted_ingest_dir`]), and a fresh dir is
/// created with `create_dir_all` at the platform default.
///
/// SEC-25 / TASK-1000: only the **leaf** ingest dir is hardened to 0o700.
/// `DirBuilder::recursive(true).mode(0o700)` would also stamp every
/// intermediate parent created during the call (e.g. `target/`,
/// `target/ops/`) with 0o700, breaking cargo / build-system convention
/// (target/ is canonically 0o755) and producing an asymmetry between
/// fresh workspaces and ones where `target/` already exists. Create the
/// parents first at the platform-default umask, then build the leaf
/// alone with the restrictive mode.
///
/// SEC-25 / TASK-1857: a pre-existing `data_dir` is *not* trusted. `mkdir`
/// reporting `AlreadyExists` says only that the name is taken — it may be a
/// symlink an attacker planted, in which case a path-based `chmod` would
/// follow it and stamp 0o700 on the attacker's chosen target while every
/// subsequent staged write landed inside it. We therefore `lstat` the path,
/// reject anything that is not a real directory, and then do the mode stamp
/// through an **open handle** whose `(dev, ino)` is checked against the
/// `lstat` result, so the check and the act refer to the same inode. The
/// intermediate parents are created with `create_dir_all` at the platform
/// default umask (TASK-1000) and are deliberately *not* hardened; the
/// co-tenant guarantee this function makes is about the leaf ingest dir
/// only.
///
/// # SEC / TASK-2039: closing the verify-then-write TOCTOU window
///
/// Everything above verifies the ingest dir through an **open handle** and
/// then drops it. `provide_via_ingestor` afterwards hands the plain `&Path`
/// to [`crate::Ingestor::collect`] and [`crate::Ingestor::load`], which
/// reopen `data_dir` by path, and `sidecar.rs` joins onto it by path too. A
/// principal who can create names in the ingest dir's **parent** can
/// therefore swap the verified directory for a symlink between the check and
/// each staged write, and the JSON the database later trusts on load lands
/// wherever they point.
///
/// Of the two options weighed on TASK-2039 — threading a verified `Dir`-like
/// handle (`cap-std` / `*at` syscalls) through the `Ingestor` trait and
/// `sidecar.rs`, versus removing the swap *capability* — this takes the
/// second: [`harden_ingest_parent`] makes the staging parent unwritable to
/// every principal but its owner, so no one else can create or rename a name
/// inside it and the window has nothing to exploit. That keeps the
/// `Ingestor` trait's `&Path` signature and every implementation unchanged.
///
/// The parent is tightened by clearing the group/other **write** bits only
/// (`0o775` → `0o755`), not stamped to `0o700`: `target/ops` is conventionally
/// readable, and TASK-1000's rule that intermediate parents keep the platform
/// default still holds for everything above the immediate parent. A parent
/// that is shared-writable but **sticky** (`/tmp`-style) is accepted as is —
/// the sticky bit already forbids other principals renaming or deleting a
/// name they do not own, which is exactly the swap being defended against.
/// If the write bits cannot be cleared (we do not own the directory), staging
/// is refused rather than performed into a directory another principal
/// controls.
///
/// Non-Unix keeps the rejection half only, as above: there is no portable
/// mode to inspect or stamp.
pub(super) fn create_ingest_dir(data_dir: &Path) -> std::io::Result<()> {
    if let Some(parent) = data_dir.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
            #[cfg(unix)]
            harden_ingest_parent(parent)?;
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        match std::fs::DirBuilder::new()
            .recursive(false)
            .mode(0o700)
            .create(data_dir)
        {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
        harden_existing_ingest_dir(data_dir)
    }
    #[cfg(not(unix))]
    {
        // SEC-25 / TASK-1857: the "a pre-existing `data_dir` is not trusted"
        // rule is not Unix-specific. `create_dir_all` succeeds silently when
        // the name is already taken by a symlink or a directory junction, so
        // without this check every staged write would land wherever the
        // reparse point points. There is no portable mode to stamp, so this
        // branch keeps the *rejection* half of the Unix behaviour and drops
        // only the `fchmod`.
        match reject_untrusted_ingest_dir(data_dir) {
            Ok(Some(_)) => Ok(()),
            Ok(None) => std::fs::create_dir_all(data_dir),
            Err(e) => Err(e),
        }
    }
}

/// `lstat` `data_dir` and refuse anything that is not a real directory.
///
/// Returns `Ok(Some(lstat))` when the path exists and is a plain directory,
/// `Ok(None)` when nothing is there, and an error when the name is taken by
/// a symlink (or, on Windows, any other reparse point — `FileType::is_symlink`
/// covers junctions too), or by a non-directory.
///
/// SEC-25 / TASK-1857: `mkdir` reporting `AlreadyExists` says only that the
/// name is taken. Following whatever is there is the whole attack.
fn reject_untrusted_ingest_dir(data_dir: &Path) -> std::io::Result<Option<std::fs::Metadata>> {
    use std::io::{Error, ErrorKind};

    let lstat = match std::fs::symlink_metadata(data_dir) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let file_type = lstat.file_type();
    if file_type.is_symlink() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "ingest dir {} is a symlink; refusing to stage data through it",
                data_dir.display()
            ),
        ));
    }
    if !file_type.is_dir() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "ingest dir {} exists but is not a directory",
                data_dir.display()
            ),
        ));
    }
    Ok(Some(lstat))
}

/// SEC / TASK-2039: remove the *capability* to swap the verified ingest dir
/// for a symlink, by making its parent directory writable only by its owner.
///
/// See the TASK-2039 section on [`create_ingest_dir`] for why this is done
/// instead of threading a directory handle through the [`crate::Ingestor`]
/// trait. Returns:
///
/// * `Ok(())` when no other principal can create names in `parent` — either
///   the group/other write bits were already clear, or the directory is
///   sticky (a name there cannot be renamed or unlinked by anyone but its
///   owner), or we cleared the bits ourselves.
/// * `Err` when the bits are set, the directory is not sticky, and the
///   `fchmod` fails — typically because the directory belongs to someone
///   else, which is precisely the situation in which staging into it is
///   unsafe.
///
/// The mode is applied through an open handle (`fchmod`), and the handle is
/// confirmed to be a directory first, so a symlink at `parent` cannot have
/// its target chmodded — the same discipline as
/// [`harden_existing_ingest_dir`].
#[cfg(unix)]
fn harden_ingest_parent(parent: &Path) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    use std::os::unix::fs::PermissionsExt;

    /// Write permission for group and other: the ability to create, rename,
    /// or unlink names inside the directory.
    const SHARED_WRITE: u32 = 0o022;
    /// The sticky bit (`S_ISVTX`), which restricts renaming and unlinking
    /// inside a shared-writable directory to the entry's own owner.
    const STICKY: u32 = 0o1000;

    let handle = std::fs::File::open(parent)?;
    let meta = handle.metadata()?;
    if !meta.is_dir() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "ingest staging parent {} is not a directory",
                parent.display()
            ),
        ));
    }
    // `PermissionsExt::mode` returns the raw `st_mode`, file-type bits and
    // all; keep only the permission + set-id/sticky bits `fchmod` accepts.
    let mode = meta.permissions().mode() & 0o7777;
    if mode & SHARED_WRITE == 0 {
        return Ok(());
    }
    if mode & STICKY != 0 {
        tracing::debug!(
            parent = ?parent.display(),
            mode = format!("{mode:o}"),
            "SEC / TASK-2039: ingest staging parent is shared-writable but sticky; names cannot be swapped by other principals"
        );
        return Ok(());
    }
    handle
        .set_permissions(std::fs::Permissions::from_mode(mode & !SHARED_WRITE))
        .map_err(|e| {
            Error::new(
                e.kind(),
                format!(
                    "ingest staging parent {} is writable by other local principals (mode {mode:o}) and its permissions could not be tightened: {e}",
                    parent.display(),
                ),
            )
        })
}

/// Stamp `0o700` on an ingest dir that already exists on disk, refusing to
/// act on anything that is not a real directory.
///
/// SEC-25 / TASK-1857: `std::fs::set_permissions` is path-based and follows
/// symlinks, so it cannot be used here — a planted symlink would have its
/// *target* chmodded. Instead we `lstat` the path, reject symlinks and
/// non-directories outright, then open a handle and confirm it resolves to
/// the very inode we inspected before applying the mode through that handle
/// (`File::set_permissions` is `fchmod`, not `chmod`).
#[cfg(unix)]
fn harden_existing_ingest_dir(data_dir: &Path) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    // Shared with the non-Unix branch so the two platforms cannot drift on
    // what counts as an untrusted pre-existing ingest dir.
    let Some(lstat) = reject_untrusted_ingest_dir(data_dir)? else {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!(
                "ingest dir {} vanished before it could be hardened",
                data_dir.display()
            ),
        ));
    };

    let handle = std::fs::File::open(data_dir)?;
    let opened = handle.metadata()?;
    if !opened.is_dir() || opened.dev() != lstat.dev() || opened.ino() != lstat.ino() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "ingest dir {} changed identity between inspection and open",
                data_dir.display()
            ),
        ));
    }

    handle.set_permissions(std::fs::Permissions::from_mode(0o700))
}

/// Default DB path for a workspace root (using default `DataConfig`).
#[must_use]
pub fn default_db_path(workspace_root: &Path) -> PathBuf {
    DuckDb::resolve_path(&ops_core::config::DataConfig::default(), workspace_root)
}

/// Convert a non-IO external error into [`DbError::External`].
///
/// Callers that return `anyhow::Error` (`collect_tokei`, `collect_coverage`,
/// `check_metadata_output`, etc.) should use this instead of the old `io_err`
/// which misleadingly wrapped them as `DbError::Io`.
///
/// SEC-21 (TASK-0862): Display renders via the alternate `{:#}` flag so
/// `anyhow::Context` chains continue to surface end-to-end.
///
/// ERR-2 / TASK-1209: passes the underlying `anyhow::Error` through as
/// `#[source]` instead of flattening it via `format!`, so consumers walking
/// `Error::source()` recover the cause graph (e.g. typed retry decisions).
#[must_use]
pub const fn external_err(e: anyhow::Error) -> DbError {
    DbError::External(e)
}

/// Compute SHA-256 checksum of a file, returning hex string.
///
/// Streams the file in 64 KiB chunks so multi-megabyte ingests (coverage,
/// tokei) do not allocate a full file-sized buffer (PERF-1).
///
/// # Errors
///
/// [`DbError::Io`] if `path` cannot be opened or read.
pub fn checksum_file(path: &Path) -> DbResult<String> {
    use sha2::{Digest, Sha256};
    use std::io::{BufReader, Read};
    let file = std::fs::File::open(path).map_err(DbError::Io)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf).map_err(DbError::Io)?;
        if n == 0 {
            break;
        }
        // A `Read` impl never reports more bytes than the buffer holds; surface a
        // violation as an I/O error instead of panicking on the slice.
        let chunk = buf
            .get(..n)
            .ok_or_else(|| std::io::Error::other("read reported more bytes than the buffer holds"))
            .map_err(DbError::Io)?;
        hasher.update(chunk);
    }
    let digest = hasher.finalize();
    Ok(hex::encode(digest.as_slice()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// SEC-25 / TASK-0787: ingest dir must be 0o700 on Unix on both fresh
    /// create and pre-existing dir paths.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_uses_restricted_mode_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("data.duckdb.ingest");
        create_ingest_dir(&dir).expect("create");
        let mode = std::fs::metadata(&dir).expect("meta").permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o700,
            "fresh-created ingest dir must be 0o700; got {:o}",
            mode & 0o777,
        );
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("relax");
        create_ingest_dir(&dir).expect("recreate");
        let mode = std::fs::metadata(&dir).expect("meta").permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o700,
            "pre-existing ingest dir must be re-stamped to 0o700; got {:o}",
            mode & 0o777,
        );
    }

    /// SEC-25 / TASK-1000: only the leaf ingest dir is 0o700.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_does_not_lock_down_intermediate_parents() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let leaf = tmp.path().join("a/b/data.duckdb.ingest");
        create_ingest_dir(&leaf).expect("create");

        let leaf_mode = std::fs::metadata(&leaf)
            .expect("leaf meta")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(leaf_mode, 0o700, "leaf must be 0o700; got {leaf_mode:o}");

        for parent in [tmp.path().join("a"), tmp.path().join("a/b")] {
            let mode = std::fs::metadata(&parent)
                .expect("parent meta")
                .permissions()
                .mode()
                & 0o777;
            assert_ne!(
                mode,
                0o700,
                "intermediate parent {} was stamped 0o700; expected umask default",
                parent.display()
            );
        }
    }

    /// SEC / TASK-2039: the swap this defends against needs the ability to
    /// create or rename names in the ingest dir's parent. After
    /// `create_ingest_dir`, a shared-writable parent must no longer grant it,
    /// so a symlink cannot be swapped in after verification and no staged
    /// write can be redirected. Starting the parent at 0o777 is the closest
    /// on-disk stand-in for a co-tenant-writable staging area.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_removes_swap_capability_from_the_staging_parent() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let parent = tmp.path().join("shared");
        std::fs::create_dir(&parent).expect("parent");
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o777))
            .expect("make parent shared-writable");

        create_ingest_dir(&parent.join("data.duckdb.ingest")).expect("create");

        let mode = std::fs::metadata(&parent)
            .expect("meta")
            .permissions()
            .mode()
            & 0o7777;
        assert_eq!(
            mode & 0o022,
            0,
            "no other principal may create or rename names in the staging parent; got {mode:o}"
        );
        assert_eq!(
            mode & 0o700,
            0o700,
            "the owner must keep full access to the staging parent; got {mode:o}"
        );
    }

    /// SEC / TASK-2039: a parent that is shared-writable but sticky already
    /// forbids other principals renaming or unlinking a name they do not own,
    /// so its mode is left alone rather than tightened — `ops` must not chmod
    /// a `/tmp`-style directory it happens to stage under.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_leaves_a_sticky_shared_parent_alone() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let parent = tmp.path().join("sticky");
        std::fs::create_dir(&parent).expect("parent");
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o1777))
            .expect("make parent sticky and shared-writable");

        create_ingest_dir(&parent.join("data.duckdb.ingest")).expect("create");

        let mode = std::fs::metadata(&parent)
            .expect("meta")
            .permissions()
            .mode()
            & 0o7777;
        assert_eq!(
            mode, 0o1777,
            "a sticky shared parent must keep its mode; got {mode:o}"
        );
    }

    /// SEC-25 / TASK-1857: a symlink planted at the ingest-dir path must be
    /// rejected, and the symlink's target must keep the mode it had — the
    /// old path-based `set_permissions` chmodded the target to 0o700.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_rejects_a_planted_symlink_and_leaves_target_mode() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("attacker-owned");
        std::fs::create_dir(&target).expect("target");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).expect("mode");

        let link = tmp.path().join("data.duckdb.ingest");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");

        let err = create_ingest_dir(&link).expect_err("symlinked ingest dir must be rejected");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("symlink"),
            "error should name the symlink: {err}"
        );

        let target_mode = std::fs::metadata(&target)
            .expect("target meta")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            target_mode, 0o755,
            "symlink target must keep its mode; got {target_mode:o}"
        );
        assert!(
            std::fs::symlink_metadata(&link)
                .expect("link meta")
                .file_type()
                .is_symlink(),
            "the planted symlink must be left in place, not replaced"
        );
    }

    /// SEC-25: the pre-existing-path policy is shared by both platform
    /// branches, so pin it directly. The non-Unix branch of
    /// `create_ingest_dir` cannot be exercised on this host, but it calls
    /// exactly this function, so a regression here breaks both.
    #[test]
    fn reject_untrusted_ingest_dir_accepts_only_a_real_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");

        let missing = tmp.path().join("absent.ingest");
        assert!(
            reject_untrusted_ingest_dir(&missing)
                .expect("absent path is not an error")
                .is_none(),
            "an absent path must report 'nothing here', not a rejection"
        );

        let real = tmp.path().join("real.ingest");
        std::fs::create_dir(&real).expect("mkdir");
        assert!(reject_untrusted_ingest_dir(&real)
            .expect("real dir accepted")
            .is_some());

        let file = tmp.path().join("file.ingest");
        std::fs::write(&file, b"x").expect("write");
        let err = reject_untrusted_ingest_dir(&file).expect_err("a file must be rejected");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("not a directory"),
            "error should say the path is not a directory: {err}"
        );

        #[cfg(unix)]
        {
            let link = tmp.path().join("link.ingest");
            std::os::unix::fs::symlink(&real, &link).expect("symlink");
            let err = reject_untrusted_ingest_dir(&link).expect_err("a symlink must be rejected");
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
            assert!(
                err.to_string().contains("symlink"),
                "error should name the symlink: {err}"
            );
        }
    }

    /// SEC-25 / TASK-1857: a plain file occupying the ingest-dir path is a
    /// hard error rather than something we chmod and write into.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_rejects_a_non_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("data.duckdb.ingest");
        std::fs::write(&path, b"not a dir").expect("write");
        let err = create_ingest_dir(&path).expect_err("file at ingest path must be rejected");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("not a directory"),
            "error should say the path is not a directory: {err}"
        );
    }

    #[test]
    fn data_dir_for_db_appends_ingest() {
        let path = PathBuf::from("/home/proj/target/ops/data.duckdb");
        let result = data_dir_for_db(&path).expect("file-backed path");
        assert_eq!(
            result,
            PathBuf::from("/home/proj/target/ops/data.duckdb.ingest")
        );
    }

    /// READ-5 / TASK-1867: the `:memory:` sentinel is a connection string,
    /// not a path. Deriving `:memory:.ingest` from it created a junk
    /// directory in the process working directory.
    #[test]
    fn data_dir_for_db_rejects_the_in_memory_sentinel() {
        let err = data_dir_for_db(Path::new(":memory:")).expect_err("sentinel must be rejected");
        assert!(
            matches!(err, DbError::NotFileBacked(ref p) if p == Path::new(":memory:")),
            "expected NotFileBacked, got: {err:?}"
        );
    }

    #[test]
    fn default_db_path_uses_target_dir() {
        let root = PathBuf::from("/home/proj");
        let path = default_db_path(&root);
        assert_eq!(path, PathBuf::from("/home/proj/target/ops/data.duckdb"));
    }

    #[test]
    fn external_err_wraps_display_error() {
        let err = external_err(anyhow::anyhow!("test error message"));
        let msg = err.to_string();
        assert!(msg.contains("test error message"));
    }

    /// SEC-21 (TASK-0862): the alternate-format wrapper must preserve the
    /// full anyhow context chain.
    #[test]
    fn external_err_preserves_anyhow_context_chain() {
        use anyhow::Context;
        let leaf = anyhow::Error::msg("leaf cause");
        let chained: anyhow::Error = Err::<(), _>(leaf)
            .context("wrap one")
            .context("wrap two")
            .unwrap_err();
        let err = external_err(chained);
        let msg = err.to_string();
        assert!(msg.contains("wrap two"), "missing outer wrap: {msg}");
        assert!(msg.contains("wrap one"), "missing middle wrap: {msg}");
        assert!(msg.contains("leaf cause"), "missing leaf cause: {msg}");
    }

    /// ERR-2 / TASK-1209: walking `std::error::Error::source()` on the
    /// resulting `DbError::External` recovers the wrapped `anyhow::Error`
    /// chain rather than the previous flattened-string leaf.
    #[test]
    fn external_err_preserves_error_source_chain() {
        use anyhow::Context;
        use std::error::Error as _;

        let chained: anyhow::Error = Err::<(), _>(anyhow::Error::msg("leaf cause"))
            .context("wrap one")
            .context("wrap two")
            .unwrap_err();
        let err = external_err(chained);

        // First source = the wrapped anyhow::Error itself; subsequent calls
        // walk the anyhow context chain down to the leaf cause.
        let mut messages = Vec::new();
        let mut current: Option<&dyn std::error::Error> = err.source();
        while let Some(e) = current {
            messages.push(e.to_string());
            current = e.source();
        }
        let joined = messages.join(" | ");
        assert!(
            joined.contains("leaf cause"),
            "expected leaf cause in source chain, got: {joined}"
        );
    }

    #[test]
    fn checksum_file_returns_sha256_hex() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.json");
        std::fs::write(&path, r#"{"test": "data"}"#).expect("write");
        let checksum = checksum_file(&path).expect("checksum");
        assert_eq!(checksum.len(), 64, "SHA-256 hex should be 64 chars");
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn checksum_file_fails_when_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = checksum_file(&dir.path().join("nonexistent.json"));
        assert!(result.is_err(), "should fail for missing file");
    }

    #[test]
    fn checksum_file_streaming_matches_in_memory_for_large_input() {
        use sha2::{Digest, Sha256};
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("big.bin");
        // Same byte sequence as `|i| i % 256`, built without a cast: 200 KiB
        // is an exact multiple of 256, so the cycle ends on a full period.
        let data: Vec<u8> = (0..=u8::MAX).cycle().take(200 * 1024).collect();
        std::fs::write(&path, &data).expect("write");

        let streamed = checksum_file(&path).expect("stream");
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let in_memory = hex::encode(hasher.finalize().as_slice());
        assert_eq!(streamed, in_memory);
    }

    #[test]
    fn checksum_file_is_deterministic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.json");
        std::fs::write(&path, b"test data").expect("write");
        let c1 = checksum_file(&path).expect("checksum1");
        let c2 = checksum_file(&path).expect("checksum2");
        assert_eq!(c1, c2, "checksum should be deterministic");
    }
}
