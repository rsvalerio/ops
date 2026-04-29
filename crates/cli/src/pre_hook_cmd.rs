//! CLI handlers for `ops run-before-{commit,push}` install subcommands.
//!
//! The two pre-* hooks differ only in their `HookOps` descriptor; install,
//! dispatch, and tests are otherwise identical, so they share this module.

use crate::hook_shared::{self, HookOps};

pub const COMMIT_OPS: HookOps = HookOps {
    hook_name: "run-before-commit",
    find_git_dir: ops_run_before_commit::find_git_dir,
    install_hook: ops_run_before_commit::install_hook,
    ensure_config_command: ops_run_before_commit::ensure_config_command,
};

pub const PUSH_OPS: HookOps = HookOps {
    hook_name: "run-before-push",
    find_git_dir: ops_run_before_push::find_git_dir,
    install_hook: ops_run_before_push::install_hook,
    ensure_config_command: ops_run_before_push::ensure_config_command,
};

pub fn run_before_commit_install(config: &ops_core::config::Config) -> anyhow::Result<()> {
    hook_shared::run_hook_install(config, &COMMIT_OPS)
}

pub fn run_before_push_install(config: &ops_core::config::Config) -> anyhow::Result<()> {
    hook_shared::run_hook_install(config, &PUSH_OPS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CwdGuard;

    fn setup_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        dir
    }

    fn install_creates_hook_and_config(ops: &HookOps, hook_filename: &str) {
        let dir = setup_repo();
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

        let selected = vec!["verify".to_string()];
        let mut buf = Vec::new();
        hook_shared::run_hook_install_with(ops, &selected, &mut buf).expect("install");

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Installed hook"));
        assert!(output.contains("Added"));

        assert!(dir.path().join(".git/hooks").join(hook_filename).exists());

        let config = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(config.contains(&format!("[commands.{}]", ops.hook_name)));
        assert!(config.contains("verify"));
    }

    fn install_with_empty_selection_skips_config(ops: &HookOps, hook_filename: &str) {
        let dir = setup_repo();
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

        let mut buf = Vec::new();
        hook_shared::run_hook_install_with(ops, &[], &mut buf).expect("install");

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Installed hook"));
        assert!(output.contains("No commands selected"));

        assert!(dir.path().join(".git/hooks").join(hook_filename).exists());
        assert!(!dir.path().join(".ops.toml").exists());
    }

    fn install_no_git_dir_errors(ops: &HookOps) {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

        let mut buf = Vec::new();
        let result = hook_shared::run_hook_install_with(ops, &[], &mut buf);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not inside a git"));
    }

    #[test]
    fn commit_install_creates_hook_and_config() {
        install_creates_hook_and_config(&COMMIT_OPS, "pre-commit");
    }

    #[test]
    fn push_install_creates_hook_and_config() {
        install_creates_hook_and_config(&PUSH_OPS, "pre-push");
    }

    #[test]
    fn commit_install_with_empty_selection_skips_config() {
        install_with_empty_selection_skips_config(&COMMIT_OPS, "pre-commit");
    }

    #[test]
    fn push_install_with_empty_selection_skips_config() {
        install_with_empty_selection_skips_config(&PUSH_OPS, "pre-push");
    }

    #[test]
    fn commit_install_no_git_dir_errors() {
        install_no_git_dir_errors(&COMMIT_OPS);
    }

    #[test]
    fn push_install_no_git_dir_errors() {
        install_no_git_dir_errors(&PUSH_OPS);
    }
}
