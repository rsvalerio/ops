//! Repository URL normalisation for `package.json::repository` values.
//!
//! This is the crate's adversarial-input surface: a `repository` value is
//! untrusted manifest text that ends up as a clickable link in About cards,
//! markdown and HTML. It lives in its own module, separate from the serde
//! model and the parse orchestrator in [`super::package_json`], so that
//! boundary has one test target.
//!
//! Every rejection **drops the whole field** — [`normalize_repo_url`] returns
//! an empty string and the caller renders the repository as missing — rather
//! than stripping the offending part, because a partially rewritten URL is
//! still a link someone will follow. The rejections are: any control byte in
//! the body, a scheme outside the `http(s)` allowlist, userinfo in the
//! authority, a hostless SSH body, and `..` path traversal.

// The control-character predicate and the scheme allowlist are shared with
// the Python provider through `ops_about::text_util`, so the sanitisation
// boundary has one definition across stacks rather than two copies that can
// drift apart.
use ops_about::text_util::{contains_control_chars, has_allowed_url_scheme};

/// Normalise a `repository` URL value: turn npm shorthand
/// (`github:owner/repo`), git+ssh, ssh scp form, git+https, and the bare
/// `.git` suffix into a plain `https://host/path` shape that renders
/// cleanly in the About card.
///
/// SSH URL handling is delegated to [`ssh_to_https`].
///
/// Returns an empty string for a value that fails any of the module's
/// rejection rules; `package_json::parse_package_json` treats that exactly
/// like a missing `repository` field. A control byte anywhere in the body is
/// evidence of tampering — `"github:owner/repo\nINJECT"` would otherwise
/// normalise to a clickable `https://github.com/owner/repoINJECT` pointing at
/// an attacker-chosen repo — and the scheme allowlist keeps
/// `javascript:alert(1)`, `data:text/html;…`, `vbscript:x`,
/// `file:///etc/passwd` and their `git+`-prefixed twins out of the About card
/// and `ops about --json`. The rewrite branches only ever emit `https://`, so
/// the allowlist is what guards the clean-URL fall-through and the
/// `git+<body>` branch, which pass their input through.
///
/// The return type is `Cow<'_, str>`: a well-formed
/// `https://github.com/owner/repo` needing no rewrite passes straight through
/// borrowed, and only the rewriting branches (npm shorthand, SSH form,
/// scrubbing) allocate. Callers needing an owned value can `.into_owned()`.
pub fn normalize_repo_url(raw: &str) -> std::borrow::Cow<'_, str> {
    let normalized = normalize_repo_url_shape(raw);
    // The scheme allowlist inspects only the leading bytes, so an authority
    // carrying RFC 3986 userinfo
    // (`https://github.com@evil.com/o/r`) reads as an allowlisted `https://`
    // URL while the effective host is `evil.com`. Any `@` in the authority
    // drops the field, the same drop-not-strip policy applied to control
    // characters, traversal, hostless authorities, and non-`http(s)`
    // schemes. The gate sits after every rewrite branch, so the clean-URL
    // fall-through and every `git://` / `git+<scheme>://` branch that routes
    // through `scrub_authority_and_path` (which deliberately preserves the
    // authority verbatim) are all covered. `ssh://git@host/…` never reaches
    // here with its userinfo: `ssh_to_https` strips the `git@` prefix during
    // the rewrite.
    if has_allowed_url_scheme(&normalized) && !authority_has_userinfo(&normalized) {
        normalized
    } else {
        std::borrow::Cow::Borrowed("")
    }
}

/// Whether the authority segment of a `<scheme>://…` URL carries RFC 3986
/// userinfo (`user@host` or `user:pass@host`) —
/// everything before the `@` is not the host, so a value like
/// `github.com@evil.com` presents a github-looking authority whose
/// effective host is `evil.com`. Per RFC 3986 the authority ends at the
/// first `/`, `?` or `#`; anything after those is path, query or fragment,
/// where `@` is legitimate (branch names, mailto-shaped filters) and must
/// not drop the field.
fn authority_has_userinfo(url: &str) -> bool {
    let Some((_, rest)) = url.split_once("://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    authority.contains('@')
}

/// Apply the shorthand / SSH / `git+` rewrite branches, without the scheme
/// allowlist. Kept separate from [`normalize_repo_url`] so the allowlist
/// guards every branch's result in one place rather than being repeated at
/// each `return`.
fn normalize_repo_url_shape(raw: &str) -> std::borrow::Cow<'_, str> {
    /// (shorthand prefix, host) for npm hostname shortcuts.
    const HOST_PREFIXES: &[(&str, &str)] = &[
        ("github:", "github.com"),
        ("gitlab:", "gitlab.com"),
        ("bitbucket:", "bitbucket.org"),
    ];

    use std::borrow::Cow;
    if contains_control_chars(raw) {
        return Cow::Borrowed("");
    }
    let s = raw.trim();
    for (prefix, host) in HOST_PREFIXES {
        if let Some(rest) = s.strip_prefix(prefix) {
            let cleaned = scrub_path_segments(rest.trim_end_matches(".git"));
            if cleaned.is_empty() {
                return Cow::Owned(format!("https://{host}"));
            }
            return Cow::Owned(format!("https://{host}/{cleaned}"));
        }
    }
    if let Some(rest) = s
        .strip_prefix("git+ssh://")
        .or_else(|| s.strip_prefix("ssh://"))
    {
        return Cow::Owned(ssh_to_https(rest));
    }
    if let Some(rest) = s.strip_prefix("git+") {
        let trimmed = rest.trim_end_matches(".git");
        if let Some(after) = trimmed.strip_prefix("git://") {
            return Cow::Owned(format!("https://{}", scrub_authority_and_path(after)));
        }
        return Cow::Owned(scrub_full_url_path(trimmed));
    }
    if let Some(rest) = s.strip_prefix("git://") {
        return Cow::Owned(format!(
            "https://{}",
            scrub_authority_and_path(rest.trim_end_matches(".git"))
        ));
    }
    if is_bare_github_shorthand(s) {
        let cleaned = scrub_path_segments(s);
        if cleaned.is_empty() {
            return Cow::Borrowed("https://github.com");
        }
        return Cow::Owned(format!("https://github.com/{cleaned}"));
    }
    // Clean-URL fall-through: borrow the trimmed slice so a well-formed
    // `https://github.com/owner/repo` (no `.git` suffix) needs no allocation.
    Cow::Borrowed(s.trim_end_matches(".git"))
}

/// Recognise the bare `owner/repo` npm shorthand that npm itself accepts in
/// `package.json::repository`. Requires:
/// - no scheme (no `:` anywhere — this also rejects `@scope/name` only via
///   the leading-`@` check, since scoped names contain no colon),
/// - no leading `@` (scoped npm package names),
/// - exactly one `/` separator,
/// - both segments non-empty,
/// - each segment composed of identifier-shaped ASCII (alphanumerics,
///   `_`, `-`, `.`).
fn is_bare_github_shorthand(s: &str) -> bool {
    if s.is_empty() || s.starts_with('@') || s.contains(':') {
        return false;
    }
    let Some((owner, repo)) = s.split_once('/') else {
        return false;
    };
    if owner.is_empty() || repo.is_empty() || repo.contains('/') {
        return false;
    }
    let ident_ok = |seg: &str| {
        seg.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
    };
    ident_ok(owner) && ident_ok(repo)
}

/// Convert the body of an `ssh://` (or `git+ssh://`) URL to its `https://`
/// equivalent: drop the `git@` user-info, replace an scp-form `host:path`
/// separator with `/`, and strip any trailing `.git` suffix. A numeric port
/// (e.g. `host:22/path`) is preserved verbatim.
///
/// A numeric port is distinguished from an scp-form path whose first segment
/// merely begins with a digit by requiring **all** characters before the next
/// `/` to be digits: `host:42/foo` is a port, `host:42-archive/x` is a path.
///
/// A hostless input (`ssh:///path`, `ssh://git@/path`, or scp-form `git@:foo`
/// with an empty host segment) returns an empty `String`, so the caller drops
/// the field rather than rendering a syntactically broken `https:///<path>`
/// with an empty authority — a clickable but malformed link out of any
/// hostile or typoed `repository.url`.
pub fn ssh_to_https(rest: &str) -> String {
    let no_user = rest.strip_prefix("git@").unwrap_or(rest);
    let trimmed = no_user.trim_end_matches(".git");
    // Drop hostless inputs deterministically.
    if trimmed.is_empty() || trimmed.starts_with('/') {
        return String::new();
    }
    let body = match trimmed.split_once(':') {
        Some(("", _)) => return String::new(),
        Some((host, path)) if !is_numeric_port_prefix(path) => {
            format!("{host}/{path}")
        }
        _ => trimmed.to_string(),
    };
    format!("https://{body}")
}

/// Append `/tree/HEAD/<directory>` to a base repository URL so monorepo
/// member packages render distinguishable links. Strips a leading `./` from
/// the directory and canonicalises slashes.
///
/// Any path component equal to `..` — and any leading absolute slash — is
/// dropped before the suffix is built: an adversarial `package.json` can
/// otherwise emit a directory like `../../../etc/passwd` and get a
/// traversal-shaped URL rendered into About cards, markdown and HTML. Empty
/// and `.` segments are collapsed for the same reason. If every component is
/// filtered out, the directory suffix is omitted and the base URL is returned
/// unchanged.
pub fn append_tree_directory(base: &str, directory: &str) -> String {
    let normalized = directory.trim().trim_start_matches("./");
    let cleaned = scrub_path_segments(normalized);
    if cleaned.is_empty() {
        return base.to_string();
    }
    let trimmed_base = base.trim_end_matches('/');
    format!("{trimmed_base}/tree/HEAD/{cleaned}")
}

/// Drop empty, `.`, and `..` segments from a `/`-separated path.
///
/// The single scrub shared by the npm-shorthand and `git://` branches of
/// [`normalize_repo_url`] and by [`append_tree_directory`], so an adversarial
/// `repository` value like `github:../../etc/passwd` cannot produce a
/// traversal-shaped URL in rendered About output.
fn scrub_path_segments(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..")
        .collect::<Vec<_>>()
        .join("/")
}

/// Scrub path traversal from a `host[/path]` body where the leading
/// segment is the authority (`host[:port]`) and must be preserved verbatim.
///
/// The `git://`, `git+git://`, and `git+<scheme>://` branches of
/// [`normalize_repo_url`] all carry an authority followed by a
/// path component; only the path is scrubbed, the host is kept intact so
/// `git://github.com/o/r` continues to round-trip to `https://github.com/o/r`.
fn scrub_authority_and_path(authority_and_path: &str) -> String {
    match authority_and_path.split_once('/') {
        Some((authority, path)) => {
            let cleaned = scrub_path_segments(path);
            if cleaned.is_empty() {
                authority.to_string()
            } else {
                format!("{authority}/{cleaned}")
            }
        }
        None => authority_and_path.to_string(),
    }
}

/// Scrub path traversal from a full URL of the form `<scheme>://<host>/<path>`.
///
/// The `git+<scheme>://` branch of [`normalize_repo_url`] keeps the scheme
/// intact, so only the path portion is scrubbed: scrubbing `scheme://` too
/// would collapse `https://` to `https:/`, the empty segment between the two
/// slashes being filtered like any other.
fn scrub_full_url_path(url: &str) -> String {
    if let Some((scheme, rest)) = url.split_once("://") {
        format!("{scheme}://{}", scrub_authority_and_path(rest))
    } else {
        scrub_authority_and_path(url)
    }
}

pub fn is_numeric_port_prefix(path: &str) -> bool {
    let port = path.split('/').next().unwrap_or("");
    !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A well-formed `https://github.com/...` URL needing no rewrite passes
    /// through as `Cow::Borrowed`, keeping the dominant clean path
    /// allocation-free.
    #[test]
    fn normalize_clean_https_url_returns_borrowed() {
        let raw = "https://github.com/owner/repo";
        let out = normalize_repo_url(raw);
        assert!(matches!(out, std::borrow::Cow::Borrowed(_)));
        assert_eq!(out, raw);
        // A `.git` suffix is stripped via slice trimming, still borrowed.
        let with_git = "https://github.com/owner/repo.git";
        let out2 = normalize_repo_url(with_git);
        assert!(matches!(out2, std::borrow::Cow::Borrowed(_)));
        assert_eq!(out2, "https://github.com/owner/repo");
    }

    #[test]
    fn normalize_git_ssh_to_https() {
        assert_eq!(
            normalize_repo_url("git+ssh://git@github.com/o/r.git"),
            "https://github.com/o/r"
        );
    }

    #[test]
    fn normalize_ssh_scp_form_url() {
        assert_eq!(
            normalize_repo_url("ssh://git@gitlab.com:owner/r.git"),
            "https://gitlab.com/owner/r"
        );
    }

    #[test]
    fn normalize_git_https_unchanged_path() {
        assert_eq!(
            normalize_repo_url("git+https://github.com/o/r.git"),
            "https://github.com/o/r"
        );
    }

    #[test]
    fn normalize_ssh_scp_form_with_digit_prefixed_owner() {
        assert_eq!(
            normalize_repo_url("ssh://git@github.com:42-archive/x.git"),
            "https://github.com/42-archive/x"
        );
    }

    /// Hostless `ssh:///path` and `ssh://git@/path` inputs must not produce
    /// a syntactically broken `https:///<path>` URL on the About card: the
    /// function returns empty, and `normalize_repo_url` propagates that so
    /// `parse_package_json` treats the field as missing.
    #[test]
    fn ssh_to_https_drops_hostless_inputs() {
        assert_eq!(ssh_to_https("/path"), "");
        assert_eq!(ssh_to_https("git@/path"), "");
        assert_eq!(ssh_to_https(""), "");
        assert_eq!(ssh_to_https("git@:foo"), "");
        assert_eq!(ssh_to_https(":foo"), "");
    }

    /// The `ssh://` / `git+ssh://` branch of `normalize_repo_url`
    /// propagates the empty result from
    /// [`ssh_to_https`], so a hostless URL never reaches the About
    /// card as `https:///<path>`.
    #[test]
    fn normalize_ssh_hostless_input_drops_field() {
        assert_eq!(normalize_repo_url("ssh:///path"), "");
        assert_eq!(normalize_repo_url("ssh://git@/path"), "");
        assert_eq!(normalize_repo_url("git+ssh:///path"), "");
        assert_eq!(normalize_repo_url("ssh://git@:foo"), "");
    }

    #[test]
    fn normalize_ssh_with_numeric_port_keeps_port() {
        assert_eq!(
            normalize_repo_url("ssh://git@host:22/path.git"),
            "https://host:22/path"
        );
    }

    /// A `directory` that escapes the repository root via `..` segments must
    /// be sanitized — the URL is rendered into About cards
    /// (and downstream markdown/HTML), so a traversal-shaped suffix is a
    /// real surface for path-shape attacks.
    #[test]
    fn append_tree_directory_strips_leading_parent_segments() {
        assert_eq!(
            append_tree_directory("https://github.com/o/r", "../foo"),
            "https://github.com/o/r/tree/HEAD/foo"
        );
    }

    #[test]
    fn append_tree_directory_strips_internal_parent_segments() {
        assert_eq!(
            append_tree_directory("https://github.com/o/r", "a/../b"),
            "https://github.com/o/r/tree/HEAD/a/b"
        );
    }

    #[test]
    fn append_tree_directory_strips_absolute_leading_slash() {
        assert_eq!(
            append_tree_directory("https://github.com/o/r", "/absolute"),
            "https://github.com/o/r/tree/HEAD/absolute"
        );
    }

    #[test]
    fn append_tree_directory_drops_when_only_parent_components() {
        assert_eq!(
            append_tree_directory("https://github.com/o/r", "../../.."),
            "https://github.com/o/r"
        );
    }

    #[test]
    fn append_tree_directory_pure_traversal_etc_passwd_is_neutralised() {
        // An adversarial package.json must not produce a URL whose path
        // component contains `../../etc/passwd` style traversal.
        let url = append_tree_directory("https://github.com/o/r", "../../../../etc/passwd");
        assert!(!url.contains(".."), "url still contains ..: {url}");
        assert_eq!(url, "https://github.com/o/r/tree/HEAD/etc/passwd");
    }

    /// `git+git://` is rewritten to `https://` — otherwise the About card
    /// renders an unclickable `git://` URL.
    #[test]
    fn normalize_git_plus_git_scheme_to_https() {
        assert_eq!(
            normalize_repo_url("git+git://github.com/o/r.git"),
            "https://github.com/o/r"
        );
    }

    #[test]
    fn normalize_github_shorthand() {
        assert_eq!(
            normalize_repo_url("github:owner/repo"),
            "https://github.com/owner/repo"
        );
    }

    /// Bare `owner/repo` npm shorthand is rewritten to a GitHub URL —
    /// otherwise the About card emits a non-URL link.
    #[test]
    fn normalize_bare_owner_repo_shorthand() {
        assert_eq!(
            normalize_repo_url("expressjs/express"),
            "https://github.com/expressjs/express"
        );
    }

    /// The bare-shorthand branch routes through `scrub_path_segments` like
    /// its `github:` / `git://` / `git+*://` siblings, so a `"repository":
    /// "../etc"` cannot surface a traversal-shaped URL. Two properties:
    /// 1. the rendered URL contains no literal `..`;
    /// 2. `../etc` lands on `https://github.com/etc` — the `..` segment is
    ///    filtered and `etc` survives.
    #[test]
    fn normalize_bare_shorthand_strips_traversal() {
        let out = normalize_repo_url("../etc");
        assert!(!out.contains(".."), "url still contains ..: {out}");
        assert_eq!(out, "https://github.com/etc");
    }

    /// A bare shorthand whose every segment is
    /// `.`/`..` collapses to the bare host, mirroring the shape
    /// `normalize_github_shorthand_pure_traversal_collapses_to_host`
    /// already pins for the explicit `github:` branch.
    #[test]
    fn normalize_bare_shorthand_pure_traversal_collapses_to_host() {
        assert_eq!(normalize_repo_url("../.."), "https://github.com");
    }

    /// A scoped npm package name like `@scope/name` is not a repo shorthand
    /// and is not rewritten into a github URL. It is not an `http(s)` URL
    /// either, so the scheme allowlist drops the field rather than surfacing
    /// the raw value.
    #[test]
    fn normalize_scoped_npm_name_is_dropped() {
        assert_eq!(normalize_repo_url("@scope/name"), "");
    }

    /// An embedded LF inside a `github:` shorthand drops the field entirely,
    /// so the silent concatenation `repo\nINJECT → repoINJECT` — a clickable
    /// attacker-chosen URL — cannot reach About cards, markdown or HTML.
    #[test]
    fn normalize_drops_field_on_embedded_lf_in_shorthand() {
        let out = normalize_repo_url("github:owner/repo\nINJECT");
        assert!(!out.contains('\n'), "url still contains LF: {out:?}");
        assert!(
            out.is_empty(),
            "field must be dropped on control byte: {out:?}"
        );
    }

    /// A CR inside a git+https URL (Object{url} shape) drops the field.
    /// Pins the behaviour for the
    /// `repository: { url: "..." }` parse path, which routes through the
    /// same `normalize_repo_url` entry point.
    #[test]
    fn normalize_drops_field_on_embedded_cr_in_git_https() {
        let out = normalize_repo_url("git+https://github.com/o/r\r.git");
        assert!(!out.contains('\r'), "url still contains CR: {out:?}");
        assert!(
            out.is_empty(),
            "field must be dropped on control byte: {out:?}"
        );
    }

    /// ANSI escape (U+001B) bytes drop
    /// the field — they would otherwise flow into operator-facing
    /// surfaces (About cards, log lines) and be interpreted as terminal
    /// escapes, or silently concatenate into a clickable URL.
    #[test]
    fn normalize_drops_field_on_embedded_ansi_escape() {
        let out = normalize_repo_url("https://github.com/o/\u{1b}[31mr");
        assert!(!out.contains('\u{1b}'), "url still contains ESC: {out:?}");
        assert!(
            out.is_empty(),
            "field must be dropped on control byte: {out:?}"
        );
    }

    /// The Text shape
    /// (`repository: "github:..."`) is treated identically.
    #[test]
    fn normalize_drops_field_on_control_chars_in_text_shape() {
        let out = normalize_repo_url("github:owner/repo\rINJECT\nMORE");
        assert!(
            !out.contains('\r') && !out.contains('\n'),
            "url still contains control chars: {out:?}"
        );
        assert!(
            out.is_empty(),
            "field must be dropped on control byte: {out:?}"
        );
    }

    /// A tampered URL containing a control byte must not produce a
    /// syntactically valid URL pointing at attacker-chosen path segments.
    /// Pins the contract directly, so a change that re-introduces silent
    /// concatenation fails here.
    #[test]
    fn normalize_drops_field_yields_no_attacker_chosen_url() {
        for raw in [
            "github:owner/repo\nINJECT",
            "github:legit\rINJECT",
            "https://example.com/o/r\u{1b}[31mfake",
            "git+https://example.com/o\u{0c}/passwd",
        ] {
            let out = normalize_repo_url(raw);
            assert!(
                out.is_empty(),
                "expected dropped URL for {raw:?}, got {out:?}"
            );
        }
    }

    /// A `github:` shorthand carrying `..` segments must not produce a
    /// traversal-shaped URL — the same threat model as
    /// [`append_tree_directory`]. The scrub drops every empty, `.`, and `..`
    /// segment before interpolation.
    #[test]
    fn normalize_github_shorthand_strips_traversal() {
        let out = normalize_repo_url("github:../../etc/passwd");
        assert!(!out.contains(".."), "url still contains ..: {out}");
        assert_eq!(out, "https://github.com/etc/passwd");
    }

    #[test]
    fn normalize_gitlab_shorthand_strips_traversal() {
        let out = normalize_repo_url("gitlab:owner/../../../etc/passwd");
        assert!(!out.contains(".."), "url still contains ..: {out}");
        assert_eq!(out, "https://gitlab.com/owner/etc/passwd");
    }

    #[test]
    fn normalize_bitbucket_shorthand_strips_traversal() {
        let out = normalize_repo_url("bitbucket:../foo/bar");
        assert!(!out.contains(".."), "url still contains ..: {out}");
        assert_eq!(out, "https://bitbucket.org/foo/bar");
    }

    /// A shorthand whose suffix is purely traversal
    /// collapses to the bare host — same shape as
    /// `append_tree_directory` returning the base URL when every
    /// component filters out.
    #[test]
    fn normalize_github_shorthand_pure_traversal_collapses_to_host() {
        assert_eq!(normalize_repo_url("github:../../.."), "https://github.com");
    }

    /// The bare `git://` branch scrubs `..` too.
    #[test]
    fn normalize_git_scheme_strips_traversal() {
        let out = normalize_repo_url("git://github.com/../../etc/passwd");
        assert!(!out.contains(".."), "url still contains ..: {out}");
        assert_eq!(out, "https://github.com/etc/passwd");
    }

    /// `git+git://` shares the scrub policy with the bare `git://` branch,
    /// which rewrites to `https://`.
    #[test]
    fn normalize_git_plus_git_scheme_strips_traversal() {
        let out = normalize_repo_url("git+git://github.com/../../etc/passwd");
        assert!(!out.contains(".."), "url still contains ..: {out}");
        assert_eq!(out, "https://github.com/etc/passwd");
    }

    /// `git+<scheme>://` (e.g. `git+https://`) also
    /// scrubs `..` from the path component before rendering into the
    /// About card.
    #[test]
    fn normalize_git_plus_https_strips_traversal() {
        let out = normalize_repo_url("git+https://github.com/o/../../etc/passwd.git");
        assert!(!out.contains(".."), "url still contains ..: {out}");
        assert_eq!(out, "https://github.com/o/etc/passwd");
    }

    /// Both `normalize_repo_url`'s shorthand branch and
    /// `append_tree_directory` route segment scrubbing through
    /// [`scrub_path_segments`], so a future tightening of the filter (Unicode
    /// bidi controls, encoded `..`, …) only needs to land in one place. Pins
    /// that equivalence on a `..`-laden input.
    #[test]
    fn append_tree_directory_and_shorthand_share_segment_filter() {
        // Both entry points must drop `..` and `.` segments identically.
        let tree = append_tree_directory("https://github.com/o/r", "a/../b/./c");
        assert_eq!(tree, "https://github.com/o/r/tree/HEAD/a/b/c");
        let shorthand = normalize_repo_url("github:a/../b/./c");
        assert_eq!(shorthand, "https://github.com/a/b/c");
    }

    /// A debug-log of the normalised URL must remain
    /// single-line — i.e. the `Debug`/`Display` rendering after
    /// normalisation contains no embedded newlines.
    #[test]
    fn normalize_debug_log_stays_single_line() {
        let out = normalize_repo_url("github:owner/repo\nINJECT\rMORE");
        let debug = format!("{out:?}");
        let display = out.to_string();
        assert!(!debug.contains('\n') && !debug.contains('\r'));
        assert!(!display.contains('\n') && !display.contains('\r'));
        // The dropped-field policy means the rendered string is empty, not
        // a silent rewrite to attacker-chosen segments.
        assert!(out.is_empty(), "field must be dropped on control byte");
    }

    /// The verbatim fall-through would otherwise hand any non-URL scheme
    /// straight to the rendered `repository` field. Each of these is a live
    /// injection or local-resource-disclosure sink for a consumer that
    /// renders the value as a hyperlink.
    #[test]
    fn normalize_drops_non_http_schemes() {
        for raw in [
            "javascript:alert(1)",
            "data:text/html;base64,AAA",
            "vbscript:x",
            "file:///etc/passwd",
            "JavaScript:alert(1)",
        ] {
            assert_eq!(normalize_repo_url(raw), "", "scheme survived: {raw:?}");
        }
    }

    /// The `git+` branch strips the prefix and then
    /// returns the body through `scrub_full_url_path`, which preserves any
    /// scheme it finds (and passes a scheme-less body through untouched).
    #[test]
    fn normalize_drops_git_plus_non_http_schemes() {
        for raw in ["git+javascript:alert(1)", "git+file:///etc/passwd"] {
            assert_eq!(normalize_repo_url(raw), "", "scheme survived: {raw:?}");
        }
    }

    /// The allowlist must not disturb any accepted
    /// shape — every rewrite branch already emits `https://`.
    #[test]
    fn normalize_accepted_shapes_still_round_trip() {
        for (raw, want) in [
            ("github:owner/repo", "https://github.com/owner/repo"),
            ("expressjs/express", "https://github.com/expressjs/express"),
            ("git+ssh://git@github.com/o/r.git", "https://github.com/o/r"),
            ("git://github.com/o/r", "https://github.com/o/r"),
            ("https://github.com/o/r", "https://github.com/o/r"),
            ("http://example.com/o/r", "http://example.com/o/r"),
        ] {
            assert_eq!(normalize_repo_url(raw), want, "input: {raw:?}");
        }
    }

    /// A repository URL whose authority carries RFC 3986 userinfo presents a
    /// github-looking host whose *effective* host is the attacker's. The
    /// clean-URL fall-through returns the trimmed input verbatim, so the
    /// final gate must drop it — the same drop-the-field policy every other
    /// authority-shaped defect gets.
    #[test]
    fn normalize_drops_userinfo_authority_in_clean_url() {
        assert_eq!(normalize_repo_url("https://github.com@evil.com/o/r"), "");
        assert_eq!(
            normalize_repo_url("https://github.com:secret@evil.com/o/r"),
            ""
        );
    }

    /// The `git://` branch rewrites through `scrub_authority_and_path`,
    /// which preserves the leading authority segment verbatim — so without
    /// the final gate `git://github.com@evil.com/o/r` would become a
    /// clickable `https://github.com@evil.com/o/r`. The `git+git://` twin
    /// routes through the same scrub.
    #[test]
    fn normalize_drops_userinfo_authority_in_git_scheme_branches() {
        assert_eq!(normalize_repo_url("git://github.com@evil.com/o/r"), "");
        assert_eq!(
            normalize_repo_url("git+git://github.com@evil.com/o/r.git"),
            ""
        );
    }

    /// An `@` in the **path** is legitimate (branch and
    /// tag names may carry it), and a numeric port in the authority
    /// (`host:22`) has no userinfo — both must keep round-tripping.
    #[test]
    fn normalize_keeps_port_and_path_ats() {
        assert_eq!(
            normalize_repo_url("https://host:22/owner/repo"),
            "https://host:22/owner/repo"
        );
        assert_eq!(
            normalize_repo_url("https://github.com/o/r/tree/HEAD/user@example.com"),
            "https://github.com/o/r/tree/HEAD/user@example.com"
        );
    }

    /// An `@` in the **query** or **fragment** — the parts after the
    /// authority's `?` / `#` delimiters — is legitimate too. The authority
    /// ends at the first of `/`, `?` or `#` (RFC 3986), so a query-only URL
    /// like `https://github.com?notify=a@b` must not be dropped as userinfo.
    #[test]
    fn normalize_keeps_query_and_fragment_ats() {
        assert_eq!(
            normalize_repo_url("https://github.com?assignee=a@b"),
            "https://github.com?assignee=a@b"
        );
        assert_eq!(
            normalize_repo_url("https://github.com#user@example.com"),
            "https://github.com#user@example.com"
        );
    }
}
