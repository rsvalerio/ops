//! Parse git remote URLs into a structured form.

use serde::Serialize;

/// Parsed remote-URL fields.
///
/// Bare `String` fields are intentional: this struct is produced by
/// [`parse_remote_url`] and immediately consumed by `provider.rs`, which serialises
/// each field individually into a flat `serde_json` object. Newtype wrappers
/// (`Host`, `Owner`, `RepoName`, `RepoUrl`) were considered for argument-order
/// safety, but every consumer accesses fields by name (never positionally) and
/// the JSON serialization shape would have to be hand-rolled to strip the wrapper
/// — paying complexity for no caller-side win. Revisit if a function takes
/// multiple of these as positional arguments.
///
/// Invariant for `url`: normalized https URL, no credentials, no `.git` suffix.
/// Enforced inside [`parse_remote_url`]; do not construct `RemoteInfo` outside
/// that function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
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
    if !is_valid_host(host) {
        return None;
    }
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

/// Schemes accepted by [`parse_remote_url`]. `file://`, `javascript:`, and
/// other custom schemes are rejected to keep attacker-influenced git config
/// values from producing unsafe URLs downstream.
const ALLOWED_SCHEMES: &[&str] = &["https", "http", "ssh", "git"];

fn split_host_and_path(raw: &str) -> Option<(&str, &str)> {
    // scp-style: git@host:owner/repo (implicitly ssh)
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
    let (scheme, after_scheme) = raw.split_once("://")?;
    if !ALLOWED_SCHEMES
        .iter()
        .any(|s| s.eq_ignore_ascii_case(scheme))
    {
        return None;
    }
    let (authority, path) = after_scheme.split_once('/')?;
    let host_part = authority.rsplit('@').next()?;
    let host = host_part.split(':').next()?;
    Some((host, path))
}

/// Permissive RFC 3986 reg-name check: ASCII alphanumeric plus `.` and `-`.
/// Rejects empty hosts and anything containing whitespace, control chars, `/`,
/// `\`, `?`, `#`, `@`, etc. — anywhere those could end up interpolated into a
/// URL or shown as a clickable link by a downstream consumer.
fn is_valid_host(host: &str) -> bool {
    !host.is_empty()
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
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

    #[test]
    fn ssh_scheme_strips_credentials_and_keeps_host_only() {
        let info = parse_remote_url("ssh://user:secret@git.example/o/r.git").expect("parsed");
        assert_eq!(info.host, "git.example");
        assert_eq!(info.owner, "o");
        assert_eq!(info.repo, "r");
        assert!(!info.url.contains("user:secret"));
        assert!(!info.url.contains('@'));
    }

    #[test]
    fn ipv6_host_form_is_rejected() {
        // [::1] / bracketed IPv6 is not in our reg-name allowlist; reject rather
        // than admit a partially-parsed weird shape into RemoteInfo.
        assert!(parse_remote_url("ssh://git@[::1]:22/o/r.git").is_none());
    }

    #[test]
    fn empty_host_authority_is_rejected() {
        assert!(parse_remote_url("https:///o/r").is_none());
    }

    #[test]
    fn file_scheme_is_rejected() {
        assert!(parse_remote_url("file:///srv/git/o/r.git").is_none());
    }

    #[test]
    fn malformed_scheme_is_rejected() {
        assert!(parse_remote_url("ht!tp://host.example/o/r").is_none());
        assert!(parse_remote_url("://host.example/o/r").is_none());
    }

    #[test]
    fn rejects_unknown_scheme() {
        assert!(parse_remote_url("file:///etc/passwd/x/y").is_none());
        assert!(parse_remote_url("javascript://evil/o/r").is_none());
        assert!(parse_remote_url("ftp://host.example/o/r").is_none());
    }

    #[test]
    fn rejects_invalid_host_charset() {
        // Spaces, slashes, and control chars in the host slot must not slip through.
        assert!(parse_remote_url("https://bad host/o/r").is_none());
        assert!(parse_remote_url("https://bad/host/o/r/extra").is_some()); // sanity: well-formed
        assert!(parse_remote_url("https://b\u{0007}d/o/r").is_none());
    }
}
