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
    /// Normalized URL preserving the input scheme (`https` / `http` / `ssh` /
    /// `git`), with scp-style remotes normalised to `ssh://`. No `.git`
    /// suffix, no credentials. READ-4 / TASK-1878: this field used to claim
    /// "normalized https URL", which contradicted the struct-level invariant
    /// above after PATTERN-1 / TASK-1237 stopped rewriting every scheme to
    /// `https` — the exact misreading that change was filed to prevent.
    pub url: String,
}

/// Parse a raw git remote URL into a [`RemoteInfo`].
///
/// Handles three common shapes:
/// - `https://host/owner/repo(.git)?` (may include `user:token@` which we strip)
/// - `git@host:owner/repo(.git)?` (scp-style)
/// - `ssh://[user@]host[:port]/owner/repo(.git)?`
#[must_use]
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

    // READ-6 / TASK-1880: canonicalise the host the same way the scheme is
    // canonicalised (`split_scheme_host_and_path` lowercases it via
    // `ALLOWED_SCHEMES`). DNS names are case-insensitive, and `git_info.host`
    // is what consumers compare against (`host == "github.com"`) and group by,
    // so a mixed-case `.git/config` must not produce a value that fails those
    // comparisons. `owner` / `repo` are deliberately left untouched: forge
    // path segments *are* case-sensitive (`GitHub/Hub` != `github/hub`), so
    // lowercasing them would synthesise a URL that 404s.
    let host = host.to_ascii_lowercase();

    let url = format!("{scheme}://{host}/{owner}/{repo}");

    Some(RemoteInfo {
        host,
        owner: owner.to_string(),
        repo: repo.to_string(),
        url,
    })
}

/// Schemes accepted by [`parse_remote_url`] on the `scheme://…` branch.
///
/// Any other `scheme://…` value (`file://`, `javascript://`, `ftp://`, …) is
/// rejected to keep attacker-influenced git config values from producing
/// unsafe URLs downstream.
///
/// SEC-11 / TASK-1861: this list gates the `://` form only. A value with a
/// *single* colon and no `//` (`file:/srv/git/o/r`, `javascript:evil/repo`)
/// never reaches this branch — it is syntactically indistinguishable from an
/// scp-style `host:owner/repo` remote, so it is gated separately by
/// [`SCHEME_LIKE_HOSTS`] on the scp branch.
const ALLOWED_SCHEMES: &[&str] = &["https", "http", "ssh", "git"];

/// SEC-11 / TASK-1861: pre-colon tokens that must never be accepted as an
/// scp-style host.
///
/// The scp branch treats everything before the first `:` as a host, which
/// made `file:/srv/git/o/repo.git` parse as a remote on a host named `file`
/// and re-advertise itself as `ssh://file/srv/git/o/repo` — the transport
/// misattribution PATTERN-1 / TASK-1237 fixed on the `://` path, reached
/// through the branch [`ALLOWED_SCHEMES`] never sees. A single colon is
/// genuinely ambiguous (`intranet:owner/repo` is a valid remote on a
/// dotless intranet host), so the gate is a deny-list of scheme names
/// rather than a hostname heuristic:
///
/// - the [`ALLOWED_SCHEMES`] names themselves — reaching the scp branch with
///   `host == "https"` means the value was a `https:/…` single-slash typo,
///   not a machine literally named `https`;
/// - well-known never-a-git-remote schemes whose rejection the
///   `ALLOWED_SCHEMES` doc comment promises (`file:`, `javascript:`, `ftp:`,
///   `mailto:`, …).
///
/// Matched case-insensitively, against the token *after* any `user@` prefix
/// is stripped. Fail-closed: a host that genuinely carries one of these
/// names drops to `None`, which `provider.rs` renders as "no remote"
/// (SEC-13 / TASK-1151) rather than as a fabricated URL.
const SCHEME_LIKE_HOSTS: &[&str] = &[
    // ALLOWED_SCHEMES in scp position — a mistyped `scheme://`.
    "https",
    "http",
    "ssh",
    "git",
    // Never-a-git-remote schemes.
    "file",
    "javascript",
    "vbscript",
    "data",
    "blob",
    "mailto",
    "ftp",
    "ftps",
    "about",
    "view-source",
];

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
        let after_user = raw.split_once('@').map_or(raw, |(_user, after)| after);
        let (host, path) = after_user.split_once(':')?;
        // Reject scp form when a `/` appears before the `:` — per git URL
        // semantics, that path is a relative filesystem path, not a remote.
        // Everything before the first `:` is `host`, so a `/` there is
        // exactly that case.
        if host.contains('/') {
            return None;
        }
        // SEC-11 / TASK-1861: the `://` branch below is the only place
        // `ALLOWED_SCHEMES` was consulted, so a single-colon value fell
        // through to here with no scheme gate at all. Reject the pre-colon
        // token when it names a URI scheme — see `SCHEME_LIKE_HOSTS`.
        if SCHEME_LIKE_HOSTS
            .iter()
            .any(|s| s.eq_ignore_ascii_case(host))
        {
            return None;
        }
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
///
/// SEC-33 / TASK-1869: also enforces the DNS size limits. The `.git/config`
/// read is capped at `MAX_GIT_CONFIG_BYTES` (4 MiB), but that bounds the
/// *file*, not a single value — a `url = https://<4 MiB of 'a'>/o/r` line
/// otherwise propagated whole into `git_info.host` and `remote_url`.
fn is_valid_host(host: &str) -> bool {
    if host.is_empty() || host.len() > MAX_HOST_BYTES {
        return false;
    }
    let bytes = host.as_bytes();
    if matches!(bytes.first(), Some(b'-' | b'.')) || matches!(bytes.last(), Some(b'-' | b'.')) {
        return false;
    }
    if host
        .split('.')
        .any(|label| label.is_empty() || label.len() > MAX_HOST_LABEL_BYTES)
    {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'.' || *b == b'-')
}

/// SEC-33 / TASK-1869: RFC 1035 caps a DNS name at 253 presentation bytes.
/// Anything longer is unresolvable, so accepting it only lets attacker-chosen
/// text of unbounded length reach `git_info.host` / `remote_url` and the
/// renderers that pad, wrap, or column-align those values.
const MAX_HOST_BYTES: usize = 253;

/// SEC-33 / TASK-1869: RFC 1035 caps a single DNS label at 63 bytes.
const MAX_HOST_LABEL_BYTES: usize = 63;

/// SEC-33 / TASK-1869: cap on one owner / repo path segment.
///
/// GitHub caps repository and account names at 100 characters and GitLab at
/// 255; 255 is the generous bound that still rejects the megabyte-scale
/// segment a hostile `.git/config` can otherwise smuggle through.
const MAX_PATH_SEGMENT_BYTES: usize = 255;

/// SEC-33 / TASK-1869: cap on the *whole* owner path, which
/// [`split_owner_repo`] deliberately preserves at arbitrary depth for nested
/// GitLab subgroups (PATTERN-1 / TASK-0724). Per-segment bounds alone leave
/// the total unbounded, so bound the depth and the total length too. GitLab
/// allows 20 levels of subgroup nesting; 32 segments / 1 KiB clears every
/// real forge layout by a wide margin.
const MAX_OWNER_SEGMENTS: usize = 32;

/// SEC-33 / TASK-1869: cap on the total owner-path length. See
/// [`MAX_OWNER_SEGMENTS`].
const MAX_OWNER_BYTES: usize = 1024;

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
    // SEC-33 / TASK-1869: bound the preserved owner path as a whole, not
    // only segment by segment — see `MAX_OWNER_SEGMENTS` / `MAX_OWNER_BYTES`.
    if owner.len() > MAX_OWNER_BYTES {
        return None;
    }
    let mut segments = 0usize;
    for seg in owner.split('/') {
        if !is_valid_path_segment(seg) {
            return None;
        }
        segments = segments.saturating_add(1);
        if segments > MAX_OWNER_SEGMENTS {
            return None;
        }
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
///
/// SEC-33 / TASK-1869: also bounded at [`MAX_PATH_SEGMENT_BYTES`] — the
/// allowlist alone left a single segment free to carry megabytes of
/// attacker-chosen text into `git_info.owner` / `repo` / `remote_url`.
fn is_valid_path_segment(segment: &str) -> bool {
    if segment.is_empty() || segment.len() > MAX_PATH_SEGMENT_BYTES {
        return false;
    }
    let rest = segment.strip_prefix('~').unwrap_or(segment).as_bytes();
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

    /// SEC-11 / TASK-1861: a single-colon value never reaches the `://`
    /// branch, so `ALLOWED_SCHEMES` never saw it. Each of these previously
    /// parsed as an scp remote with a fabricated host (`ssh://file/…`,
    /// `ssh://javascript/…`), which is the transport misattribution
    /// TASK-1237 closed on the other branch.
    #[test]
    fn rejects_single_colon_scheme_forms() {
        assert_eq!(parse_remote_url("file:/srv/git/o/repo.git"), None);
        assert_eq!(parse_remote_url("javascript:evil/repo"), None);
        assert_eq!(parse_remote_url("ftp:host/o/r"), None);
        assert_eq!(parse_remote_url("mailto:o/r"), None);
        // Single-slash `https:/` typo: the pre-colon token is an
        // ALLOWED_SCHEMES name, which can only mean a mistyped `https://`.
        assert_eq!(parse_remote_url("https:/github.com/o/r"), None);
        // Case-insensitive, and unaffected by a `user@` prefix.
        assert_eq!(parse_remote_url("FILE:/srv/git/o/repo.git"), None);
        assert_eq!(parse_remote_url("git@javascript:evil/repo"), None);
    }

    /// SEC-11 / TASK-1861: the deny-list must not cost the genuine scp
    /// shapes — both the raw `user@host:owner/repo` form and the
    /// already-redacted `host:owner/repo` form `read_origin_url` produces.
    #[test]
    fn genuine_scp_forms_still_parse_after_scheme_gate() {
        assert_eq!(
            parse_remote_url("git@github.com:o/r.git"),
            Some(info_scheme("ssh", "github.com", "o", "r")),
        );
        assert_eq!(
            parse_remote_url("github.com:o/r.git"),
            Some(info_scheme("ssh", "github.com", "o", "r")),
        );
        // A dotless intranet host is ambiguous with `scheme:path` but is a
        // real remote shape, so it stays accepted — only scheme *names* are
        // denied.
        assert_eq!(
            parse_remote_url("git@intranet:o/r.git"),
            Some(info_scheme("ssh", "intranet", "o", "r")),
        );
    }

    /// SEC-33 / TASK-1869: the 4 MiB `.git/config` cap bounds the file, not
    /// a single value. Without per-value bounds a megabyte-long host or
    /// owner segment propagates whole into `git_info` JSON and About cards.
    #[test]
    fn rejects_host_over_dns_length_limit() {
        // Build from DNS-legal labels so this test pins the *total* bound,
        // not the per-label one: 63 + 1 + 63 + 1 + 63 + 1 + N.
        let host_of_len = |total: usize| -> String {
            let label = "a".repeat(MAX_HOST_LABEL_BYTES);
            let tail = total - (MAX_HOST_LABEL_BYTES + 1) * 3;
            format!("{label}.{label}.{label}.{}", "b".repeat(tail))
        };

        let just_under = host_of_len(MAX_HOST_BYTES);
        assert_eq!(just_under.len(), MAX_HOST_BYTES);
        assert!(parse_remote_url(&format!("https://{just_under}/o/r")).is_some());

        let just_over = host_of_len(MAX_HOST_BYTES + 1);
        assert_eq!(just_over.len(), MAX_HOST_BYTES + 1);
        assert!(parse_remote_url(&format!("https://{just_over}/o/r")).is_none());
    }

    #[test]
    fn rejects_host_label_over_dns_limit() {
        let ok = "a".repeat(MAX_HOST_LABEL_BYTES);
        assert!(parse_remote_url(&format!("https://{ok}.com/o/r")).is_some());

        let too_long = "a".repeat(MAX_HOST_LABEL_BYTES + 1);
        assert!(parse_remote_url(&format!("https://{too_long}.com/o/r")).is_none());
    }

    #[test]
    fn rejects_path_segment_over_limit() {
        let ok = "a".repeat(MAX_PATH_SEGMENT_BYTES);
        assert!(parse_remote_url(&format!("https://github.com/{ok}/r")).is_some());
        assert!(parse_remote_url(&format!("https://github.com/o/{ok}")).is_some());

        let too_long = "a".repeat(MAX_PATH_SEGMENT_BYTES + 1);
        assert!(parse_remote_url(&format!("https://github.com/{too_long}/r")).is_none());
        assert!(parse_remote_url(&format!("https://github.com/o/{too_long}")).is_none());
    }

    #[test]
    fn rejects_owner_path_over_segment_and_byte_limits() {
        let ok_depth = vec!["a"; MAX_OWNER_SEGMENTS].join("/");
        assert!(parse_remote_url(&format!("https://gitlab.com/{ok_depth}/repo")).is_some());

        let too_deep = vec!["a"; MAX_OWNER_SEGMENTS + 1].join("/");
        assert!(parse_remote_url(&format!("https://gitlab.com/{too_deep}/repo")).is_none());

        // Few segments, but a total owner path past the byte cap.
        let wide = vec!["a".repeat(200); 6].join("/");
        assert!(wide.len() > MAX_OWNER_BYTES);
        assert!(parse_remote_url(&format!("https://gitlab.com/{wide}/repo")).is_none());
    }

    /// SEC-33 / TASK-1869: a realistic nested GitLab subgroup must be
    /// unaffected by the new bounds.
    #[test]
    fn realistic_nested_subgroup_unaffected_by_length_bounds() {
        assert_eq!(
            parse_remote_url("https://gitlab.com/a/b/c/d/repo.git"),
            Some(info("gitlab.com", "a/b/c/d", "repo")),
        );
    }

    /// READ-6 / TASK-1880: the host is canonicalised to lowercase alongside
    /// the scheme, so `host == "github.com"` comparisons and any grouping
    /// keyed on `host` / `remote_url` survive a mixed-case `.git/config`.
    #[test]
    fn host_normalises_to_lowercase() {
        let parsed = parse_remote_url("HTTPS://GitHub.COM/o/r").expect("parsed");
        assert_eq!(parsed.host, "github.com");
        assert_eq!(parsed.url, "https://github.com/o/r");
    }

    /// READ-6 / TASK-1880: owner and repo keep their case — forge path
    /// segments are case-sensitive, so lowercasing them would synthesise a
    /// URL that 404s.
    #[test]
    fn owner_and_repo_case_is_preserved() {
        let parsed = parse_remote_url("https://GitHub.com/OpenBao/OpenBao.git").expect("parsed");
        assert_eq!(parsed.host, "github.com");
        assert_eq!(parsed.owner, "OpenBao");
        assert_eq!(parsed.repo, "OpenBao");
        assert_eq!(parsed.url, "https://github.com/OpenBao/OpenBao");
    }

    #[test]
    fn rejects_invalid_host_charset() {
        // Spaces, slashes, and control chars in the host slot must not slip through.
        assert!(parse_remote_url("https://bad host/o/r").is_none());
        assert!(parse_remote_url("https://bad/host/o/r/extra").is_some()); // sanity: well-formed
        assert!(parse_remote_url("https://b\u{0007}d/o/r").is_none());
    }
}
