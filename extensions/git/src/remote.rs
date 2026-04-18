//! Parse git remote URLs into a structured form.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemoteInfo {
    pub host: String,
    pub owner: String,
    pub repo: String,
    /// Normalized https URL (no `.git` suffix, no credentials).
    pub url: String,
}

/// Parse a raw git remote URL into a [`RemoteInfo`].
///
/// Handles three common shapes:
/// - `https://host/owner/repo(.git)?` (may include `user:token@` which we strip)
/// - `git@host:owner/repo(.git)?` (scp-style)
/// - `ssh://[user@]host[:port]/owner/repo(.git)?`
pub fn parse_remote_url(raw: &str) -> Option<RemoteInfo> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    let (host, path) = split_host_and_path(raw)?;
    let (owner, repo) = split_owner_repo(path)?;

    let repo = repo.strip_suffix(".git").unwrap_or(repo);
    if owner.is_empty() || repo.is_empty() {
        return None;
    }

    Some(RemoteInfo {
        host: host.to_string(),
        owner: owner.to_string(),
        repo: repo.to_string(),
        url: format!("https://{host}/{owner}/{repo}"),
    })
}

fn split_host_and_path(raw: &str) -> Option<(&str, &str)> {
    // scp-style: git@host:owner/repo
    if !raw.contains("://") {
        if let Some(at) = raw.find('@') {
            let rest = &raw[at + 1..];
            let colon = rest.find(':')?;
            let host = &rest[..colon];
            let path = &rest[colon + 1..];
            return Some((host, path));
        }
        return None;
    }

    // URL form: scheme://[user@]host[:port]/path
    let after_scheme = raw.split_once("://")?.1;
    let (authority, path) = after_scheme.split_once('/')?;
    let host_part = authority.rsplit('@').next()?;
    let host = host_part.split(':').next()?;
    Some((host, path))
}

fn split_owner_repo(path: &str) -> Option<(&str, &str)> {
    let path = path.trim_start_matches('/');
    // Take the last two non-empty path segments (handles nested GitLab groups by
    // using only owner = last dir before repo; callers can refine later if needed).
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 2 {
        return None;
    }
    let repo = segments[segments.len() - 1];
    let owner = segments[segments.len() - 2];
    Some((owner, repo))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(host: &str, owner: &str, repo: &str) -> RemoteInfo {
        RemoteInfo {
            host: host.into(),
            owner: owner.into(),
            repo: repo.into(),
            url: format!("https://{host}/{owner}/{repo}"),
        }
    }

    #[test]
    fn https_with_dot_git() {
        assert_eq!(
            parse_remote_url("https://github.com/openbao/openbao.git"),
            Some(info("github.com", "openbao", "openbao")),
        );
    }

    #[test]
    fn https_without_dot_git() {
        assert_eq!(
            parse_remote_url("https://github.com/openbao/openbao"),
            Some(info("github.com", "openbao", "openbao")),
        );
    }

    #[test]
    fn https_with_credentials_is_normalized() {
        assert_eq!(
            parse_remote_url("https://user:token@github.com/o/r.git"),
            Some(info("github.com", "o", "r")),
        );
    }

    #[test]
    fn scp_style() {
        assert_eq!(
            parse_remote_url("git@github.com:openbao/openbao.git"),
            Some(info("github.com", "openbao", "openbao")),
        );
    }

    #[test]
    fn ssh_scheme() {
        assert_eq!(
            parse_remote_url("ssh://git@github.com/o/r.git"),
            Some(info("github.com", "o", "r")),
        );
    }

    #[test]
    fn ssh_scheme_with_port() {
        assert_eq!(
            parse_remote_url("ssh://git@git.example.com:2222/o/r.git"),
            Some(info("git.example.com", "o", "r")),
        );
    }

    #[test]
    fn gitlab_nested_group_uses_last_two_segments() {
        // GitLab subgroups: owner/subgroup/repo — we take the last two as owner/repo.
        assert_eq!(
            parse_remote_url("https://gitlab.com/group/subgroup/repo.git"),
            Some(info("gitlab.com", "subgroup", "repo")),
        );
    }

    #[test]
    fn self_hosted_host() {
        assert_eq!(
            parse_remote_url("git@git.sr.ht:~user/repo"),
            Some(info("git.sr.ht", "~user", "repo")),
        );
    }

    #[test]
    fn empty_and_garbage() {
        assert!(parse_remote_url("").is_none());
        assert!(parse_remote_url("not a url").is_none());
        assert!(parse_remote_url("https://github.com/only-one-segment").is_none());
    }
}
