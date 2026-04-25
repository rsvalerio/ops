//! CLI handler for `ops run-before-push` subcommands.

use crate::hook_shared::{self, HookOps};

const OPS: HookOps = HookOps {
    hook_name: "run-before-push",
    find_git_dir: ops_run_before_push::find_git_dir,
    install_hook: ops_run_before_push::install_hook,
    ensure_config_command: ops_run_before_push::ensure_config_command,
};

pub fn run_before_push_install() -> anyhow::Result<()> {
    hook_shared::run_hook_install(&OPS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CwdGuard;

    #[test]
    fn install_creates_hook_and_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

        let selected = vec!["verify".to_string()];
        let mut buf = Vec::new();
        hook_shared::run_hook_install_with(&OPS, &selected, &mut buf).expect("install");

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Installed hook"));
        assert!(output.contains("Added"));

        let hook_path = dir.path().join(".git/hooks/pre-push");
        assert!(hook_path.exists());

        let config = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(config.contains("[commands.run-before-push]"));
        assert!(config.contains("verify"));
    }

    #[test]
    fn install_with_empty_selection_skips_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

        let mut buf = Vec::new();
        hook_shared::run_hook_install_with(&OPS, &[], &mut buf).expect("install");

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Installed hook"));
        assert!(output.contains("No commands selected"));

        assert!(dir.path().join(".git/hooks/pre-push").exists());
        assert!(!dir.path().join(".ops.toml").exists());
    }

    #[test]
    fn install_no_git_dir_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

        let mut buf = Vec::new();
        let result = hook_shared::run_hook_install_with(&OPS, &[], &mut buf);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not inside a git"));
    }
}
