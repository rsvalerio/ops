//! Git hook file installation.
//!
//! Writes the hook script into `<git_dir>/hooks/<filename>`, canonicalising
//! the target and refusing symlinked or out-of-tree destinations. Idempotent
//! when an ops-installed hook already matches; upgrades legacy ops hooks via
//! a temp-file + atomic rename to close the read/write TOCTOU window.

use std::fs::{File, OpenOptions};
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

    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&hook_path)
    {
        Ok(file) => write_new_hook(file, &hook_path, config, w),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            handle_existing_hook(&hook_path, config, w)
        }
        Err(e) => Err(e).context("failed to create hook"),
    }
}

fn write_new_hook(
    mut file: File,
    hook_path: &Path,
    config: &HookConfig,
    w: &mut dyn Write,
) -> anyhow::Result<PathBuf> {
    file.write_all(config.hook_script.as_bytes())
        .context("failed to write hook")?;
    // Mirror write_temp_hook's durability: if the system crashes between
    // install and the next git invocation, fsync prevents a zero-byte hook.
    file.sync_all().context("failed to fsync hook")?;
    drop(file);
    set_hook_executable(hook_path)?;
    writeln!(w, "Installed hook at {}", hook_path.display())?;
    Ok(hook_path.to_path_buf())
}

fn handle_existing_hook(
    hook_path: &Path,
    config: &HookConfig,
    w: &mut dyn Write,
) -> anyhow::Result<PathBuf> {
    let existing = std::fs::read_to_string(hook_path).context("failed to read existing hook")?;
    if existing == config.hook_script {
        writeln!(w, "Hook already installed at {}", hook_path.display())?;
        return Ok(hook_path.to_path_buf());
    }
    if !has_legacy_marker(&existing, config) {
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
    upgrade_legacy_hook(hook_path, config, w)
}

fn has_legacy_marker(content: &str, config: &HookConfig) -> bool {
    config
        .legacy_markers
        .iter()
        .any(|marker| content.contains(marker))
}

/// Replace a legacy ops hook with the current script via a sibling temp file
/// and an atomic rename.
///
/// SEC-25: the previous implementation `read_to_string` → `fs::write` left a
/// race window in which a user-authored hook could be written between the
/// marker check and the overwrite. We now (a) stage the new content in a temp
/// file created with `create_new`, (b) re-read the original and re-verify the
/// legacy marker right before the rename, and (c) `rename(2)` over the target
/// (atomic on POSIX). The remaining window is a single rename call.
fn upgrade_legacy_hook(
    hook_path: &Path,
    config: &HookConfig,
    w: &mut dyn Write,
) -> anyhow::Result<PathBuf> {
    let parent = hook_path
        .parent()
        .context("hook path has no parent directory")?;
    let file_name = hook_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("hook path has no filename")?;
    let tmp_path = parent.join(format!(".{file_name}.ops-tmp"));

    write_temp_hook(&tmp_path, config).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp_path);
    })?;

    let recheck = std::fs::read_to_string(hook_path)
        .context("failed to re-read existing hook before upgrade")?;
    if !has_legacy_marker(&recheck, config) {
        let _ = std::fs::remove_file(&tmp_path);
        anyhow::bail!(
            "refusing to upgrade {}: file changed during install and no longer \
             looks like an ops-installed hook",
            hook_path.display()
        );
    }

    writeln!(w, "Updating outdated ops hook at {}", hook_path.display())?;
    std::fs::rename(&tmp_path, hook_path).context("failed to rename temp hook into place")?;
    Ok(hook_path.to_path_buf())
}

fn write_temp_hook(tmp_path: &Path, config: &HookConfig) -> anyhow::Result<()> {
    let mut tmp = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(tmp_path)
        .with_context(|| format!("failed to create temp hook file {}", tmp_path.display()))?;
    tmp.write_all(config.hook_script.as_bytes())
        .context("failed to write temp hook")?;
    tmp.sync_all().context("failed to fsync temp hook")?;
    drop(tmp);
    set_hook_executable(tmp_path)?;
    Ok(())
}

fn set_hook_executable(_path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(_path, std::fs::Permissions::from_mode(0o755))
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
        // Temp file is cleaned up.
        let tmp = hooks.join(".pre-commit.ops-tmp");
        assert!(!tmp.exists(), "temp file should be removed on bail");
    }
}
