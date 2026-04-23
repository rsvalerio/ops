//! Shared logic for git hook extensions (run-before-commit, run-before-push).
//!
//! Both hook crates share identical control flow differing only in constants
//! (hook filename, env var name, legacy markers, help text). This crate
//! extracts those common functions behind a [`HookConfig`] descriptor.

use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;

/// Describes one git-hook extension so the shared helpers know which file to
/// create, which env var to check, etc.
///
/// Marked `#[non_exhaustive]`: out-of-crate code must use [`HookConfig::new`]
/// (or the [`crate::impl_hook_wrappers!`] macro that wraps it) so adding new
/// fields stays a non-breaking change.
#[non_exhaustive]
pub struct HookConfig {
    /// Command name, e.g. `"run-before-commit"`.
    pub name: &'static str,
    /// Git hook filename inside `.git/hooks/`, e.g. `"pre-commit"`.
    pub hook_filename: &'static str,
    /// The full hook script to install.
    pub hook_script: &'static str,
    /// Environment variable that, when set to `"1"`, skips execution.
    pub skip_env_var: &'static str,
    /// Substrings in an existing hook that mark it as a legacy ops hook
    /// (will be overwritten).
    pub legacy_markers: &'static [&'static str],
    /// Help text written into the TOML command entry.
    pub command_help: &'static str,
}

impl HookConfig {
    /// Build a `HookConfig` from its parts. Use this instead of struct-literal
    /// construction so adding fields here stays backwards compatible.
    ///
    /// One arg per field: the constructor mirrors the struct's shape so the
    /// macro that drives it stays a flat declarative description.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        name: &'static str,
        hook_filename: &'static str,
        hook_script: &'static str,
        skip_env_var: &'static str,
        legacy_markers: &'static [&'static str],
        command_help: &'static str,
    ) -> Self {
        Self {
            name,
            hook_filename,
            hook_script,
            skip_env_var,
            legacy_markers,
            command_help,
        }
    }
}

/// Returns `true` if the skip env var is set to `"1"`.
pub fn should_skip(config: &HookConfig) -> bool {
    std::env::var(config.skip_env_var).is_ok_and(|v| v == "1")
}

/// Generate the per-extension hook wrappers (`HOOK_CONFIG`, `should_skip`,
/// `find_git_dir`, `install_hook`, `ensure_config_command`) from a single
/// declarative description. Keeps the two hook extension crates in lockstep.
#[macro_export]
macro_rules! impl_hook_wrappers {
    (
        name: $name:expr,
        hook_filename: $hook_filename:expr,
        hook_script: $hook_script:expr,
        skip_env_var: $skip_env_var:expr,
        legacy_markers: $legacy_markers:expr,
        command_help: $command_help:expr $(,)?
    ) => {
        pub const HOOK_CONFIG: $crate::HookConfig = $crate::HookConfig::new(
            $name,
            $hook_filename,
            $hook_script,
            $skip_env_var,
            $legacy_markers,
            $command_help,
        );

        pub fn hook_config() -> $crate::HookConfig {
            HOOK_CONFIG
        }

        pub fn should_skip() -> bool {
            $crate::should_skip(&HOOK_CONFIG)
        }

        pub fn find_git_dir(from: &::std::path::Path) -> Option<::std::path::PathBuf> {
            $crate::find_git_dir(from)
        }

        pub fn install_hook(
            git_dir: &::std::path::Path,
            w: &mut dyn ::std::io::Write,
        ) -> ::anyhow::Result<::std::path::PathBuf> {
            $crate::install_hook(&HOOK_CONFIG, git_dir, w)
        }

        pub fn ensure_config_command(
            config_dir: &::std::path::Path,
            selected_commands: &[String],
            w: &mut dyn ::std::io::Write,
        ) -> ::anyhow::Result<()> {
            $crate::ensure_config_command(&HOOK_CONFIG, config_dir, selected_commands, w)
        }
    };
}

/// Maximum number of parent directories to walk while searching for `.git`.
/// Bounds the loop so a hostile cwd cannot force us to ascend to `/` repeatedly.
const FIND_GIT_DIR_MAX_DEPTH: usize = 64;

/// Find the `.git` directory by walking up from the given path.
///
/// Handles three cases:
/// 1. Plain repos: `.git` is a real directory (symlinked `.git` is rejected).
/// 2. Worktrees / submodules: `.git` is a regular file with body
///    `gitdir: <path>`. The path is resolved relative to the working copy root
///    and returned.
/// 3. Otherwise walks up to the parent, up to [`FIND_GIT_DIR_MAX_DEPTH`] times.
///
/// Symlinked `.git` entries are deliberately skipped: callers like the hook
/// installer write into this directory and a redirected symlink is a
/// supply-chain risk. The returned path is canonicalised so downstream
/// consumers see a stable, real location.
///
/// There is no caller-supplied root ceiling — the depth limit serves as the
/// bound. Pass an already-canonicalised input if the caller has a stricter
/// containment requirement.
pub fn find_git_dir(from: &Path) -> Option<PathBuf> {
    let mut dir = from.to_path_buf();
    for _ in 0..FIND_GIT_DIR_MAX_DEPTH {
        let candidate = dir.join(".git");
        let meta = std::fs::symlink_metadata(&candidate).ok();
        if let Some(meta) = meta {
            if meta.file_type().is_symlink() {
                // Skip silently — never trust a symlinked .git for writes.
            } else if meta.is_dir() {
                return std::fs::canonicalize(&candidate).ok().or(Some(candidate));
            } else if meta.is_file() {
                if let Some(resolved) = read_gitdir_pointer(&candidate) {
                    return std::fs::canonicalize(&resolved).ok().or(Some(resolved));
                }
            }
        }
        if !dir.pop() {
            return None;
        }
    }
    None
}

fn read_gitdir_pointer(file: &Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(file).ok()?;
    let rest = content.lines().find_map(|l| l.strip_prefix("gitdir:"))?;
    let target = Path::new(rest.trim());
    let resolved = if target.is_absolute() {
        target.to_path_buf()
    } else {
        file.parent()?.join(target)
    };
    Some(resolved)
}

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
        Ok(mut file) => {
            file.write_all(config.hook_script.as_bytes())
                .context("failed to write hook")?;
            drop(file);
            set_hook_executable(&hook_path)?;
            writeln!(w, "Installed hook at {}", hook_path.display())?;
            Ok(hook_path)
        }
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            let existing =
                std::fs::read_to_string(&hook_path).context("failed to read existing hook")?;
            if existing == config.hook_script {
                writeln!(w, "Hook already installed at {}", hook_path.display())?;
                return Ok(hook_path);
            }
            if !config
                .legacy_markers
                .iter()
                .any(|marker| existing.contains(marker))
            {
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
            // Accepted race: between the read above and the write below another
            // process could replace the file. Tolerated because the
            // legacy-marker branch is explicitly an overwrite-and-update path.
            writeln!(w, "Updating outdated ops hook at {}", hook_path.display())?;
            std::fs::write(&hook_path, config.hook_script).context("failed to write hook")?;
            set_hook_executable(&hook_path)?;
            Ok(hook_path)
        }
        Err(e) => Err(e).context("failed to create hook"),
    }
}

fn canonical_subdir(parent: &Path, child: &Path) -> anyhow::Result<PathBuf> {
    let canonical = std::fs::canonicalize(child)
        .with_context(|| format!("failed to canonicalize {}", child.display()))?;
    let symlink_meta = std::fs::symlink_metadata(child)
        .with_context(|| format!("failed to stat {}", child.display()))?;
    if symlink_meta.file_type().is_symlink() {
        anyhow::bail!("refusing to install hook: {} is a symlink", child.display());
    }
    if !canonical.starts_with(parent) {
        anyhow::bail!(
            "refusing to install hook: {} resolves outside {}",
            canonical.display(),
            parent.display()
        );
    }
    Ok(canonical)
}

fn canonical_git_dir(git_dir: &Path) -> anyhow::Result<PathBuf> {
    let canonical = std::fs::canonicalize(git_dir)
        .with_context(|| format!("failed to canonicalize git_dir {}", git_dir.display()))?;
    if !is_accepted_git_dir(&canonical) {
        anyhow::bail!(
            "refusing to install hook: {} is not a .git directory or worktree gitdir",
            canonical.display()
        );
    }
    Ok(canonical)
}

fn is_accepted_git_dir(path: &Path) -> bool {
    // Plain `.git` directory.
    if path.file_name().is_some_and(|n| n == ".git") {
        return true;
    }
    // Worktree gitdir: `<repo>/.git/worktrees/<name>`.
    let parent = path.parent();
    let grandparent = parent.and_then(Path::parent);
    parent.is_some_and(|p| p.file_name().is_some_and(|n| n == "worktrees"))
        && grandparent.is_some_and(|g| g.file_name().is_some_and(|n| n == ".git"))
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

/// Ensure a `[commands.<name>]` entry exists in `.ops.toml`.
///
/// If the config already has the command, does nothing.
/// Otherwise, adds a composite command that runs the given `selected_commands`.
/// If `selected_commands` is empty, skips writing the entry.
pub fn ensure_config_command(
    config: &HookConfig,
    config_dir: &Path,
    selected_commands: &[String],
    w: &mut dyn Write,
) -> anyhow::Result<()> {
    if selected_commands.is_empty() {
        writeln!(w, "No commands selected; skipping .ops.toml update")?;
        return Ok(());
    }

    let config_path = config_dir.join(".ops.toml");

    let content = if config_path.exists() {
        std::fs::read_to_string(&config_path).context("failed to read .ops.toml")?
    } else {
        String::new()
    };

    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .unwrap_or_else(|_| toml_edit::DocumentMut::new());

    // Check if command already exists
    if let Some(commands) = doc.get("commands").and_then(|c| c.as_table()) {
        if commands.contains_key(config.name) {
            writeln!(w, "Command '{}' already defined in .ops.toml", config.name)?;
            return Ok(());
        }
    }

    // Ensure [commands] table exists
    if !doc.contains_key("commands") {
        doc["commands"] = toml_edit::Item::Table(toml_edit::Table::new());
    }

    let commands = doc["commands"]
        .as_table_mut()
        .context("commands is not a table")?;

    let mut cmd = toml_edit::Table::new();

    let mut arr = toml_edit::Array::new();
    for name in selected_commands {
        arr.push(name.as_str());
    }
    cmd.insert("commands", toml_edit::value(arr));
    cmd.insert("fail_fast", toml_edit::value(true));
    cmd.insert("help", toml_edit::value(config.command_help));

    commands.insert(config.name, toml_edit::Item::Table(cmd));

    std::fs::write(&config_path, doc.to_string()).context("failed to write .ops.toml")?;
    writeln!(w, "Added '{}' command to .ops.toml", config.name)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit_config() -> HookConfig {
        HookConfig {
            name: "run-before-commit",
            hook_filename: "pre-commit",
            hook_script: "#!/usr/bin/env bash\nexec ops run-before-commit\n",
            skip_env_var: "SKIP_OPS_RUN_BEFORE_COMMIT",
            legacy_markers: &[
                "ops run-before-commit",
                "ops before-commit",
                "ops pre-commit",
            ],
            command_help: "Run run-before-commit checks before committing",
        }
    }

    fn push_config() -> HookConfig {
        HookConfig {
            name: "run-before-push",
            hook_filename: "pre-push",
            hook_script: "#!/usr/bin/env bash\nexec ops run-before-push\n",
            skip_env_var: "SKIP_OPS_RUN_BEFORE_PUSH",
            legacy_markers: &["ops run-before-push", "ops before-push"],
            command_help: "Run run-before-push checks before pushing",
        }
    }

    // -- find_git_dir --

    #[test]
    fn find_git_dir_in_current() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git = dir.path().join(".git");
        std::fs::create_dir(&git).unwrap();
        let expected = std::fs::canonicalize(&git).unwrap();
        assert_eq!(find_git_dir(dir.path()), Some(expected));
    }

    #[test]
    fn find_git_dir_in_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git = dir.path().join(".git");
        std::fs::create_dir(&git).unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let expected = std::fs::canonicalize(&git).unwrap();
        assert_eq!(find_git_dir(&sub), Some(expected));
    }

    #[test]
    fn find_git_dir_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = find_git_dir(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn find_git_dir_resolves_worktree_pointer_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let real_gitdir = dir.path().join("worktrees/feature");
        std::fs::create_dir_all(&real_gitdir).unwrap();
        let worktree = dir.path().join("checkout");
        std::fs::create_dir(&worktree).unwrap();
        let pointer = worktree.join(".git");
        std::fs::write(&pointer, format!("gitdir: {}\n", real_gitdir.display())).unwrap();
        let expected = std::fs::canonicalize(&real_gitdir).unwrap();
        assert_eq!(find_git_dir(&worktree), Some(expected));
    }

    #[cfg(unix)]
    #[test]
    fn find_git_dir_skips_symlinked_dot_git() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = dir.path().join("attacker_repo");
        std::fs::create_dir(&outside).unwrap();
        let workspace = dir.path().join("workspace");
        std::fs::create_dir(&workspace).unwrap();
        std::os::unix::fs::symlink(&outside, workspace.join(".git")).unwrap();
        // Symlinked .git is skipped; with no real .git anywhere, the walk fails.
        assert_eq!(find_git_dir(&workspace), None);
    }

    #[test]
    fn find_git_dir_relative_pointer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let worktree = dir.path().join("checkout");
        std::fs::create_dir_all(worktree.join("../actual_gitdir")).unwrap();
        let pointer = worktree.join(".git");
        std::fs::write(&pointer, "gitdir: ../actual_gitdir\n").unwrap();
        let result = find_git_dir(&worktree).expect("should resolve");
        assert!(result.ends_with("actual_gitdir"));
    }

    // -- should_skip --

    #[test]
    #[serial_test::serial]
    fn should_skip_returns_false_by_default() {
        let cfg = commit_config();
        let _guard = EnvGuard::remove(cfg.skip_env_var);
        assert!(!should_skip(&cfg));
    }

    /// RAII guard that restores an env var to its previous value on drop.
    /// Pair with `#[serial_test::serial]` to prevent races with other env-mutating tests.
    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn remove(key: &'static str) -> Self {
            let original = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }

    // -- install_hook (test both configs) --

    #[test]
    fn install_hook_creates_executable_file_commit() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();

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

    // -- ensure_config_command --

    #[test]
    fn ensure_config_creates_command_in_empty_file() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");

        let selected = vec!["verify".to_string()];
        let mut buf = Vec::new();
        ensure_config_command(&cfg, dir.path(), &selected, &mut buf)
            .expect("ensure_config_command");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("[commands.run-before-commit]"));
        assert!(content.contains("verify"));
        assert!(content.contains("fail_fast"));

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Added"));
    }

    #[test]
    fn ensure_config_creates_push_command() {
        let cfg = push_config();
        let dir = tempfile::tempdir().expect("tempdir");

        let selected = vec!["verify".to_string()];
        let mut buf = Vec::new();
        ensure_config_command(&cfg, dir.path(), &selected, &mut buf)
            .expect("ensure_config_command");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("[commands.run-before-push]"));
    }

    #[test]
    fn ensure_config_preserves_existing_command() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(".ops.toml"),
            "[commands.run-before-commit]\ncommands = [\"test\"]\n",
        )
        .unwrap();

        let selected = vec!["verify".to_string()];
        let mut buf = Vec::new();
        ensure_config_command(&cfg, dir.path(), &selected, &mut buf)
            .expect("ensure_config_command");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains(r#"commands = ["test"]"#));

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("already defined"));
    }

    #[test]
    fn ensure_config_appends_to_existing_config() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(".ops.toml"),
            "[output]\ntheme = \"compact\"\n\n[commands.build]\nprogram = \"cargo\"\nargs = [\"build\"]\n",
        )
        .unwrap();

        let selected = vec!["build".to_string(), "test".to_string()];
        let mut buf = Vec::new();
        ensure_config_command(&cfg, dir.path(), &selected, &mut buf)
            .expect("ensure_config_command");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("theme = \"compact\""));
        assert!(content.contains("[commands.build]"));
        assert!(content.contains("[commands.run-before-commit]"));
    }

    #[test]
    fn ensure_config_empty_selection_skips() {
        let cfg = commit_config();
        let dir = tempfile::tempdir().expect("tempdir");

        let mut buf = Vec::new();
        ensure_config_command(&cfg, dir.path(), &[], &mut buf).expect("ensure_config_command");

        assert!(!dir.path().join(".ops.toml").exists());

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("No commands selected"));
    }
}
