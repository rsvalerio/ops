//! Shared `.ops.toml` edit helper used by interactive CLI handlers.
//!
//! Consolidates the read → parse → mutate → atomic-write pattern previously
//! duplicated across `theme_cmd`, `about_cmd`, `new_command_cmd` and
//! `hook-common`. Three important properties:
//!
//! - A missing file is treated as empty (no check-then-read TOCTOU).
//! - A parse error is propagated with the file path as context rather than
//!   silently discarded (would overwrite the user's malformed-but-meaningful
//!   file with an empty one).
//! - Writes are atomic: a sibling temp file is written then renamed, so a
//!   crash mid-write leaves the previous content intact.
//!
//! See also ERR-5, SEC-25, SEC-32 in the backlog for the motivating findings.

use std::io::Write;
use std::path::Path;

use anyhow::Context;

/// Read and parse `.ops.toml` at `path`, treating missing as an empty
/// document. Used by callers that want to inspect the document without
/// necessarily writing back.
pub fn read_ops_toml(path: &Path) -> anyhow::Result<toml_edit::DocumentMut> {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("failed to read config file: {}", path.display()));
        }
    };
    content.parse::<toml_edit::DocumentMut>().with_context(|| {
        format!(
            "failed to parse {} as TOML; refusing to overwrite to avoid data loss",
            path.display()
        )
    })
}

/// Atomically write the serialized `doc` back to `path` (sibling temp file +
/// rename). Pair with [`read_ops_toml`] for a read / mutate / write pipeline
/// where the caller wants to skip the write on some branches.
pub fn write_ops_toml(path: &Path, doc: &toml_edit::DocumentMut) -> anyhow::Result<()> {
    atomic_write(path, doc.to_string().as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))
}

/// Load `.ops.toml` at `path` (missing → empty), apply `mutate`, then write
/// atomically back to `path`.
///
/// # Errors
///
/// - Returns an error if the file exists but fails to read (anything other
///   than `NotFound`).
/// - Returns an error if the existing content fails to parse as TOML.
/// - Returns any error the `mutate` closure returns.
/// - Returns an error if the atomic write fails.
pub fn edit_ops_toml<F>(path: &Path, mutate: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut toml_edit::DocumentMut) -> anyhow::Result<()>,
{
    let mut doc = read_ops_toml(path)?;
    mutate(&mut doc)?;
    write_ops_toml(path, &doc)
}

/// Write `bytes` to `path` atomically by writing to a sibling temp file and
/// renaming. On error the original content at `path` is untouched.
///
/// The temp file name is unique per (process, monotonic counter, nanos) so two
/// concurrent writers — even within the same process — cannot race on the same
/// sibling path. After the rename the parent directory is fsync-d on Unix so
/// the new directory entry survives a crash.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no file name")
        })?
        .to_string_lossy()
        .into_owned();

    let pid = std::process::id();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let tmp = parent.join(format!(".{file_name}.tmp.{pid}.{counter}.{nanos}"));

    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }

    if let Err(e) = std::fs::rename(&tmp, path) {
        if let Err(cleanup) = std::fs::remove_file(&tmp) {
            tracing::warn!(
                tmp = %tmp.display(),
                error = %cleanup,
                "leaked atomic_write temp file after rename failure",
            );
        }
        return Err(e);
    }

    // Persist the new directory entry so a crash after rename still finds the
    // updated file. macOS does not require this for crash safety in practice,
    // but Linux ext4 does, and it is cheap.
    #[cfg(unix)]
    if let Ok(dir) = std::fs::File::open(parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_missing_file_treated_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".ops.toml");
        edit_ops_toml(&path, |doc| {
            doc["output"] = toml_edit::Item::Table(toml_edit::Table::new());
            doc["output"]["theme"] = toml_edit::value("classic");
            Ok(())
        })
        .unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("theme = \"classic\""));
    }

    #[test]
    fn edit_malformed_file_preserved_on_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".ops.toml");
        let bad = "this is = = not valid toml {{{";
        std::fs::write(&path, bad).unwrap();

        let result = edit_ops_toml(&path, |_doc| Ok(()));
        assert!(result.is_err(), "expected parse failure");
        let err = format!("{:#}", result.unwrap_err());
        assert!(err.contains("TOML"), "err should mention TOML: {err}");

        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after, bad, "malformed file must not be overwritten");
    }

    #[test]
    fn edit_writes_atomically_replaces_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".ops.toml");
        std::fs::write(&path, "[output]\ntheme = \"compact\"\n").unwrap();

        edit_ops_toml(&path, |doc| {
            doc["output"]["theme"] = toml_edit::value("classic");
            Ok(())
        })
        .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("theme = \"classic\""));
        let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1, "temp file should have been renamed away");
    }

    #[test]
    fn atomic_write_uses_unique_temp_per_call() {
        // Two back-to-back writes must not collide on a deterministic temp
        // name. If they shared one, the second create_new would fail.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.toml");
        atomic_write(&path, b"first").unwrap();
        atomic_write(&path, b"second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
    }

    #[test]
    fn atomic_write_rename_failure_does_not_leak_tmp() {
        // Forcing rename failure: target path is an existing non-empty
        // directory. The remove_file fallback should clear the sibling tmp
        // so no .{name}.tmp.* file lingers in the parent.
        let dir = tempfile::tempdir().unwrap();
        let target_dir = dir.path().join("target");
        std::fs::create_dir(&target_dir).unwrap();
        std::fs::write(target_dir.join("inside"), b"x").unwrap();

        let err = atomic_write(&target_dir, b"data").unwrap_err();
        assert!(matches!(
            err.kind(),
            std::io::ErrorKind::IsADirectory
                | std::io::ErrorKind::DirectoryNotEmpty
                | std::io::ErrorKind::Other
        ));

        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                let n = e.file_name();
                let s = n.to_string_lossy();
                s.starts_with(".target.tmp.")
            })
            .collect();
        assert!(
            leftovers.is_empty(),
            "expected tmp cleanup, leftovers: {leftovers:?}"
        );
    }

    #[test]
    fn edit_mutate_error_leaves_file_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".ops.toml");
        let original = "[output]\ntheme = \"compact\"\n";
        std::fs::write(&path, original).unwrap();

        let result = edit_ops_toml(&path, |_doc| anyhow::bail!("mutate failed"));
        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }
}
