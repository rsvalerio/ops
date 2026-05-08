//! `git_info` data provider.

use std::path::Path;

use ops_extension::{Context, DataProvider, DataProviderError, DataProviderSchema};
use serde::Serialize;

use crate::config;
use crate::remote::{parse_remote_url, RemoteInfo};

pub const DATA_PROVIDER_NAME: &str = "git_info";

#[derive(Debug, Clone, Default, Serialize)]
#[non_exhaustive]
pub struct GitInfo {
    pub host: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub remote_url: Option<String>,
    pub branch: Option<String>,
}

/// Resolve a project repository URL by preferring the manifest-declared value
/// (when non-empty) and falling back to the local git remote. Shared helper so
/// every language's `project_identity` provider applies the same precedence.
#[must_use]
pub fn resolve_repository_with_git_fallback(
    cwd: &Path,
    manifest_repo: Option<String>,
) -> Option<String> {
    manifest_repo
        .filter(|s| !s.is_empty())
        .or_else(|| GitInfo::collect(cwd).remote_url)
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
        // ARCH-2 / TASK-0894: `raw` is a `RedactedUrl` so the type
        // system already guarantees userinfo has been stripped. Pull
        // the inner str only at the parse boundary.
        match parse_remote_url(raw.as_str()) {
            Some(RemoteInfo {
                host,
                owner,
                repo,
                url,
            }) => Self {
                host: Some(host),
                owner: Some(owner),
                repo: Some(repo),
                remote_url: Some(url),
                branch,
            },
            None => {
                // PATTERN-1 (TASK-0863): the parser intentionally rejects
                // bracketed-IPv6 hosts and other shapes outside the reg-name
                // allowlist. Without this breadcrumb the about card silently
                // loses host/owner/repo with no operator clue why; debug-level
                // keeps it out of the default log volume while remaining
                // discoverable when someone goes looking.
                tracing::debug!(
                    raw_remote = %raw.as_str(),
                    "git remote URL did not match parse_remote_url shape; host/owner/repo will be omitted"
                );
                Self {
                    remote_url: Some(raw.into_string()),
                    branch,
                    ..Self::default()
                }
            }
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
        DataProviderSchema::new(
            "Git repository metadata (remote URL, owner/repo, current branch)",
            vec![
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
        )
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
    fn collect_unparseable_remote_strips_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        // Trailing `/extra/segment/path` keeps parse_remote_url from succeeding
        // — we still must scrub the user:token authority from the fallback.
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = https://user:tok@host.example/weird\n",
        )
        .unwrap();

        let info = GitInfo::collect(dir.path());
        let url = info.remote_url.expect("remote_url");
        assert!(!url.contains("user:tok"), "url leaked credentials: {url}");
        assert!(!url.contains('@'), "url retained userinfo: {url}");
    }

    /// SEC-2 / TASK-1102: a `.git/config` whose `url = ...` value contains
    /// ASCII control bytes (raw newline, ANSI escape) must not surface
    /// those bytes through `info.remote_url`. The redaction layer drops
    /// the line entirely, so the fallback at `GitInfo::collect` never
    /// observes the poisoned value and `remote_url` ends up `None`.
    #[test]
    fn collect_drops_remote_url_with_control_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        // Write the config bytes directly so the embedded ANSI escape +
        // raw newline survive into the file. A newline inside the value
        // would normally end the line, so we use only the ESC byte here
        // and pin the post-newline `fake` content as a separate line that
        // the parser should not pick up either.
        let cfg = "[remote \"origin\"]\n\turl = https://host/repo\u{1b}[31m fake\n";
        std::fs::write(git_dir.join("config"), cfg.as_bytes()).unwrap();

        let info = GitInfo::collect(dir.path());
        let url = info.remote_url.clone().unwrap_or_default();
        assert!(
            !url.contains('\n'),
            "remote_url leaked raw newline: {url:?}"
        );
        assert!(
            !url.contains('\u{1b}'),
            "remote_url leaked ANSI escape: {url:?}"
        );
        assert!(
            info.remote_url.is_none(),
            "control-byte url= must be dropped, got: {:?}",
            info.remote_url
        );
    }

    /// OWN-8 / TASK-0785 AC#2: pin that `read_origin_url_from` already returns
    /// redacted values, so the provider's fallback branch can trust it without
    /// re-redacting. If this test breaks, the provider must add redaction back.
    #[test]
    fn read_origin_url_from_already_redacts_credentials() {
        let cfg = "[remote \"origin\"]\n\turl = https://user:secret@host.example/repo.git\n";
        let url = config::read_origin_url_from(cfg).expect("url");
        let s = url.as_str();
        assert!(
            !s.contains("secret"),
            "read_origin_url_from must redact before returning, got: {s}"
        );
        assert!(
            !s.contains("user@"),
            "read_origin_url_from must strip userinfo, got: {s}"
        );
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
