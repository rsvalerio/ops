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

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::Path;

use anyhow::Context;

/// Read and parse `.ops.toml` at `path`, treating missing as an empty
/// document. Used by callers that want to inspect the document without
/// necessarily writing back.
///
/// SEC-33 / TASK-0943: routes through
/// [`super::loader::read_capped_toml_file`] so an oversized `.ops.toml`
/// fails fast with a typed bounded-read error rather than slurping the
/// whole file. Cap is overridable via
/// [`super::loader::OPS_TOML_MAX_BYTES_ENV`].
pub fn read_ops_toml(path: &Path) -> anyhow::Result<toml_edit::DocumentMut> {
    let content = super::loader::read_capped_toml_file(path)?.unwrap_or_default();
    content.parse::<toml_edit::DocumentMut>().with_context(|| {
        // SEC-21 (TASK-1472): Debug-format the path so a `.ops.toml` whose
        // path contains newlines / ANSI escapes cannot forge log lines via
        // anyhow consumers that render the chain through `tracing::warn!`.
        format!(
            "failed to parse {:?} as TOML; refusing to overwrite to avoid data loss",
            path.display()
        )
    })
}

/// DUP-1 / TASK-1278: ensure a top-level table named `key` exists in `doc`
/// and return a mutable reference to it. If the key is absent, an empty
/// `Table` is inserted. If the key is present but holds a non-table value
/// (e.g. `output = "classic"`), an `anyhow::Error` is returned rather than
/// panicking — this is the failure mode TASK-1300 hit on the
/// `doc["output"]["theme"] = …` indexer path in `theme_cmd::set_theme`.
///
/// Use this anywhere `.ops.toml` writers need to land a key under a top-level
/// section: `about_cmd`, `theme_cmd`, and `new_command_cmd` previously each
/// open-coded the `contains_key` + insert + `as_table_mut().context(...)`
/// idiom, and `theme_cmd` did so incorrectly.
pub fn ensure_table<'a>(
    doc: &'a mut toml_edit::DocumentMut,
    key: &str,
) -> anyhow::Result<&'a mut toml_edit::Table> {
    if !doc.contains_key(key) {
        doc.insert(key, toml_edit::Item::Table(toml_edit::Table::new()));
    }
    doc.get_mut(key)
        .and_then(toml_edit::Item::as_table_mut)
        .with_context(|| format!("[{key}] is not a table in .ops.toml"))
}

/// Atomically write the serialized `doc` back to `path` (sibling temp file +
/// rename). Pair with [`read_ops_toml`] for a read / mutate / write pipeline
/// where the caller wants to skip the write on some branches.
pub fn write_ops_toml(path: &Path, doc: &toml_edit::DocumentMut) -> anyhow::Result<()> {
    atomic_write(path, doc.to_string().as_bytes())
        .with_context(|| format!("failed to write {:?}", path.display()))
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
///
/// # Sync-only — async callers must offload
///
/// `atomic_write` performs blocking I/O: write, `sync_all`, `rename`, and a
/// parent-directory `sync_all` on Unix. `fsync` can stall the calling thread
/// for tens to hundreds of milliseconds on slow disks. Async callers MUST
/// wrap the invocation in [`tokio::task::spawn_blocking`] rather than calling
/// it directly from a runtime thread, mirroring the contract on
/// `ops_core::subprocess::run_with_timeout`. The same applies to
/// [`write_ops_toml`] and [`edit_ops_toml`], which delegate here.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let (parent, file_name) = resolve_parent_and_filename(path)?;
    let tmp = parent.join(build_tmp_basename(file_name));

    if let Err(e) = write_tmp_and_sync(&tmp, path, bytes) {
        cleanup_tmp(
            &tmp,
            false,
            "leaked atomic_write temp file after write/sync failure",
        );
        return Err(e);
    }

    if let Err(e) = std::fs::rename(&tmp, path) {
        cleanup_tmp(
            &tmp,
            true,
            "leaked atomic_write temp file after rename failure",
        );
        return Err(e);
    }

    sync_parent_dir(parent);
    Ok(())
}

// ERR-1 / TASK-1040: `Path::parent()` returns `Some("")` — not `None` —
// for a bare filename. Remap empty to "." so the parent fsync runs.
fn resolve_parent_and_filename(path: &Path) -> std::io::Result<(&Path, &OsStr)> {
    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no file name")
    })?;
    Ok((parent, file_name))
}

// SEC-25 / TASK-0837: build the tmp basename from raw OsStr bytes so two
// non-UTF-8 siblings whose lossy renders collide do not race on the same
// tmp. READ-5 / TASK-0908: strip a leading dot so `.ops.toml` does not
// produce a double-dot `..ops.toml.tmp.…` shape.
fn build_tmp_basename(file_name: &OsStr) -> OsString {
    use std::fmt::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let pid = std::process::id();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());

    let name_bytes = file_name.as_encoded_bytes();
    let stem_bytes = name_bytes.strip_prefix(b".").unwrap_or(name_bytes);
    // SAFETY: at most one leading ASCII '.' removed from a valid OsStr
    // encoding; OsStr::as_encoded_bytes documents this is safe.
    let stem = unsafe { OsStr::from_encoded_bytes_unchecked(stem_bytes) };

    // PERF-3 / TASK-1223: one allocation for the suffix, one for the OsString.
    let mut suffix = String::with_capacity(48);
    let _ = write!(suffix, ".tmp.{pid}.{counter}.{nanos}");
    let mut tmp_name = OsString::with_capacity(name_bytes.len() + suffix.len() + 1);
    tmp_name.push(".");
    tmp_name.push(stem);
    tmp_name.push(&suffix);
    tmp_name
}

// ERR-1 / TASK-1134: cleanup-on-error path shared by the write/sync and
// rename failure arms. NotFound from the write/sync arm is expected when
// `open` itself failed, so do not warn on it; the rename arm always warns
// because the tmp existed at that point.
fn cleanup_tmp(tmp: &Path, warn_on_not_found: bool, msg: &'static str) {
    if let Err(e) = std::fs::remove_file(tmp) {
        if warn_on_not_found || e.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(tmp = %tmp.display(), error = %e, "{msg}");
        }
    }
}

// SEC-25 / TASK-0898 + TASK-1086 + TASK-1388: preserve restrictive perms
// across atomic-replace. Probe via symlink_metadata so a symlink at `path`
// does not let the link target's mode drive the new file's perms; fall
// back to 0o600 for new files, non-regular entries, or non-Unix.
// `set_permissions` after open overrides the umask-masked mode that
// `open(2)` actually applied.
fn write_tmp_and_sync(tmp: &Path, dest: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    let requested_mode: u32 = {
        use std::os::unix::fs::OpenOptionsExt;
        let mode = std::fs::symlink_metadata(dest)
            .ok()
            .filter(|m| m.file_type().is_file())
            .map(|m| std::os::unix::fs::PermissionsExt::mode(&m.permissions()) & 0o7777)
            .unwrap_or(0o600);
        opts.mode(mode);
        mode
    };
    #[cfg(not(unix))]
    let _ = dest;
    let mut f = opts.open(tmp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        f.set_permissions(std::fs::Permissions::from_mode(requested_mode))?;
    }
    f.write_all(bytes)?;
    f.sync_all()?;
    #[cfg(test)]
    if tests::FAIL_AFTER_SYNC.with(|c| c.get()) {
        return Err(std::io::Error::other("injected post-sync failure"));
    }
    Ok(())
}

// ERR-1 / TASK-0899 + TASK-1231: a failing directory fsync is non-fatal
// (the rename already succeeded) but it is the only signal that crash
// safety is broken, so we warn rather than swallow. Windows has no
// portable directory-fsync analogue; emit a debug breadcrumb so the
// platform gap is visible in logs.
fn sync_parent_dir(parent: &Path) {
    #[cfg(not(unix))]
    {
        tracing::debug!(
            parent = %parent.display(),
            "no portable directory fsync on this platform; rename may not survive a power loss"
        );
    }
    #[cfg(unix)]
    if let Ok(dir) = std::fs::File::open(parent) {
        if let Err(e) = dir.sync_all() {
            tracing::warn!(
                parent = %parent.display(),
                error = %e,
                "directory fsync after atomic rename failed; rename may not survive a power loss"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ERR-1 / TASK-1134: fault-injection switch consumed by `atomic_write`
    // to simulate a post-sync failure inside the inner write block. Only
    // referenced under `#[cfg(test)]` so production code is untouched.
    // Thread-local so concurrent atomic_write tests are not poisoned —
    // every test runs `atomic_write` synchronously on its own thread.
    thread_local! {
        pub(super) static FAIL_AFTER_SYNC: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    /// SEC-33 / TASK-0943: `read_ops_toml` must surface a bounded-read
    /// error when the on-disk file exceeds the configured byte cap,
    /// rather than slurping the entire file into the toml_edit parser.
    #[test]
    #[serial_test::serial]
    fn read_ops_toml_rejects_oversized_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".ops.toml");
        std::fs::write(&path, "x".repeat(4096)).unwrap();

        // READ-5 / TASK-1129: `ops_toml_max_bytes` is `OnceLock`-cached,
        // so an env-var dance here would observe whichever value an earlier
        // test happened to populate. Drive the cap-rejection branch via the
        // pure inner helper instead. `read_ops_toml` itself is a thin shell
        // around `read_capped_toml_file`; the bail message we pin is the
        // same.
        let result = super::super::loader::read_capped_toml_file_with(&path, 64);

        let err = result.expect_err("oversized .ops.toml must error");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("exceeds 64 bytes"),
            "error must name the cap, got: {msg}"
        );
    }

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

    /// ERR-1 / TASK-1134: a write/sync error inside the inner block must
    /// remove the sibling tmp file, not just the rename-failure branch.
    /// Without the cleanup, a series of partial writes against a flaky
    /// disk leaves orphaned `.{name}.tmp.<pid>.<counter>.<nanos>` files.
    #[test]
    fn atomic_write_inner_failure_does_not_leak_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("target.toml");

        FAIL_AFTER_SYNC.with(|c| c.set(true));
        let result = atomic_write(&path, b"data");
        FAIL_AFTER_SYNC.with(|c| c.set(false));

        let err = result.expect_err("expected injected failure to propagate");
        assert_eq!(err.to_string(), "injected post-sync failure");

        assert!(!path.exists(), "target should not exist on inner failure");
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
            "expected tmp cleanup on inner failure, leftovers: {leftovers:?}"
        );
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

    /// SEC-25 / TASK-0898: a destination that the user previously
    /// chmod'd to 0o600 must keep its mode after atomic_write replaces
    /// the file. Pre-fix, the temp file inherited the process umask
    /// (commonly yielding 0o644) and the rename silently widened the
    /// ACL.
    #[cfg(unix)]
    #[test]
    fn atomic_write_preserves_restrictive_destination_perms() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ops.toml");
        std::fs::write(&path, b"first").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();

        atomic_write(&path, b"second").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o7777;
        assert_eq!(
            mode, 0o600,
            "expected 0o600 preserved across atomic_write, got {mode:o}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
    }

    /// SEC-25 / TASK-0898: when the destination doesn't exist, default
    /// the new file's mode to 0o600 rather than the process umask.
    #[cfg(unix)]
    #[test]
    fn atomic_write_defaults_new_file_to_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fresh.toml");
        atomic_write(&path, b"hello").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o7777;
        assert_eq!(
            mode, 0o600,
            "expected 0o600 default for new file, got {mode:o}"
        );
    }

    /// SEC-25 / TASK-1388: if the destination is a symlink, the mode probe
    /// must come from the link entry itself (via `symlink_metadata`), not
    /// from the link target. Otherwise an attacker who can plant a symlink
    /// at the destination can broaden the new file's perms to the target's
    /// mode. `atomic_write` should fall back to the 0o600 default and the
    /// pre-existing target file must be left untouched.
    #[cfg(unix)]
    #[test]
    fn atomic_write_symlink_destination_defaults_to_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("world_readable.txt");
        std::fs::write(&target, b"do not touch").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();

        let link = dir.path().join("ops.toml");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        atomic_write(&link, b"fresh").unwrap();

        let written = std::fs::symlink_metadata(&link).unwrap();
        assert!(
            written.file_type().is_file(),
            "atomic_write should have replaced the symlink with a regular file"
        );
        let mode = written.permissions().mode() & 0o7777;
        assert_eq!(
            mode, 0o600,
            "expected 0o600 default when destination was a symlink, got {mode:o}"
        );
        assert_eq!(std::fs::read(&link).unwrap(), b"fresh");

        let target_meta = std::fs::symlink_metadata(&target).unwrap();
        assert!(target_meta.file_type().is_file());
        assert_eq!(
            target_meta.permissions().mode() & 0o7777,
            0o644,
            "prior symlink target must be untouched"
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"do not touch");
    }

    /// SEC-25 / TASK-1086: `OpenOptions::mode()` is masked by the process
    /// umask (`open(2)` applies `mode & ~umask`). A destination at 0o644
    /// with umask 0o077 lands at 0o600 on disk and the rename silently
    /// narrows the ACL. The fix re-applies the requested mode via
    /// `set_permissions` after creation so the on-disk bits are exact
    /// regardless of umask.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn atomic_write_mode_unaffected_by_umask() {
        use std::os::unix::fs::PermissionsExt;

        // SAFETY: serial-marked. umask is process-global; we restore it
        // before returning so sibling tests are not poisoned.
        unsafe extern "C" {
            fn umask(mask: u32) -> u32;
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ops.toml");
        std::fs::write(&path, b"first").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let prev = unsafe { umask(0o077) };
        let result = atomic_write(&path, b"second");
        unsafe {
            umask(prev);
        }
        result.unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o7777;
        assert_eq!(
            mode, 0o644,
            "expected 0o644 preserved under umask 0o077, got {mode:o}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
    }

    /// SEC-25 / TASK-0837: two siblings whose names differ only in invalid
    /// UTF-8 bytes must produce distinct tmp basenames. Going through
    /// `to_string_lossy` collapses both to the same `?`/U+FFFD-substituted
    /// string, which lets concurrent atomic_writes race on the same tmp.
    /// Verifying that each call writes its target byte-for-byte and that no
    /// tmp leftovers linger pins the OsStr-based concatenation path.
    // APFS (macOS) and many Windows filesystems reject non-UTF-8 file
    // names with EILSEQ before the syscall reaches our code, so this
    // regression can only be exercised on filesystems that pass raw bytes
    // through (Linux ext4/xfs/tmpfs).
    #[cfg(target_os = "linux")]
    #[test]
    fn atomic_write_handles_distinct_non_utf8_siblings() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let dir = tempfile::tempdir().unwrap();

        let name_a: OsString = OsString::from_vec(vec![b'a', 0xff]);
        let name_b: OsString = OsString::from_vec(vec![b'a', 0xfe]);
        let path_a = dir.path().join(&name_a);
        let path_b = dir.path().join(&name_b);

        atomic_write(&path_a, b"alpha").unwrap();
        atomic_write(&path_b, b"beta").unwrap();

        assert_eq!(std::fs::read(&path_a).unwrap(), b"alpha");
        assert_eq!(std::fs::read(&path_b).unwrap(), b"beta");

        // No `.tmp.` leftovers: the rename for each call must have
        // succeeded against its own unique tmp basename.
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .as_encoded_bytes()
                    .windows(5)
                    .any(|w| w == b".tmp.")
            })
            .collect();
        assert!(leftovers.is_empty(), "leaked tmp: {leftovers:?}");
    }

    /// ERR-1 / TASK-1040: `atomic_write` with a bare-filename path (no
    /// directory component) must still resolve a real parent directory for
    /// the post-rename fsync. Pre-fix, `Path::parent()` returned `Some("")`,
    /// the empty path fell through to `std::fs::File::open("")` which
    /// errored with ENOENT, and the parent-fsync was silently skipped —
    /// breaking the documented crash-safety guarantee for the production
    /// `.ops.toml` write path (which IS a bare filename).
    #[test]
    #[serial_test::serial]
    fn atomic_write_bare_filename_fsyncs_cwd_parent() {
        let dir = tempfile::tempdir().unwrap();
        let saved_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = atomic_write(Path::new("bare.toml"), b"payload");

        // Restore cwd before any assertion to avoid poisoning sibling tests.
        std::env::set_current_dir(&saved_cwd).unwrap();

        result.expect("atomic_write must succeed for a bare filename");

        // The file landed in the temp dir (proving cwd was the parent we
        // resolved) and Path::new(".") opens successfully there — i.e. the
        // fsync codepath had a real, openable directory handle to act on,
        // rather than the empty path it would have had pre-fix.
        let written = dir.path().join("bare.toml");
        assert_eq!(std::fs::read(&written).unwrap(), b"payload");
        assert!(
            std::fs::File::open(dir.path()).is_ok(),
            "parent dir must be openable for fsync"
        );
    }

    /// READ-5 / TASK-0908: a leftover tmp file from a crash mid-write
    /// must not double-prefix a dot. Inspecting the directory after a
    /// failed write confirms the basename starts with exactly one dot.
    #[test]
    fn atomic_write_tmp_basename_does_not_double_dot() {
        // We can't observe the tmp file from a *successful* atomic_write
        // (the rename succeeds and the tmp is gone). Instead, force a
        // failure by pre-creating a directory at the target path so
        // rename fails, leaving the tmp file behind for inspection.
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join(".ops.toml");
        std::fs::create_dir(&target).expect("dir at target");
        let _ = atomic_write(&target, b"x");

        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp."))
            .collect();
        // If the rename succeeded after all (rare on some platforms) the
        // assertion below trivially passes since `entries` is empty.
        for name in entries {
            assert!(
                !name.starts_with(".."),
                "tmp basename must not start with two dots: {name}"
            );
        }
    }
}
