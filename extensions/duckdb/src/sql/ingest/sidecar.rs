//! Workspace-root sidecar I/O for ingestors that don't embed the path in JSON.

use super::dir::IngestDir;
use crate::{DbError, DbResult};

/// Single source of truth for the workspace sidecar filename convention
/// (DUP-3). All write/read/remove helpers route through here.
///
/// SEC-25 / TASK-2054: returns the bare **entry name**, not a joined path.
/// Every caller now feeds it to an [`IngestDir`] method that resolves it
/// against the verified directory descriptor, so there is no path to join and
/// nothing to re-resolve by name.
#[must_use]
pub fn sidecar_name(name: &str) -> String {
    format!("{name}_workspace.txt")
}

/// Write a workspace root sidecar file alongside collected data.
///
/// Used by ingestors that don't embed `workspace_root` in their JSON output
/// (e.g., tokei, coverage). The sidecar is read back during `load()` for
/// `upsert_data_source`.
///
/// Persists the path's raw OS bytes (via `as_encoded_bytes`) so that
/// non-UTF-8 paths round-trip exactly rather than being silently corrupted
/// to `U+FFFD` (READ-5).
///
/// # Errors
///
/// [`DbError::Io`] if the sidecar cannot be written atomically.
pub fn write_workspace_sidecar(
    dir: &IngestDir,
    name: &str,
    working_directory: &std::path::Path,
) -> DbResult<()> {
    // SEC-25 (TASK-0663): a bare `fs::write` could leave a zero-byte or torn
    // sidecar after a crash; the write is atomic (temp + fsync + rename).
    // SEC-25 / TASK-2054: and anchored — the temp is created and renamed
    // through the verified directory descriptor, so swapping the ingest dir's
    // *name* after verification cannot redirect the staged sidecar.
    dir.write_atomic(
        &sidecar_name(name),
        working_directory.as_os_str().as_encoded_bytes(),
    )
}

/// SEC-33 / TASK-0951: hard cap on workspace sidecar read size.
///
/// A real sidecar holds a single filesystem path (kilobytes at most);
/// an adversarial or `/dev/zero`-symlinked sidecar could otherwise OOM
/// the CLI before the unsafe `from_encoded_bytes_unchecked` boundary.
pub const MAX_SIDECAR_BYTES: u64 = 4 * 1024 * 1024;

/// Read a workspace root sidecar file written during collect.
///
/// SEC-33 / TASK-0951: read is bounded by [`MAX_SIDECAR_BYTES`].
/// SEC-21 / TASK-1217: rejects ASCII control bytes at the read boundary.
/// UNSAFE-1 (TASK-1104): no `from_encoded_bytes_unchecked` — uses
/// `OsString::from_vec` on Unix and validated UTF-8 elsewhere.
/// SEC-25 / TASK-2054: opened through the verified directory descriptor
/// (`openat`, `O_NOFOLLOW`), so neither the directory name nor a symlink at the
/// sidecar's own name can redirect the read.
///
/// # Errors
///
/// [`DbError::Io`] if the sidecar is missing or unreadable. Exceeding
/// [`MAX_SIDECAR_BYTES`] also surfaces as `DbError::Io`, with
/// [`std::io::ErrorKind::InvalidData`] — there is no dedicated oversize
/// variant (READ-4 / TASK-1875: this section used to name a
/// `DbError::SidecarTooLarge` that does not exist).
pub fn read_workspace_sidecar(dir: &IngestDir, name: &str) -> DbResult<std::ffi::OsString> {
    use std::io::Read;
    let mut file = dir.open_read(&sidecar_name(name))?;
    let limit = MAX_SIDECAR_BYTES.saturating_add(1);
    let mut bytes = Vec::new();
    (&mut file)
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(DbError::Io)?;
    // A length that does not fit in a `u64` is necessarily far above the
    // 4 MiB cap, so saturating to `u64::MAX` keeps this comparison exact for
    // every value the check can actually distinguish.
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_SIDECAR_BYTES {
        return Err(DbError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("workspace sidecar exceeds {MAX_SIDECAR_BYTES} byte cap; refusing to load"),
        )));
    }
    if let Some(idx) = bytes.iter().position(|b| (*b <= 0x1f) || *b == 0x7f) {
        return Err(DbError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "workspace sidecar contains ASCII control byte at offset {idx}; \
                 refusing to load (SEC-21 defense-in-depth, see TASK-1217)"
            ),
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(std::ffi::OsString::from_vec(bytes))
    }
    #[cfg(not(unix))]
    {
        let s = std::str::from_utf8(&bytes).map_err(|e| {
            DbError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("workspace sidecar contains invalid UTF-8: {e}"),
            ))
        })?;
        Ok(std::ffi::OsString::from(s))
    }
}

/// Remove a workspace root sidecar file. Best-effort: a missing file is
/// fine, but other errors (EACCES, IO) are logged so accumulated stale
/// sidecars do not silently mask broken cleanup (ERR-1).
///
/// SEC-25 / TASK-2054: unlinked through the verified directory descriptor
/// (`unlinkat`), for the same reason the write is anchored.
pub fn remove_workspace_sidecar(dir: &IngestDir, name: &str) {
    let entry = sidecar_name(name);
    match dir.remove_file(&entry) {
        Ok(()) => {}
        Err(DbError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            tracing::warn!(
                "remove_workspace_sidecar({}): {e}",
                dir.entry_path(&entry).display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Every test stages into a fresh verified anchor, exactly as
    /// `provide_via_ingestor` does.
    fn anchor(tmp: &tempfile::TempDir) -> IngestDir {
        IngestDir::open(&tmp.path().join("data.duckdb.ingest")).expect("open ingest dir")
    }

    #[test]
    fn workspace_sidecar_round_trip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&tmp);
        let working = PathBuf::from("/some/workspace/root");
        write_workspace_sidecar(&dir, "tokei", &working).expect("write sidecar");

        let expected = dir.entry_path("tokei_workspace.txt");
        assert!(expected.exists(), "sidecar file at expected path");

        let read = read_workspace_sidecar(&dir, "tokei").expect("read sidecar");
        assert_eq!(read, "/some/workspace/root");

        remove_workspace_sidecar(&dir, "tokei");
        assert!(!expected.exists(), "sidecar removed");
    }

    #[test]
    #[cfg(unix)]
    fn workspace_sidecar_round_trips_non_utf8_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&tmp);
        let bytes = b"/ws/\xff\xfe/proj";
        let working = PathBuf::from(OsStr::from_bytes(bytes));
        write_workspace_sidecar(&dir, "tokei", &working).expect("write");

        let raw = std::fs::read(dir.entry_path("tokei_workspace.txt")).expect("read raw");
        assert_eq!(raw, bytes, "non-UTF-8 bytes preserved verbatim");
    }

    /// ERR-4 / TASK-0928: round-trips non-UTF-8 OS bytes via the read helper.
    #[test]
    #[cfg(unix)]
    fn read_workspace_sidecar_round_trips_non_utf8_via_helper() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&tmp);
        let bytes = b"/ws/\xff\xfe/proj";
        let working = PathBuf::from(OsStr::from_bytes(bytes));
        write_workspace_sidecar(&dir, "tokei", &working).expect("write");

        let read = read_workspace_sidecar(&dir, "tokei").expect("read sidecar");
        assert_eq!(
            read.into_vec(),
            bytes.to_vec(),
            "non-UTF-8 bytes survive write→read round-trip via helper"
        );
    }

    /// SEC-25 (TASK-0663): atomic write leaves no temp sibling.
    #[test]
    fn workspace_sidecar_write_is_atomic_and_leaves_no_temp() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&tmp);
        let working = PathBuf::from("/some/workspace/root");
        write_workspace_sidecar(&dir, "tokei", &working).expect("write sidecar");

        let dest = dir.entry_path("tokei_workspace.txt");
        let bytes = std::fs::read(&dest).expect("read dest");
        assert_eq!(bytes, b"/some/workspace/root");

        let leftover = std::fs::read_dir(dir.path())
            .expect("readdir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|name| name.starts_with(".tokei_workspace.txt.tmp."));
        assert!(
            leftover.is_none(),
            "anchored write left a temp: {leftover:?}"
        );
    }

    /// SEC-33 / TASK-0951: oversized sidecar errors out.
    #[test]
    fn read_workspace_sidecar_rejects_oversize_input() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&tmp);
        // The 4 MiB cap fits every `usize` these tests run on; on a narrower
        // platform `usize::MAX` would itself be below the cap, so the
        // fallback still yields an allocatable buffer instead of an unwrap.
        let oversize = usize::try_from(MAX_SIDECAR_BYTES.saturating_add(1)).unwrap_or(usize::MAX);
        std::fs::write(dir.entry_path("huge_workspace.txt"), vec![b'a'; oversize])
            .expect("plant oversize sidecar");

        let err = read_workspace_sidecar(&dir, "huge").expect_err("oversize sidecar must error");
        match err {
            DbError::Io(e) => assert_eq!(
                e.kind(),
                std::io::ErrorKind::InvalidData,
                "expected InvalidData, got {e:?}"
            ),
            other => panic!("expected DbError::Io, got {other:?}"),
        }
    }

    /// SEC-21 / TASK-1217: tampered sidecar with control byte rejected.
    #[test]
    fn read_workspace_sidecar_rejects_embedded_newline() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&tmp);
        std::fs::write(
            dir.entry_path("tampered_workspace.txt"),
            b"/ws/path\nfake/path",
        )
        .expect("plant tampered sidecar");

        let err =
            read_workspace_sidecar(&dir, "tampered").expect_err("control-byte sidecar must error");
        match err {
            DbError::Io(e) => assert_eq!(
                e.kind(),
                std::io::ErrorKind::InvalidData,
                "expected InvalidData, got {e:?}"
            ),
            other => panic!("expected DbError::Io, got {other:?}"),
        }
    }

    /// SEC-25 / TASK-2054: a symlink planted at the *sidecar's own* name is
    /// refused rather than read through — `openat` carries `O_NOFOLLOW`.
    #[test]
    #[cfg(unix)]
    fn read_workspace_sidecar_refuses_symlinked_entry() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&tmp);
        let secret = tmp.path().join("secret.txt");
        std::fs::write(&secret, b"/attacker/root").expect("write secret");
        std::os::unix::fs::symlink(&secret, dir.entry_path("linked_workspace.txt"))
            .expect("plant symlink");

        let err =
            read_workspace_sidecar(&dir, "linked").expect_err("symlinked sidecar must not be read");
        assert!(
            matches!(err, DbError::Io(_)),
            "expected an IO error, got {err:?}"
        );
    }

    #[test]
    fn workspace_sidecar_remove_is_best_effort() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&tmp);
        remove_workspace_sidecar(&dir, "missing_name");
    }

    #[test]
    fn workspace_sidecar_remove_logs_but_does_not_panic_on_failure() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&tmp);
        std::fs::create_dir(dir.entry_path("blocker_workspace.txt")).expect("create blocker dir");
        remove_workspace_sidecar(&dir, "blocker");
        assert!(dir.entry_path("blocker_workspace.txt").exists());
    }

    #[test]
    fn workspace_sidecar_filename_uses_name_prefix() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&tmp);
        let working = PathBuf::from("/ws");
        write_workspace_sidecar(&dir, "coverage", &working).expect("write");
        write_workspace_sidecar(&dir, "tokei", &working).expect("write");
        assert!(dir.entry_path("coverage_workspace.txt").exists());
        assert!(dir.entry_path("tokei_workspace.txt").exists());
    }

    #[cfg(not(unix))]
    #[test]
    fn read_workspace_sidecar_rejects_invalid_utf8_on_non_unix() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&tmp);
        std::fs::write(dir.entry_path("bad_workspace.txt"), [0xFFu8, 0xFE, 0xFD])
            .expect("plant bad sidecar");
        let err =
            read_workspace_sidecar(&dir, "bad").expect_err("invalid encoding must error, not UB");
        match err {
            DbError::Io(e) => assert_eq!(
                e.kind(),
                std::io::ErrorKind::InvalidData,
                "expected InvalidData, got {e:?}"
            ),
            other => panic!("expected DbError::Io, got {other:?}"),
        }
    }
}
