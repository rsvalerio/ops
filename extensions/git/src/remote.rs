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
/// Invariant for `url`: normalized URL preserving the original input scheme
/// (https / http / ssh / git), no credentials, no `.git` suffix. PATTERN-1
/// (TASK-1237): the previous shape unconditionally synthesised `https://…`,
/// which silently rewrote `http`/`git`/`ssh` remotes to advertise TLS — a
/// misattribution audit/policy code that distinguishes scheme can mistake for
/// "TLS-fronted". scp-style remotes (`git@host:owner/repo`) are normalised to
/// `ssh://…` since scp form has no syntactic equivalent in the JSON contract.
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

    let (scheme, host, path) = split_scheme_host_and_path(raw)?;
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
        url: format!("{scheme}://{host}/{owner}/{repo}"),
    })
}

/// Schemes accepted by [`parse_remote_url`]. `file://`, `javascript:`, and
/// other custom schemes are rejected to keep attacker-influenced git config
/// values from producing unsafe URLs downstream.
const ALLOWED_SCHEMES: &[&str] = &["https", "http", "ssh", "git"];

/// PATTERN-1 (TASK-1237): return the original scheme alongside the host/path
/// split, so the synthesised `RemoteInfo.url` can preserve it. scp form has
/// no scheme syntax — return `"ssh"` for it, matching how every Git client
/// dispatches scp-style remotes.
fn split_scheme_host_and_path(raw: &str) -> Option<(&'static str, &str, &str)> {
    // scp-style: [user@]host:owner/repo (implicitly ssh). The user prefix is
    // optional — `redact_userinfo` strips it before this point on scp inputs
    // that pass through `read_origin_url_from`, so the parser must accept the
    // already-redacted form (`host:owner/repo`) as well.
    if !raw.contains("://") {
        let after_user = match raw.find('@') {
            Some(at) => &raw[at + 1..],
            None => raw,
        };
        let colon = after_user.find(':')?;
        // Reject scp form when a `/` appears before the `:` — per git URL
        // semantics, that path is a relative filesystem path, not a remote.
        if let Some(slash) = after_user.find('/') {
            if slash < colon {
                return None;
            }
        }
        let host = &after_user[..colon];
        let path = &after_user[colon + 1..];
        return Some(("ssh", host, path));
    }

    // URL form: scheme://[user@]host[:port]/path
    let (scheme, after_scheme) = raw.split_once("://")?;
    let canonical_scheme = ALLOWED_SCHEMES
        .iter()
        .find(|s| s.eq_ignore_ascii_case(scheme))
        .copied()?;
    let (authority, path) = after_scheme.split_once('/')?;
    let host_part = authority.rsplit('@').next()?;
    let host = host_part.split(':').next()?;
    Some((canonical_scheme, host, path))
}

/// Permissive RFC 3986 reg-name check: ASCII alphanumeric plus `.` and `-`.
/// Rejects empty hosts and anything containing whitespace, control chars, `/`,
/// `\`, `?`, `#`, `@`, etc. — anywhere those could end up interpolated into a
/// URL or shown as a clickable link by a downstream consumer.
///
/// SEC-11 / TASK-0782: also rejects degenerate shapes that pass the byte
/// allowlist but produce hosts that no DNS resolver would accept and that
/// downstream consumers can mis-parse — a leading `-` is treated as a flag
/// by some legacy curl-like consumers, a leading/trailing `.` is meaningless
/// DNS, and an empty label (e.g. `..` or `foo..bar`) is invalid.
fn is_valid_host(host: &str) -> bool {
    if host.is_empty() {
        return false;
    }
    let bytes = host.as_bytes();
    let first = bytes[0];
    let last = bytes[bytes.len() - 1];
    if first == b'-' || first == b'.' || last == b'-' || last == b'.' {
        return false;
    }
    if host.split('.').any(|label| label.is_empty()) {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'.' || *b == b'-')
}

fn split_owner_repo(path: &str) -> Option<(&str, &str)> {
    let path = path.trim_start_matches('/');
    // PATTERN-1 / TASK-0724: preserve the full owner path so nested GitLab
    // subgroups (`group/subgroup/repo`) round-trip correctly. The previous
    // behaviour kept only the last two segments, which produced a 404 URL
    // for any subgroup project. Each owner segment is still validated by
    // `is_valid_path_segment` to keep the smuggled-char allowlist intact.
    let trimmed = path.trim_end_matches('/');
    let (owner, repo) = trimmed.rsplit_once('/')?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    if !is_valid_path_segment(repo) {
        return None;
    }
    if owner.split('/').any(|seg| !is_valid_path_segment(seg)) {
        return None;
    }
    Some((owner, repo))
}

/// Allowlist for owner/repo path segments.
///
/// The reconstructed `https://{host}/{owner}/{repo}` URL flows into JSON output
/// and downstream renderers, so a control byte or shell metacharacter in
/// owner/repo would silently smuggle bytes into something that looks
/// "normalized". Allowed: ASCII alphanumerics, `.`, `-`, `_`, plus a single
/// leading `~` for sourcehut-style users (`~user/repo`).
fn is_valid_path_segment(segment: &str) -> bool {
    if segment.is_empty() {
        return false;
    }
    let bytes = segment.as_bytes();
    let rest = if bytes[0] == b'~' { &bytes[1..] } else { bytes };
    if rest.is_empty() {
        return false;
    }
    // SEC-13 (TASK-0929): reject segments composed entirely of `.` (`.`,
    // `..`, `...`, ...). Otherwise a hostile `.git/config` like
    // `https://github.com/../etc.git` would round-trip through
    // `git_info.remote_url`, and downstream tools that consume the JSON
    // literally (audit logs, mirrors, tickets) would capture a
    // path-traversal form. Aligns with the host-segment validator that
    // already rejects empty / dot-only labels (TASK-0782).
    if rest.iter().all(|b| *b == b'.') {
        return false;
    }
    rest.iter()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(host: &str, owner: &str, repo: &str) -> RemoteInfo {
        info_scheme("https", host, owner, repo)
    }

    fn info_scheme(scheme: &str, host: &str, owner: &str, repo: &str) -> RemoteInfo {
        RemoteInfo {
            host: host.into(),
            owner: owner.into(),
            repo: repo.into(),
            url: format!("{scheme}://{host}/{owner}/{repo}"),
        }
    }

    /// SEC-13 (TASK-0929): a `.`-only path segment (`.`, `..`, `...`) must
    /// be rejected before the synthesized URL can capture a traversal form.
    /// Browsers collapse `../` away, but downstream tools that consume the
    /// JSON literally (audit logs, mirrors, tickets) capture the
    /// traversal — silently misdirecting operators.
    #[test]
    fn dot_only_owner_segment_rejected() {
        assert_eq!(parse_remote_url("https://github.com/../etc.git"), None);
        assert_eq!(
            parse_remote_url("https://gitlab.com/group/../repo.git"),
            None
        );
        assert_eq!(parse_remote_url("https://github.com/owner/.."), None);
        assert_eq!(parse_remote_url("https://github.com/./repo.git"), None);
        assert_eq!(parse_remote_url("https://github.com/.../repo.git"), None);
    }

    /// SEC-13 (TASK-0929): legitimate `.`-containing names (e.g. `my.lib`,
    /// `lib.rs`) must still parse — the rejection is *all*-`.` segments,
    /// not any segment containing a `.`.
    #[test]
    fn dot_containing_names_still_accepted() {
        assert_eq!(
            parse_remote_url("https://github.com/my.lib/lib.rs.git"),
            Some(info("github.com", "my.lib", "lib.rs")),
        );
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

    /// PATTERN-1 (TASK-1237): scp-style remotes synthesise an `ssh://` URL,
    /// not `https://` — the original transport is ssh, not TLS.
    #[test]
    fn scp_style() {
        assert_eq!(
            parse_remote_url("git@github.com:openbao/openbao.git"),
            Some(info_scheme("ssh", "github.com", "openbao", "openbao")),
        );
    }

    /// PATTERN-1 (TASK-1237): an explicit `ssh://` scheme round-trips into
    /// the synthesised `RemoteInfo.url` — previously rewritten to https.
    #[test]
    fn ssh_scheme() {
        assert_eq!(
            parse_remote_url("ssh://git@github.com/o/r.git"),
            Some(info_scheme("ssh", "github.com", "o", "r")),
        );
    }

    #[test]
    fn ssh_scheme_with_port() {
        assert_eq!(
            parse_remote_url("ssh://git@git.example.com:2222/o/r.git"),
            Some(info_scheme("ssh", "git.example.com", "o", "r")),
        );
    }

    /// PATTERN-1 (TASK-1237): an `http://` remote keeps its scheme — audit
    /// code that distinguishes TLS-fronted (`https`) from cleartext (`http`)
    /// must not see the previous silent rewrite.
    #[test]
    fn http_scheme_round_trips() {
        assert_eq!(
            parse_remote_url("http://internal.example.com/o/r.git"),
            Some(info_scheme("http", "internal.example.com", "o", "r")),
        );
    }

    /// PATTERN-1 (TASK-1237): the `git://` anonymous-clone scheme is
    /// preserved verbatim, not silently upgraded to `https`.
    #[test]
    fn git_scheme_round_trips() {
        assert_eq!(
            parse_remote_url("git://anon.example.com/o/r.git"),
            Some(info_scheme("git", "anon.example.com", "o", "r")),
        );
    }

    /// PATTERN-1 (TASK-1237): scheme matching is case-insensitive on input
    /// but the synthesised scheme is normalised to lowercase, so audit code
    /// downstream sees a canonical value.
    #[test]
    fn scheme_normalises_to_lowercase() {
        let parsed = parse_remote_url("HTTPS://github.com/o/r").expect("parsed");
        assert_eq!(parsed.url, "https://github.com/o/r");
    }

    #[test]
    fn gitlab_nested_group_preserves_full_owner_path() {
        // PATTERN-1 / TASK-0724: nested GitLab subgroups round-trip with the
        // full owner path (`group/subgroup`), so the synthesised URL points
        // at a real project page instead of a 404.
        assert_eq!(
            parse_remote_url("https://gitlab.com/group/subgroup/repo.git"),
            Some(info("gitlab.com", "group/subgroup", "repo")),
        );
    }

    #[test]
    fn gitlab_deeply_nested_group_round_trips() {
        let parsed = parse_remote_url("https://gitlab.com/a/b/c/d/repo.git").expect("parsed");
        assert_eq!(parsed.owner, "a/b/c/d");
        assert_eq!(parsed.url, "https://gitlab.com/a/b/c/d/repo");
    }

    #[test]
    fn self_hosted_host() {
        assert_eq!(
            parse_remote_url("git@git.sr.ht:~user/repo"),
            Some(info_scheme("ssh", "git.sr.ht", "~user", "repo")),
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
    fn rejects_owner_or_repo_with_smuggled_chars() {
        // SEC-11 / SEC-13: the reconstructed `https://{host}/{owner}/{repo}`
        // URL must not silently embed quotes, angle brackets, control chars,
        // or other shell metacharacters smuggled through the owner/repo slot.
        assert!(parse_remote_url("https://github.com/own'er/repo").is_none());
        assert!(parse_remote_url("https://github.com/owner/<script>").is_none());
        assert!(parse_remote_url("https://github.com/foo\u{0007}/bar").is_none());
        assert!(parse_remote_url("https://github.com/foo bar/baz").is_none());
        assert!(parse_remote_url("https://github.com/foo/bar?evil").is_none());
    }

    /// SEC-11 / TASK-0782: hosts must reject leading/trailing dash or dot
    /// and any empty label — these shapes pass the byte allowlist but are
    /// invalid DNS and can be mis-parsed downstream (a leading `-` is a
    /// flag to some curl-like consumers; `..` and `host.` have no resolver
    /// meaning and would surface as broken clickable URLs).
    #[test]
    fn rejects_host_with_leading_dash() {
        assert!(parse_remote_url("https://-evil.com/o/r").is_none());
    }

    #[test]
    fn rejects_host_with_trailing_dash() {
        assert!(parse_remote_url("https://host-/o/r").is_none());
    }

    #[test]
    fn rejects_host_with_leading_dot() {
        assert!(parse_remote_url("https://.com/o/r").is_none());
    }

    #[test]
    fn rejects_host_with_trailing_dot() {
        assert!(parse_remote_url("https://host./o/r").is_none());
    }

    #[test]
    fn rejects_host_with_empty_label() {
        // Consecutive dots → empty label between them.
        assert!(parse_remote_url("https://foo..bar/o/r").is_none());
        assert!(parse_remote_url("https://../o/r").is_none());
    }

    #[test]
    fn rejects_invalid_host_charset() {
        // Spaces, slashes, and control chars in the host slot must not slip through.
        assert!(parse_remote_url("https://bad host/o/r").is_none());
        assert!(parse_remote_url("https://bad/host/o/r/extra").is_some()); // sanity: well-formed
        assert!(parse_remote_url("https://b\u{0007}d/o/r").is_none());
    }
}
