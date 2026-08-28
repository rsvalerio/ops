//! Git hook file installation.
//!
//! Writes the hook script into `<git_dir>/hooks/<filename>`, canonicalising
//! the target and refusing symlinked or out-of-tree destinations. Idempotent
//! when an ops-installed hook already matches; both the create and the
//! upgrade path stage the payload in a randomised sibling and rename it into
//! place, so `<git_dir>/hooks/<filename>` is never observable half-written.

use std::fs::File;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::paths::{canonical_git_dir, canonical_subdir};
use crate::HookConfig;

/// Install the git hook described by `config`.
///
/// `git_dir` must be a real `.git` directory or a worktree gitdir
/// (`<repo>/.git/worktrees/<name>`). The path is canonicalized before any
/// writes to defend against symlink redirection, and a non-`.git` target is
/// refused outright.
///
/// Returns the path to the created hook file.
///
/// # Errors
///
/// If the git directory cannot be canonicalized, `.git/hooks` cannot be
/// created, or the hook file cannot be written or made executable.
pub fn install_hook(
    config: &HookConfig,
    git_dir: &Path,
    w: &mut dyn Write,
) -> anyhow::Result<PathBuf> {
    let git_dir = canonical_git_dir(git_dir)?;
    let hooks_dir = git_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir).context("failed to create .git/hooks directory")?;
    let hooks_dir = canonical_subdir(&git_dir, &hooks_dir)?;
    let hook_path = hooks_dir.join(config.hook_filename);
    reject_symlinked_hook(&hook_path)?;

    if let Some(installed) = write_new_hook(&hook_path, config, w)? {
        return Ok(installed);
    }
    handle_existing_hook(&hook_path, config, w)
}

/// SEC-25 (TASK-1892): the hook file is the last component of the write path,
/// and until now the only one with no symlink check — `canonical_git_dir`
/// refuses a symlinked `.git`, `canonical_subdir` a symlinked `hooks/`, and
/// `looks_like_git_dir` a symlinked `HEAD`, each with a test.
///
/// A symlinked `<git_dir>/hooks/<hook>` split the decision from the write:
/// `read_to_string` follows the link, so ops classified the *target's*
/// content, while every write targets the link path. Two divergent outcomes
/// followed. If the target held the current ops script, ops reported "Hook
/// already installed" and exited 0 for a hook whose real body lives outside
/// the repository and can be changed by whoever owns that file — and
/// reinstalling never corrected it, because the idempotent branch returned
/// early every time. If it held a legacy marker, the upgrade path's
/// `rename(2)` (which does not follow symlinks) silently replaced the link
/// with a regular file, destroying an operator's deliberate wiring into a
/// shared hooks directory with no diagnostic beyond "Updating outdated ops
/// hook".
///
/// Writing the symlink requires write access to `.git/hooks` already, so this
/// is not a privilege boundary — but reporting a hook as installed when the
/// file git executes is one ops neither wrote nor can vouch for is a
/// correctness bug on its own.
fn reject_symlinked_hook(hook_path: &Path) -> anyhow::Result<()> {
    if std::fs::symlink_metadata(hook_path).is_ok_and(|m| m.file_type().is_symlink()) {
        anyhow::bail!(
            "refusing to install hook: {} is a symlink",
            hook_path.display()
        );
    }
    Ok(())
}

/// Create the hook for the first time, atomically.
///
/// SEC-25 (TASK-1882): the previous shape opened `hook_path` itself with
/// `OpenOptions::create_new(true)` and wrote the script straight into the
/// live path. A failure (ENOSPC, EIO, EDQUOT) or a kill mid-write left a
/// truncated — most often zero-byte — file at `.git/hooks/<name>`, which git
/// happily executes: an empty hook exits 0, so every subsequent commit passed
/// with no checks and no diagnostic. The create path now mirrors
/// [`upgrade_legacy_hook`]: stage the payload in a randomised sibling, fsync
/// it, make it executable, then link it into place. Nothing is ever written
/// through `hook_path`, so the destination only ever appears complete.
///
/// `persist_noclobber` keeps the exclusive-create semantics `create_new` gave
/// us: it fails with [`ErrorKind::AlreadyExists`] rather than clobbering a
/// hook that appeared concurrently. Returns `Ok(None)` in that case so the
/// caller falls through to [`handle_existing_hook`]; the staged file is
/// unlinked by `NamedTempFile`'s `Drop`.
fn write_new_hook(
    hook_path: &Path,
    config: &HookConfig,
    w: &mut dyn Write,
) -> anyhow::Result<Option<PathBuf>> {
    let tmp = stage_hook_payload(hook_path, config)?;
    match tmp.persist_noclobber(hook_path) {
        Ok(_file) => {
            // SEC-25 (TASK-0713): fsync the parent so the new directory entry
            // survives a power loss. Without this, the inode is durable but
            // the .git/hooks/<name> link can be lost on ext4/xfs, silently
            // disabling the hook even though `ops install` reported success.
            // Mirrors ops_core::config::atomic_write (TASK-0340).
            sync_parent_dir(hook_path);
            writeln!(w, "Installed hook at {}", hook_path.display())?;
            Ok(Some(hook_path.to_path_buf()))
        }
        // A hook appeared between our stage and the link — the same race
        // `create_new` used to report. Hand off to the existing-hook path.
        Err(e) if e.error.kind() == ErrorKind::AlreadyExists => Ok(None),
        Err(e) => Err(anyhow::Error::from(e.error))
            .with_context(|| format!("failed to install hook at {}", hook_path.display())),
    }
}

/// How an already-present `<git_dir>/hooks/<name>` relates to ops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExistingHook {
    /// Byte-identical to the current ops script: nothing to do.
    Current,
    /// Carries a legacy ops marker: upgrade it in place.
    Legacy,
    /// Empty, whitespace-only, or a strict prefix of the current ops script.
    ///
    /// SEC-25 (TASK-1882): this is the shape a pre-TASK-1882 install left
    /// behind when it died mid-write, and the shape a truncating filesystem
    /// error still produces for any writer. It is ops's own artefact, not
    /// user content, so replacing it is safe — and reporting it as a foreign
    /// user-authored hook (which is what the old marker-only check did) told
    /// the operator to "remove it manually" about a file ops wrote itself.
    Partial,
    /// Anything else: user-authored, refuse to touch it.
    Foreign,
}

fn classify_existing_hook(content: &str, config: &HookConfig) -> ExistingHook {
    if content == config.hook_script {
        ExistingHook::Current
    } else if content.trim().is_empty() || config.hook_script.starts_with(content) {
        ExistingHook::Partial
    } else if has_legacy_marker(content, config) {
        ExistingHook::Legacy
    } else {
        ExistingHook::Foreign
    }
}

fn handle_existing_hook(
    hook_path: &Path,
    config: &HookConfig,
    w: &mut dyn Write,
) -> anyhow::Result<PathBuf> {
    let existing = std::fs::read_to_string(hook_path).context("failed to read existing hook")?;
    match classify_existing_hook(&existing, config) {
        ExistingHook::Current => {
            writeln!(w, "Hook already installed at {}", hook_path.display())?;
            Ok(hook_path.to_path_buf())
        }
        ExistingHook::Legacy | ExistingHook::Partial => upgrade_legacy_hook(hook_path, config, w),
        ExistingHook::Foreign => {
            let first_line = existing.lines().next().unwrap_or("").trim();
            anyhow::bail!(
                "a {} hook already exists at {} and was not installed by ops \
                 (first line: {:?}). Remove it manually or back it up before \
                 running install.",
                config.hook_filename,
                hook_path.display(),
                first_line,
            );
        }
    }
}

/// Match a legacy ops marker against the script body.
///
/// PATTERN-1 (TASK-1072 / TASK-1239): match the marker only as a **leading
/// word** of an uncommented line — the line, after `trim_start`, must begin
/// with either:
///
/// * `<marker>` (followed by whitespace or end-of-line), or
/// * `exec <marker>` (followed by whitespace or end-of-line).
///
/// `trim_start` discards leading whitespace; lines whose first non-whitespace
/// char is `#` are skipped as shell comments (TASK-1072). Earlier shapes used
/// `trimmed.contains(marker)` so a string literal inside an `echo`, `printf`,
/// or here-doc body that *mentioned* the marker triggered the upgrade path
/// and clobbered a hand-written user hook (TASK-1239).
///
/// Note: the marker is treated as the verbatim prefix of a command line; it
/// must end at a word boundary so `ops run-before-commit-extra` is not
/// mis-classified for marker `ops run-before-commit`.
fn has_legacy_marker(content: &str, config: &HookConfig) -> bool {
    content.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return false;
        }
        let after_exec = trimmed.strip_prefix("exec ").map(str::trim_start);
        let candidates = [Some(trimmed), after_exec];
        config.legacy_markers.iter().any(|marker| {
            candidates.iter().flatten().any(|head| {
                let Some(rest) = head.strip_prefix(marker) else {
                    return false;
                };
                // Require word boundary after the marker: either EOL or
                // ASCII whitespace. Anything else (e.g. `able` continuing
                // the identifier) is not a marker hit.
                rest.is_empty() || rest.starts_with(|c: char| c.is_ascii_whitespace())
            })
        })
    })
}

/// Replace a legacy ops hook with the current script via a sibling temp file
/// and an atomic rename.
///
/// SEC-25: the previous implementation `read_to_string` → `fs::write` left a
/// race window in which a user-authored hook could be written between the
/// marker check and the overwrite. We now (a) stage the new content in a temp
/// file created with a randomised sibling name, (b) re-read the original and
/// re-verify the legacy marker right before the rename, and (c) `rename(2)`
/// over the target (atomic on POSIX). The remaining window is a single
/// rename call.
///
/// SEC-25 / TASK-1210: the temp-file name is randomised via
/// [`tempfile::NamedTempFile::new_in`] rather than the previous fixed
/// `.{file_name}.ops-tmp` sibling. Concurrent installs against shared
/// worktrees (`.git/worktrees/<name>/hooks/` is shared between checkouts of
/// the same repo) used to race: process A would create the fixed temp path
/// and start writing, process B would observe `AlreadyExists`, fall into the
/// TASK-1113 stale-recovery branch, **delete process A's mid-write temp
/// file**, then create its own. With randomised names the two processes get
/// disjoint paths — both can stage in parallel and the rename serialises
/// (`rename(2)` is atomic on POSIX, so exactly one writer wins the
/// destination and the loser's stage gets cleaned up). The TASK-1113
/// stale-leftover recovery path is therefore no longer needed: a crashed
/// install leaves a randomised orphan, which subsequent installs simply
/// ignore.
fn upgrade_legacy_hook(
    hook_path: &Path,
    config: &HookConfig,
    w: &mut dyn Write,
) -> anyhow::Result<PathBuf> {
    let tmp = stage_hook_payload(hook_path, config)?;

    let recheck = std::fs::read_to_string(hook_path)
        .context("failed to re-read existing hook before upgrade")?;
    let message = match classify_existing_hook(&recheck, config) {
        ExistingHook::Legacy => "Updating outdated ops hook at",
        // SEC-25 (TASK-1882): a truncated ops artefact is replaced, not
        // reported as a foreign hook.
        ExistingHook::Partial => "Replacing partially written ops hook at",
        // A concurrent installer won the race with the identical payload.
        // Nothing left to do: drop the stage and report the same success the
        // idempotent path would have. Erroring here would make two honest
        // simultaneous `ops <hook>-install` runs fail one of themselves.
        ExistingHook::Current => {
            drop(tmp);
            writeln!(w, "Hook already installed at {}", hook_path.display())?;
            return Ok(hook_path.to_path_buf());
        }
        ExistingHook::Foreign => {
            // Drop runs on `tmp` and unlinks the staged file. Bail loudly so
            // the user-authored content stays intact.
            anyhow::bail!(
                "refusing to upgrade {}: file changed during install and no longer \
                 looks like an ops-installed hook",
                hook_path.display()
            );
        }
    };

    writeln!(w, "{message} {}", hook_path.display())?;
    // `persist` consumes the NamedTempFile and renames its randomised
    // path over `hook_path`. On error the inner `(io::Error, NamedTempFile)`
    // pair lets the temp file fall back into Drop, unlinking the stage.
    tmp.persist(hook_path).map_err(|e| {
        anyhow::Error::from(e.error).context("failed to rename temp hook into place")
    })?;
    // SEC-25 (TASK-0713): fsync the parent so the rename hits disk; without
    // this a crash can leave the directory entry pointing at the temp
    // inode (or the old hook) even though the new content is durable.
    sync_parent_dir(hook_path);
    Ok(hook_path.to_path_buf())
}

/// Persist the parent directory entry for `path`. Unix-only; on other
/// platforms this is a no-op (Windows does not require the equivalent for
/// crash safety, and an `open(parent)` there would fail anyway). Errors are
/// logged rather than returned because the install has already succeeded —
/// surfacing a parent-dir fsync failure as a hard error would regress the
/// success path on filesystems that do not support directory fsync.
fn sync_parent_dir(path: &Path) {
    #[cfg(not(unix))]
    let _ = path;
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        match File::open(parent) {
            Ok(dir) => {
                if let Err(e) = dir.sync_all() {
                    // ERR-7 (TASK-0937 / TASK-1886): Debug-format the path and
                    // error so a directory name carrying newlines or ANSI
                    // escapes cannot forge lines in the developer's terminal.
                    tracing::debug!(
                        parent = ?parent.display(),
                        error = ?e,
                        "fsync of hook parent directory failed; install kept",
                    );
                }
            }
            Err(e) => {
                // ERR-7 (TASK-0937 / TASK-1886): see above.
                tracing::debug!(
                    parent = ?parent.display(),
                    error = ?e,
                    "could not open hook parent for fsync; install kept",
                );
            }
        }
    }
}

/// Stage the hook payload in a randomised sibling of `hook_path`, fsynced and
/// already executable, ready to be renamed into place.
///
/// SEC-25 / TASK-1210: the name is randomised rather than a fixed
/// `.{file_name}.ops-tmp` sibling. Prefix with `.` so the partial write is
/// hidden by typical directory listings, and tag with the hook filename so an
/// orphan from a crashed install is recognisable in a post-mortem `ls -la`.
///
/// SEC-25 / TASK-1882: shared by the create and the upgrade path so both get
/// the same crash-safety. On any failure the staged file is unlinked and
/// nothing is left at `hook_path`.
fn stage_hook_payload(
    hook_path: &Path,
    config: &HookConfig,
) -> anyhow::Result<tempfile::NamedTempFile> {
    let parent = hook_path
        .parent()
        .context("hook path has no parent directory")?;
    let file_name = hook_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("hook path has no filename")?;

    let tmp = tempfile::Builder::new()
        .prefix(&format!(".{file_name}.ops-tmp."))
        .tempfile_in(parent)
        .with_context(|| {
            format!(
                "failed to create temp hook in {} for atomic rename",
                parent.display()
            )
        })?;
    let tmp_path = tmp.path().to_path_buf();
    write_hook_payload(tmp.as_file(), &tmp_path, config).inspect_err(|_| {
        // NamedTempFile's Drop unlinks on failure too, but be explicit so
        // a Drop-disabled future refactor cannot silently leave orphans.
        let _ = std::fs::remove_file(&tmp_path);
    })?;
    set_hook_executable(&tmp_path)?;
    Ok(tmp)
}

/// Write the hook script bytes into an already-open temp file and fsync.
/// Caller owns the file handle (typically `tempfile::NamedTempFile::as_file`)
/// so the randomised path stays single-source-of-truth.
fn write_hook_payload(file: &File, tmp_path: &Path, config: &HookConfig) -> anyhow::Result<()> {
    let mut file = file;
    file.write_all(config.hook_script.as_bytes())
        .with_context(|| format!("failed to write temp hook {}", tmp_path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to fsync temp hook {}", tmp_path.display()))?;
    Ok(())
}

fn set_hook_executable(path: &Path) -> anyhow::Result<()> {
    #[cfg(not(unix))]
    let _ = path;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .context("failed to make hook executable")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{commit_config, push_config};

    #[test]
    fn install_hook_creates_executable_file_commit() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&cfg, &git_dir, &mut buf).expect("install_hook");

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("ops run-before-commit"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert!(mode & 0o111 != 0, "hook should be executable");
        }

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Installed hook"));
    }

    #[test]
    fn install_hook_creates_executable_file_push() {
        let cfg = push_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&cfg, &git_dir, &mut buf).expect("install_hook");

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("ops run-before-push"));

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Installed hook"));
    }

    /// SEC-25 / TASK-1882: if the create path fails, nothing may be left at
    /// `.git/hooks/<hook>`. The pre-TASK-1882 shape opened the live hook path
    /// with `create_new` before writing, so any failure downstream of that
    /// open left a zero-byte hook behind — a file git runs and that exits 0,
    /// silently disabling the gate. Staging in a sibling makes the failure
    /// path leave the destination untouched.
    #[cfg(unix)]
    #[test]
    fn install_hook_create_failure_leaves_no_hook_file() {
        use std::os::unix::fs::PermissionsExt;

        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        let hooks = git_dir.join("hooks");
        std::fs::create_dir_all(&hooks).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        // Read+execute but not write: the stage cannot be created, so the
        // install fails at the earliest possible point.
        std::fs::set_permissions(&hooks, std::fs::Permissions::from_mode(0o555)).unwrap();

        let mut buf = Vec::new();
        let result = install_hook(&cfg, &git_dir, &mut buf);

        let hook_path = hooks.join("pre-commit");
        let leftover = hook_path.exists();
        // Restore permissions so tempdir cleanup succeeds.
        std::fs::set_permissions(&hooks, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert!(
            result.is_err(),
            "install must fail on an unwritable hooks dir"
        );
        assert!(
            !leftover,
            "no file — in particular no empty file — may remain at the hook path"
        );
    }

    /// SEC-25 / TASK-1882: an install killed between staging and the rename
    /// leaves a randomised `.pre-commit.ops-tmp.*` orphan and *no* hook. The
    /// next install must ignore the orphan and complete normally.
    #[test]
    fn install_hook_after_interrupted_stage_installs_cleanly() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        let hooks = git_dir.join("hooks");
        std::fs::create_dir_all(&hooks).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        // What an interruption can leave behind: a staged, partially written
        // sibling. Never a partial `pre-commit`.
        let orphan = hooks.join(".pre-commit.ops-tmp.AbCxYz");
        std::fs::write(&orphan, "#!/usr/bin/env bash\nexec ops run-before-com").unwrap();
        assert!(!hooks.join("pre-commit").exists());

        let mut buf = Vec::new();
        let path = install_hook(&cfg, &git_dir, &mut buf).expect("install_hook");

        assert_eq!(std::fs::read_to_string(&path).unwrap(), cfg.hook_script);
        assert!(orphan.exists(), "orphan stage is not ours to remove");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Installed hook"), "unexpected: {output}");
    }

    /// SEC-25 / TASK-1882: a truncated hook is ops's own artefact, not a
    /// user-authored hook. Before the fix the installer read it, found no
    /// legacy marker, and told the operator to remove "a hook not installed
    /// by ops" — about a file ops itself had half-written, wedging reinstall.
    #[test]
    fn install_hook_replaces_truncated_hook_rather_than_calling_it_foreign() {
        let cfg = commit_config();
        for partial in ["", "   \n", "#!/usr/bin/env bash\nexec ops run-before-com"] {
            let dir = tempfile::tempdir().expect("tempdir");
            let git_dir = dir.path().join(".git");
            std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
            std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
            std::fs::write(git_dir.join("hooks/pre-commit"), partial).unwrap();

            let mut buf = Vec::new();
            let path = install_hook(&cfg, &git_dir, &mut buf)
                .unwrap_or_else(|e| panic!("partial hook {partial:?} must be replaced: {e:#}"));

            assert_eq!(std::fs::read_to_string(&path).unwrap(), cfg.hook_script);
            let output = String::from_utf8(buf).unwrap();
            assert!(
                !output.contains("not installed by ops"),
                "partial hook {partial:?} must not be reported as foreign: {output}"
            );
        }
    }

    #[test]
    fn classify_existing_hook_separates_partial_from_foreign() {
        let cfg = commit_config();
        assert_eq!(
            classify_existing_hook(cfg.hook_script, &cfg),
            ExistingHook::Current
        );
        assert_eq!(classify_existing_hook("", &cfg), ExistingHook::Partial);
        assert_eq!(
            classify_existing_hook("#!/usr/bin/env bash\nexec ops run-before-com", &cfg),
            ExistingHook::Partial
        );
        assert_eq!(
            classify_existing_hook("#!/bin/sh\nexec ops before-commit\n", &cfg),
            ExistingHook::Legacy
        );
        assert_eq!(
            classify_existing_hook("#!/bin/sh\necho mine\n", &cfg),
            ExistingHook::Foreign
        );
    }

    #[test]
    fn install_hook_idempotent_when_ops_hook_exists() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(git_dir.join("hooks/pre-commit"), cfg.hook_script).unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&cfg, &git_dir, &mut buf).expect("install_hook");

        assert!(path.exists());
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("already installed"));
    }

    #[test]
    fn install_hook_updates_outdated_ops_hook() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            git_dir.join("hooks/pre-commit"),
            "#!/bin/sh\necho old\nops run-before-commit\n",
        )
        .unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&cfg, &git_dir, &mut buf).expect("install_hook");

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, cfg.hook_script);

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Updating outdated"));
    }

    #[test]
    fn install_hook_updates_legacy_hook() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            git_dir.join("hooks/pre-commit"),
            "#!/bin/sh\nexec ops before-commit\n",
        )
        .unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&cfg, &git_dir, &mut buf).expect("install_hook");

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, cfg.hook_script);

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Updating outdated"));
    }

    /// PATTERN-1 (TASK-1072): a user-authored hook whose only mention of an
    /// ops legacy marker lives inside a shell comment must NOT be classified
    /// as an ops legacy hook. The installer must refuse to overwrite it.
    #[test]
    fn install_hook_refuses_foreign_hook_with_marker_in_comment() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        // The marker appears only in a comment — this is a user-authored
        // hook, not an ops-installed one.
        let user_hook =
            "#!/bin/sh\n# ops legacy marker: do not actually run `ops run-before-commit`\n\
             echo my own checks\n";
        std::fs::write(git_dir.join("hooks/pre-commit"), user_hook).unwrap();

        let mut buf = Vec::new();
        let result = install_hook(&cfg, &git_dir, &mut buf);
        assert!(
            result.is_err(),
            "installer must refuse user hook whose marker is only in a comment"
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not installed by ops"));

        // User's hook content must be preserved verbatim.
        assert_eq!(
            std::fs::read_to_string(git_dir.join("hooks/pre-commit")).unwrap(),
            user_hook
        );
    }

    /// PATTERN-1 (TASK-1239): a user-authored hook that mentions the
    /// marker only inside an `echo`/`printf` argument or a here-doc body
    /// must NOT be classified as an ops legacy hook. The leading-word
    /// contract ensures the marker is matched only when it is the head of
    /// the command line (optionally `exec`-prefixed).
    #[test]
    fn has_legacy_marker_ignores_marker_inside_echo_printf_and_heredoc() {
        let cfg = commit_config();

        let echo_arg = "#!/bin/sh\necho \"Tip: run 'ops run-before-commit' manually\"\n";
        assert!(
            !has_legacy_marker(echo_arg, &cfg),
            "marker inside echo argument must not match"
        );

        let printf_arg =
            "#!/bin/sh\nprintf 'see: ops run-before-commit\\n'\nexec /usr/local/bin/my-checks\n";
        assert!(
            !has_legacy_marker(printf_arg, &cfg),
            "marker inside printf argument must not match"
        );

        let heredoc = "#!/bin/sh\ncat <<'NOTE'\nrun ops run-before-commit when ready\nNOTE\n";
        assert!(
            !has_legacy_marker(heredoc, &cfg),
            "marker inside here-doc body must not match"
        );

        // Word boundary: a marker followed by an identifier continuation is
        // not a hit either.
        let extended_word = "#!/bin/sh\nops run-before-commit-extra\n";
        assert!(
            !has_legacy_marker(extended_word, &cfg),
            "marker followed by identifier continuation must not match"
        );

        // Sanity: real legacy lines still match.
        let bare = "#!/bin/sh\nops run-before-commit\n";
        assert!(has_legacy_marker(bare, &cfg));
        let exec_prefixed = "#!/bin/sh\nexec ops run-before-commit\n";
        assert!(has_legacy_marker(exec_prefixed, &cfg));
        let with_args = "#!/bin/sh\nops run-before-commit --quiet\n";
        assert!(has_legacy_marker(with_args, &cfg));
    }

    /// PATTERN-1 (TASK-1239): integration-level analogue of TASK-1072 — a
    /// user-authored hook whose only mention of the marker lives inside an
    /// `echo` argument must be refused, not silently overwritten.
    #[test]
    fn install_hook_refuses_user_hook_with_marker_in_echo_argument() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        let user_hook = "#!/bin/sh\necho \"Tip: run 'ops run-before-commit' manually\"\n";
        std::fs::write(git_dir.join("hooks/pre-commit"), user_hook).unwrap();

        let mut buf = Vec::new();
        let result = install_hook(&cfg, &git_dir, &mut buf);
        assert!(
            result.is_err(),
            "marker inside echo arg must not trigger upgrade"
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not installed by ops"));

        // User content preserved verbatim.
        assert_eq!(
            std::fs::read_to_string(git_dir.join("hooks/pre-commit")).unwrap(),
            user_hook
        );
    }

    /// PATTERN-1 (TASK-1072): unit-level coverage of `has_legacy_marker`
    /// covering the comment-skip and indented-comment paths.
    #[test]
    fn has_legacy_marker_skips_commented_lines() {
        let cfg = commit_config();
        // Marker only inside a comment — must not match.
        let only_in_comment = "#!/bin/sh\n# ops run-before-commit (legacy note)\necho hi\n";
        assert!(!has_legacy_marker(only_in_comment, &cfg));

        // Indented comment — still must not match.
        let indented_comment = "#!/bin/sh\n    # ops run-before-commit\necho hi\n";
        assert!(!has_legacy_marker(indented_comment, &cfg));

        // Genuine ops hook line — must match.
        let real_hook = "#!/bin/sh\nexec ops run-before-commit\n";
        assert!(has_legacy_marker(real_hook, &cfg));
    }

    #[test]
    fn install_hook_refuses_foreign_hook() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            git_dir.join("hooks/pre-commit"),
            "#!/bin/sh\necho foreign\n",
        )
        .unwrap();

        let mut buf = Vec::new();
        let result = install_hook(&cfg, &git_dir, &mut buf);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not installed by ops"));
    }

    #[cfg(unix)]
    #[test]
    fn install_hook_rejects_symlinked_hooks_dir() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        let target = dir.path().join("evil_hooks");
        std::fs::create_dir(&target).unwrap();
        std::os::unix::fs::symlink(&target, git_dir.join("hooks")).unwrap();

        let mut buf = Vec::new();
        let err = install_hook(&cfg, &git_dir, &mut buf).unwrap_err();
        assert!(
            err.to_string().contains("symlink") || err.to_string().contains("outside"),
            "unexpected: {err}"
        );
    }

    /// SEC-25 / TASK-1892: a symlinked `pre-commit` pointing at a file whose
    /// content *is* the ops script must not be reported as "Hook already
    /// installed" — the body git executes lives outside the repository and is
    /// owned by whoever owns that file.
    #[cfg(unix)]
    #[test]
    fn install_hook_rejects_symlinked_hook_matching_ops_script() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        let outside = dir.path().join("shared-hook");
        std::fs::write(&outside, cfg.hook_script).unwrap();
        let hook_path = git_dir.join("hooks/pre-commit");
        std::os::unix::fs::symlink(&outside, &hook_path).unwrap();

        let mut buf = Vec::new();
        let err = install_hook(&cfg, &git_dir, &mut buf).unwrap_err();

        assert!(
            err.to_string().contains("is a symlink"),
            "unexpected: {err}"
        );
        let output = String::from_utf8(buf).unwrap();
        assert!(
            !output.contains("already installed"),
            "must not claim the hook is installed: {output}"
        );
        assert!(std::fs::symlink_metadata(&hook_path)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    /// SEC-25 / TASK-1892: `rename(2)` does not follow symlinks, so the
    /// upgrade path used to replace a deliberately symlinked hook with a
    /// regular file and report only "Updating outdated ops hook". Refuse
    /// instead, leaving the operator's wiring intact.
    #[cfg(unix)]
    #[test]
    fn install_hook_does_not_replace_symlinked_legacy_hook() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        let outside = dir.path().join("shared-hook");
        let legacy = "#!/bin/sh\nexec ops before-commit\n";
        std::fs::write(&outside, legacy).unwrap();
        let hook_path = git_dir.join("hooks/pre-commit");
        std::os::unix::fs::symlink(&outside, &hook_path).unwrap();

        let mut buf = Vec::new();
        let err = install_hook(&cfg, &git_dir, &mut buf).unwrap_err();

        assert!(
            err.to_string().contains("is a symlink"),
            "unexpected: {err}"
        );
        assert!(
            std::fs::symlink_metadata(&hook_path)
                .unwrap()
                .file_type()
                .is_symlink(),
            "the symlink must not be replaced by a regular file"
        );
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), legacy);
    }

    /// SEC-25 / TASK-0361: HEAD must be a real regular file. A symlinked HEAD
    /// is the simplest swap an attacker can stage between the shape check and
    /// the hook write, so the substance check rejects it outright.
    #[cfg(unix)]
    #[test]
    fn install_hook_rejects_symlinked_head() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        let real_head = dir.path().join("real_head");
        std::fs::write(&real_head, "ref: refs/heads/main\n").unwrap();
        std::os::unix::fs::symlink(&real_head, git_dir.join("HEAD")).unwrap();

        let mut buf = Vec::new();
        let err = install_hook(&cfg, &git_dir, &mut buf).unwrap_err();
        assert!(
            err.to_string().contains("not a .git directory"),
            "unexpected: {err}"
        );
    }

    /// SEC-14: a directory named `.git` that lacks `HEAD` is not a real git
    /// repo. The installer must refuse it so an attacker-controlled path
    /// canonicalising to `.../.git` cannot pass the filename heuristic alone.
    #[test]
    fn install_hook_rejects_bogus_dot_git_without_head() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        // Deliberately no HEAD file — looks like .git only by name.

        let mut buf = Vec::new();
        let err = install_hook(&cfg, &git_dir, &mut buf).unwrap_err();
        assert!(
            err.to_string().contains("not a .git directory"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn install_hook_rejects_non_git_directory() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let bogus = dir.path().join("not_dot_git");
        std::fs::create_dir(&bogus).unwrap();

        let mut buf = Vec::new();
        let err = install_hook(&cfg, &bogus, &mut buf).unwrap_err();
        assert!(
            err.to_string().contains("not a .git directory"),
            "unexpected: {err}"
        );
    }

    /// SEC-25 regression: if the on-disk file is replaced with non-ops
    /// content between the initial legacy-marker check and the rename, the
    /// upgrade path must bail without clobbering the user's content.
    #[test]
    fn upgrade_legacy_hook_bails_if_file_replaced_after_initial_check() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let hooks = dir.path().join("hooks");
        std::fs::create_dir(&hooks).unwrap();
        let hook_path = hooks.join("pre-commit");

        // Simulate the racing writer: by the time we call upgrade_legacy_hook,
        // the file already holds non-ops content. The recheck inside must
        // catch this and refuse to overwrite.
        let foreign = "#!/bin/sh\necho user-authored hook, not ops\n";
        std::fs::write(&hook_path, foreign).unwrap();

        let mut buf = Vec::new();
        let err = upgrade_legacy_hook(&hook_path, &cfg, &mut buf).unwrap_err();
        assert!(
            err.to_string().contains("file changed during install"),
            "unexpected: {err}"
        );

        // User's hook is preserved.
        assert_eq!(std::fs::read_to_string(&hook_path).unwrap(), foreign);
        // TEST-11 (TASK-1888): count randomised stages by prefix, matching
        // the two sibling tests below. The previous assertion probed the
        // pre-TASK-1210 fixed name `.pre-commit.ops-tmp`, which randomised
        // staging can never create — so it held unconditionally and left the
        // bail path's stage cleanup, the only coverage it has, unverified.
        let stray = std::fs::read_dir(&hooks)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(".pre-commit.ops-tmp."))
            .count();
        assert_eq!(stray, 0, "staged temp file must be removed on bail");
    }

    /// ERR-1 (TASK-1113) + SEC-25 (TASK-1210): a prior `ops install` that
    /// crashed between `write_hook_payload` and the rename now leaves a
    /// **randomised** orphan (e.g. `.pre-commit.ops-tmp.AbCxYz`) rather
    /// than the previous fixed `.pre-commit.ops-tmp` sibling. The next
    /// upgrade must succeed regardless of the orphan: with randomised
    /// names there is no collision, so no stale-recovery branch is
    /// needed. We assert the legacy fixed-name file is left untouched
    /// (this is a foreign file from the new code's perspective) and the
    /// install completes with a fresh randomised stage that gets
    /// renamed onto the hook path.
    #[test]
    fn upgrade_legacy_hook_ignores_legacy_fixed_name_orphan() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let hooks = dir.path().join("hooks");
        std::fs::create_dir(&hooks).unwrap();
        let hook_path = hooks.join("pre-commit");

        // Legacy ops hook on disk that should be upgraded.
        std::fs::write(&hook_path, "#!/bin/sh\nexec ops before-commit\n").unwrap();

        // Pre-TASK-1210 orphan: the fixed sibling name a crashed install
        // *used to* leave on disk. With randomised names this is no
        // longer ours; a future cleanup can sweep it, but the install
        // must not block on it.
        let legacy_orphan = hooks.join(".pre-commit.ops-tmp");
        std::fs::write(
            &legacy_orphan,
            "garbage from a previous crashed install (pre-TASK-1210)",
        )
        .unwrap();

        let mut buf = Vec::new();
        let path = upgrade_legacy_hook(&hook_path, &cfg, &mut buf)
            .expect("randomised stage must not collide with the legacy fixed name");

        assert_eq!(path, hook_path);
        let content = std::fs::read_to_string(&hook_path).unwrap();
        assert_eq!(content, cfg.hook_script);
        // Legacy fixed-name orphan is left in place (not ours to remove).
        assert!(
            legacy_orphan.exists(),
            "pre-TASK-1210 fixed-name orphan must not be touched"
        );
        // The randomised stage created by this install was consumed by
        // `persist`; no `.pre-commit.ops-tmp.*` siblings should remain
        // *aside from* the legacy orphan we explicitly seeded.
        let stray_random_stages = std::fs::read_dir(&hooks)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(".pre-commit.ops-tmp."))
            .count();
        assert_eq!(
            stray_random_stages, 0,
            "randomised stage from this install must not leak"
        );

        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("Updating outdated"),
            "unexpected output: {output}"
        );
    }

    /// SEC-25 / TASK-1210 AC #2: two concurrent `upgrade_legacy_hook`
    /// calls against the same `hook_path` must not collide on a fixed
    /// temp-file name. With randomised stages the two writers get
    /// disjoint files and the rename serialises atomically — exactly
    /// one wins and writes the new payload, the other observes the
    /// post-win content and returns the "file changed during install"
    /// typed error from the recheck step. Pre-TASK-1210 the loser
    /// would either (a) hit `AlreadyExists` on the fixed sibling or
    /// (b) **delete the winner's mid-write temp file** via the
    /// stale-recovery branch and clobber the install.
    #[test]
    fn upgrade_legacy_hook_concurrent_callers_do_not_corrupt_install() {
        use std::sync::Arc;
        use std::thread;

        let cfg = Arc::new(commit_config());
        let dir = tempfile::tempdir().expect("tempdir");
        let hooks = dir.path().join("hooks");
        std::fs::create_dir(&hooks).unwrap();
        let hook_path = hooks.join("pre-commit");
        std::fs::write(&hook_path, "#!/bin/sh\nexec ops before-commit\n").unwrap();
        let hook_path = Arc::new(hook_path);

        let n = 8usize;
        let mut handles = Vec::with_capacity(n);
        for _ in 0..n {
            let cfg = Arc::clone(&cfg);
            let hook_path = Arc::clone(&hook_path);
            handles.push(thread::spawn(move || {
                let mut buf = Vec::new();
                upgrade_legacy_hook(&hook_path, &cfg, &mut buf).map(|_| ())
            }));
        }

        let mut wins = 0usize;
        let mut typed_losses = 0usize;
        for h in handles {
            match h.join().expect("thread panicked") {
                Ok(()) => wins += 1,
                Err(e) => {
                    let msg = e.to_string();
                    // The loser of the rename race re-reads the file
                    // (now the new ops payload) and surfaces the
                    // typed "file changed during install" error.
                    assert!(
                        msg.contains("file changed during install")
                            || msg.contains("failed to re-read existing hook"),
                        "unexpected error from concurrent upgrade: {msg}"
                    );
                    typed_losses += 1;
                }
            }
        }
        assert_eq!(
            wins + typed_losses,
            n,
            "every caller must return a typed result, no panics or AlreadyExists leaks"
        );
        assert!(
            wins >= 1,
            "at least one caller must observe a successful upgrade"
        );

        // Final on-disk content is the ops payload (one of the wins
        // committed it).
        assert_eq!(
            std::fs::read_to_string(&*hook_path).unwrap(),
            cfg.hook_script
        );

        // No randomised stage files should remain — every staged file
        // is either renamed (winner) or dropped (loser).
        let stray = std::fs::read_dir(&hooks)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(".pre-commit.ops-tmp."))
            .count();
        assert_eq!(
            stray, 0,
            "no randomised stage may leak after concurrent upgrade"
        );
    }
}
