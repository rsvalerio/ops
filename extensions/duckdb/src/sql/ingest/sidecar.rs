//! Workspace-root sidecar I/O for ingestors that don't embed the path in JSON.

use crate::{DbError, DbResult};
use std::path::{Path, PathBuf};

/// Single source of truth for the workspace sidecar filename convention
/// (DUP-3). All write/read/remove helpers route through here.
pub fn sidecar_path(data_dir: &Path, name: &str) -> PathBuf {
    data_dir.join(format!("{name}_workspace.txt"))
}

/// Write a workspace root sidecar file alongside collected data.
///
/// Used by ingestors that don't embed workspace_root in their JSON output
/// (e.g., tokei, coverage). The sidecar is read back during `load()` for
/// `upsert_data_source`.
///
/// Persists the path's raw OS bytes (via `as_encoded_bytes`) so that
/// non-UTF-8 paths round-trip exactly rather than being silently corrupted
/// to `U+FFFD` (READ-5).
pub fn write_workspace_sidecar(
    data_dir: &Path,
    name: &str,
    working_directory: &Path,
) -> DbResult<()> {
    let workspace_path = sidecar_path(data_dir, name);
    // SEC-25 (TASK-0663): a bare `fs::write` could leave a zero-byte or torn
    // sidecar after a crash; route through `atomic_write` so the destination
    // only appears once the temp file has been fsync'd and renamed.
    ops_core::config::atomic_write(
        &workspace_path,
        working_directory.as_os_str().as_encoded_bytes(),
    )
    .map_err(DbError::Io)
}

/// SEC-33 / TASK-0951: hard cap on workspace sidecar read size. A real
/// sidecar holds a single filesystem path (kilobytes at most); an
/// adversarial or `/dev/zero`-symlinked sidecar could otherwise OOM the
/// CLI before the unsafe `from_encoded_bytes_unchecked` boundary.
pub const MAX_SIDECAR_BYTES: u64 = 4 * 1024 * 1024;

/// Read a workspace root sidecar file written during collect.
///
/// SEC-33 / TASK-0951: read is bounded by [`MAX_SIDECAR_BYTES`].
/// SEC-21 / TASK-1217: rejects ASCII control bytes at the read boundary.
/// UNSAFE-1 (TASK-1104): no `from_encoded_bytes_unchecked` — uses
/// `OsString::from_vec` on Unix and validated UTF-8 elsewhere.
pub fn read_workspace_sidecar(data_dir: &Path, name: &str) -> DbResult<std::ffi::OsString> {
    use std::io::Read;
    let workspace_path = sidecar_path(data_dir, name);
    let mut file = std::fs::File::open(&workspace_path).map_err(DbError::Io)?;
    let limit = MAX_SIDECAR_BYTES.saturating_add(1);
    let mut bytes = Vec::new();
    (&mut file)
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(DbError::Io)?;
    if bytes.len() as u64 > MAX_SIDECAR_BYTES {
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
pub fn remove_workspace_sidecar(data_dir: &Path, name: &str) {
    let workspace_path = sidecar_path(data_dir, name);
    match std::fs::remove_file(&workspace_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            tracing::warn!(
                "remove_workspace_sidecar({}): {e}",
                workspace_path.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn workspace_sidecar_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let working = PathBuf::from("/some/workspace/root");
        write_workspace_sidecar(dir.path(), "tokei", &working).expect("write sidecar");

        let expected = dir.path().join("tokei_workspace.txt");
        assert!(expected.exists(), "sidecar file at expected path");

        let read = read_workspace_sidecar(dir.path(), "tokei").expect("read sidecar");
        assert_eq!(read, "/some/workspace/root");

        remove_workspace_sidecar(dir.path(), "tokei");
        assert!(!expected.exists(), "sidecar removed");
    }

    #[test]
    #[cfg(unix)]
    fn workspace_sidecar_round_trips_non_utf8_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let bytes = b"/ws/\xff\xfe/proj";
        let working = PathBuf::from(OsStr::from_bytes(bytes));
        write_workspace_sidecar(dir.path(), "tokei", &working).expect("write");

        let raw = std::fs::read(dir.path().join("tokei_workspace.txt")).expect("read raw");
        assert_eq!(raw, bytes, "non-UTF-8 bytes preserved verbatim");
    }

    /// ERR-4 / TASK-0928: round-trips non-UTF-8 OS bytes via the read helper.
    #[test]
    #[cfg(unix)]
    fn read_workspace_sidecar_round_trips_non_utf8_via_helper() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let dir = tempfile::tempdir().expect("tempdir");
        let bytes = b"/ws/\xff\xfe/proj";
        let working = PathBuf::from(OsStr::from_bytes(bytes));
        write_workspace_sidecar(dir.path(), "tokei", &working).expect("write");

        let read = read_workspace_sidecar(dir.path(), "tokei").expect("read sidecar");
        assert_eq!(
            read.into_vec(),
            bytes.to_vec(),
            "non-UTF-8 bytes survive write→read round-trip via helper"
        );
    }

    /// SEC-25 (TASK-0663): atomic write leaves no temp sibling.
    #[test]
    fn workspace_sidecar_write_is_atomic_and_leaves_no_temp() {
        let dir = tempfile::tempdir().expect("tempdir");
        let working = PathBuf::from("/some/workspace/root");
        write_workspace_sidecar(dir.path(), "tokei", &working).expect("write sidecar");

        let dest = dir.path().join("tokei_workspace.txt");
        let bytes = std::fs::read(&dest).expect("read dest");
        assert_eq!(bytes, b"/some/workspace/root");

        let leftover = std::fs::read_dir(dir.path())
            .expect("readdir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|name| name.starts_with(".tokei_workspace.txt.tmp."));
        assert!(leftover.is_none(), "atomic_write left a temp: {leftover:?}");
    }

    /// SEC-33 / TASK-0951: oversized sidecar errors out.
    #[test]
    fn read_workspace_sidecar_rejects_oversize_input() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = sidecar_path(dir.path(), "huge");
        let oversize = (MAX_SIDECAR_BYTES + 1) as usize;
        std::fs::write(&path, vec![b'a'; oversize]).expect("plant oversize sidecar");

        let err =
            read_workspace_sidecar(dir.path(), "huge").expect_err("oversize sidecar must error");
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
        let dir = tempfile::tempdir().expect("tempdir");
        let path = sidecar_path(dir.path(), "tampered");
        std::fs::write(&path, b"/ws/path\nfake/path").expect("plant tampered sidecar");

        let err = read_workspace_sidecar(dir.path(), "tampered")
            .expect_err("control-byte sidecar must error");
        match err {
            DbError::Io(e) => assert_eq!(
                e.kind(),
                std::io::ErrorKind::InvalidData,
                "expected InvalidData, got {e:?}"
            ),
            other => panic!("expected DbError::Io, got {other:?}"),
        }
    }

    #[test]
    fn workspace_sidecar_remove_is_best_effort() {
        let dir = tempfile::tempdir().expect("tempdir");
        remove_workspace_sidecar(dir.path(), "missing_name");
    }

    #[test]
    fn workspace_sidecar_remove_logs_but_does_not_panic_on_failure() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("blocker_workspace.txt")).expect("create blocker dir");
        remove_workspace_sidecar(dir.path(), "blocker");
        assert!(dir.path().join("blocker_workspace.txt").exists());
    }

    #[test]
    fn workspace_sidecar_filename_uses_name_prefix() {
        let dir = tempfile::tempdir().expect("tempdir");
        let working = PathBuf::from("/ws");
        write_workspace_sidecar(dir.path(), "coverage", &working).expect("write");
        write_workspace_sidecar(dir.path(), "tokei", &working).expect("write");
        assert!(dir.path().join("coverage_workspace.txt").exists());
        assert!(dir.path().join("tokei_workspace.txt").exists());
    }

    #[cfg(not(unix))]
    #[test]
    fn read_workspace_sidecar_rejects_invalid_utf8_on_non_unix() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = sidecar_path(dir.path(), "bad");
        std::fs::write(&path, [0xFFu8, 0xFE, 0xFD]).expect("plant bad sidecar");
        let err = read_workspace_sidecar(dir.path(), "bad")
            .expect_err("invalid encoding must error, not UB");
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
