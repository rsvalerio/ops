//! `git_info` data provider.

use std::path::Path;

use ops_extension::{Context, DataProvider, DataProviderError, DataProviderSchema};
use serde::Serialize;

use crate::config;
use crate::remote::{parse_remote_url, RemoteInfo};

pub const DATA_PROVIDER_NAME: &str = "git_info";

#[derive(Debug, Clone, Default, Serialize)]
pub struct GitInfo {
    pub host: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub remote_url: Option<String>,
    pub branch: Option<String>,
}

impl GitInfo {
    /// Collect git metadata for the given working directory. Always succeeds;
    /// missing data is represented by `None` fields.
    pub fn collect(cwd: &Path) -> Self {
        let Some(git_dir) = config::find_git_dir(cwd) else {
            return Self::default();
        };
        let branch = config::read_head_branch(&git_dir);
        let Some(raw) = config::read_origin_url(&git_dir) else {
            return Self {
                branch,
                ..Self::default()
            };
        };
        let parsed: Option<RemoteInfo> = parse_remote_url(&raw);
        Self {
            host: parsed.as_ref().map(|r| r.host.clone()),
            owner: parsed.as_ref().map(|r| r.owner.clone()),
            repo: parsed.as_ref().map(|r| r.repo.clone()),
            remote_url: parsed.as_ref().map(|r| r.url.clone()).or(Some(raw)),
            branch,
        }
    }
}

pub struct GitInfoProvider;

impl DataProvider for GitInfoProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let info = GitInfo::collect(&ctx.working_directory);
        serde_json::to_value(&info).map_err(DataProviderError::from)
    }

    fn schema(&self) -> DataProviderSchema {
        use ops_extension::data_field;
        DataProviderSchema {
            description: "Git repository metadata (remote URL, owner/repo, current branch)",
            fields: vec![
                data_field!("host", "Option<String>", "Remote host (e.g. github.com)"),
                data_field!("owner", "Option<String>", "Owner/organization segment"),
                data_field!("repo", "Option<String>", "Repository name"),
                data_field!(
                    "remote_url",
                    "Option<String>",
                    "Normalized https URL for the origin remote"
                ),
                data_field!(
                    "branch",
                    "Option<String>",
                    "Current branch, or None if HEAD is detached"
                ),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_no_git_dir() {
        let dir = tempfile::tempdir().unwrap();
        let info = GitInfo::collect(dir.path());
        assert!(info.host.is_none());
        assert!(info.owner.is_none());
        assert!(info.repo.is_none());
        assert!(info.remote_url.is_none());
        assert!(info.branch.is_none());
    }

    #[test]
    fn collect_full_github_remote() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = https://github.com/openbao/openbao.git\n",
        )
        .unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let info = GitInfo::collect(dir.path());
        assert_eq!(info.host.as_deref(), Some("github.com"));
        assert_eq!(info.owner.as_deref(), Some("openbao"));
        assert_eq!(info.repo.as_deref(), Some("openbao"));
        assert_eq!(
            info.remote_url.as_deref(),
            Some("https://github.com/openbao/openbao")
        );
        assert_eq!(info.branch.as_deref(), Some("main"));
    }

    #[test]
    fn collect_branch_without_remote() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/dev\n").unwrap();

        let info = GitInfo::collect(dir.path());
        assert_eq!(info.branch.as_deref(), Some("dev"));
        assert!(info.remote_url.is_none());
        assert!(info.host.is_none());
    }

    #[test]
    fn collect_unparseable_remote_preserves_raw_url() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = weird-value-without-shape\n",
        )
        .unwrap();

        let info = GitInfo::collect(dir.path());
        assert_eq!(
            info.remote_url.as_deref(),
            Some("weird-value-without-shape")
        );
        assert!(info.host.is_none());
    }

    #[test]
    fn provider_name() {
        assert_eq!(GitInfoProvider.name(), "git_info");
    }

    #[test]
    fn provider_provides_json() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = git@github.com:o/r.git\n",
        )
        .unwrap();

        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let v = GitInfoProvider.provide(&mut ctx).unwrap();
        assert_eq!(
            v.get("remote_url").and_then(|s| s.as_str()),
            Some("https://github.com/o/r")
        );
        assert_eq!(v.get("host").and_then(|s| s.as_str()), Some("github.com"));
    }
}
