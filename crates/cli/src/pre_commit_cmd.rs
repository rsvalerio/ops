//! CLI handler for `ops pre-commit` subcommands.

use std::io::Write;

use ops_pre_commit::{ensure_config_command, find_git_dir, install_hook};

pub fn run_pre_commit_install() -> anyhow::Result<()> {
    run_pre_commit_install_to(&mut std::io::stdout())
}

fn run_pre_commit_install_to(w: &mut dyn Write) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let git_dir = find_git_dir(&cwd)
        .ok_or_else(|| anyhow::anyhow!("not inside a git repository (no .git found)"))?;

    install_hook(&git_dir, w)?;
    ensure_config_command(&cwd, w)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CwdGuard;

    #[test]
    fn install_creates_hook_and_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

        let mut buf = Vec::new();
        run_pre_commit_install_to(&mut buf).expect("install");

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Installed hook"));
        assert!(output.contains("Added default"));

        // Hook exists and is executable
        let hook_path = dir.path().join(".git/hooks/pre-commit");
        assert!(hook_path.exists());

        // Config exists with pre-commit command
        let config = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(config.contains("[commands.pre-commit]"));
    }

    #[test]
    fn install_no_git_dir_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

        let mut buf = Vec::new();
        let result = run_pre_commit_install_to(&mut buf);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not inside a git"));
    }
}
