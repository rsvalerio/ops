//! Repository URL normalisation for `package.json::repository` values.
//!
//! ARCH-1 / TASK-0848: the `repository.url` rewriting surface is the
//! highest-risk code in the package_json module — see SEC-14 / TASK-0811
//! for the path-traversal fix that motivated this split. Living in its
//! own module makes future adversarial-input fixes have a clear test
//! target and a documented boundary, separate from the serde model and
//! the parse orchestrator in [`super::package_json`].

/// Detect ASCII / Unicode control characters (C0: U+0000..U+001F, DEL U+007F,
/// plus the broader `char::is_control` set covering C1 etc.) in a repository
/// URL body. SEC-2 / TASK-1165: previously these were silently filtered, so
/// `"github:owner/repo\nINJECT"` became `"https://github.com/owner/repoINJECT"`
/// — a clickable URL pointing at an attacker-named repo. The defence against
/// log-injection succeeded but the rendered URL was still attacker-chosen.
/// We now treat any control byte as evidence of tampering and the caller
/// drops the field entirely (returns an empty `String` from
/// [`normalize_repo_url`]), so the About card surfaces no link at all rather
/// than a silently rewritten one.
fn contains_control_chars(raw: &str) -> bool {
    raw.chars().any(|c| c.is_control() || c == '\u{007f}')
}

/// Normalise a `repository` URL value: turn npm shorthand
/// (`github:owner/repo`), git+ssh, ssh scp form, git+https, and the bare
/// `.git` suffix into a plain `https://host/path` shape that renders
/// cleanly in the About card.
///
/// SSH URL handling is delegated to [`ssh_to_https`].
///
/// SEC-2 / TASK-1080: control characters (CR, LF, ANSI escape, other
/// C0 / DEL) are stripped from the URL body before any prefix logic
/// runs, so a `"github:owner/repo\nINJECT"` repository field cannot
/// inject a newline into the rendered link or a debug-log line.
///
/// SEC-2 / TASK-1165: a `repository` containing any control byte is
/// dropped entirely (returns an empty `String`) rather than silently
/// concatenated. The previous filter let `"github:owner/repo\nINJECT"`
/// normalise to `"https://github.com/owner/repoINJECT"` — a clickable
/// URL pointing at an attacker-chosen repo. Returning empty surfaces
/// the field as missing in the About card / markdown / HTML and avoids
/// the silent-rewrite. Callers (`package_json::parse_package_json`)
/// treat the empty result the same as a missing repository field.
/// PERF-3 / TASK-1257: returns `Cow<'_, str>` so a well-formed
/// `https://github.com/owner/repo` URL with no rewrites required passes
/// straight through as `Cow::Borrowed`. Callers that need owned `String`
/// can `.into_owned()`. Branches that rewrite the URL (npm shorthand,
/// SSH form, scrubbing) still allocate; only the fall-through clean path
/// stays alloc-free, where it dominates the per-`parse_package_json`
/// invocation count.
pub(crate) fn normalize_repo_url(raw: &str) -> std::borrow::Cow<'_, str> {
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
    // PERF-3 / TASK-1257: clean URL fall-through — borrow the trimmed slice
    // so a well-formed `https://github.com/owner/repo` (no `.git` suffix)
    // returns `Cow::Borrowed` and the per-call allocation drops to zero.
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
/// PATTERN-1 / TASK-0692: distinguish a numeric port from an scp-form path
/// whose first segment merely begins with a digit (e.g. `host:42-archive/x`)
/// by requiring **all** characters before the next `/` to be digits — a
/// `host:42/foo` is a port, `host:42-archive/x` is an scp-form path.
pub(crate) fn ssh_to_https(rest: &str) -> String {
    let no_user = rest.strip_prefix("git@").unwrap_or(rest);
    let trimmed = no_user.trim_end_matches(".git");
    let body = match trimmed.split_once(':') {
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
/// SEC-14 / TASK-0811: any path component equal to `..` (or any leading
/// absolute slash) is dropped before the suffix is built. An adversarial
/// `package.json` can otherwise emit a directory like `../../../etc/passwd`,
/// which the previous implementation passed through verbatim and produced a
/// traversal-shaped URL rendered into About cards / markdown / HTML. Empty
/// segments and `.` segments are also collapsed for the same reason. If
/// every component is filtered out, the directory suffix is omitted and the
/// base URL is returned unchanged.
pub(crate) fn append_tree_directory(base: &str, directory: &str) -> String {
    let normalized = directory.trim().trim_start_matches("./");
    let cleaned = scrub_path_segments(normalized);
    if cleaned.is_empty() {
        return base.to_string();
    }
    let trimmed_base = base.trim_end_matches('/');
    format!("{trimmed_base}/tree/HEAD/{cleaned}")
}

/// Drop empty, `.`, and `..` segments from a `/`-separated path. SEC-14 /
/// TASK-1111: shared scrub used by the npm-shorthand and `git://` branches
/// of [`normalize_repo_url`]. Mirrors the segment filter in
/// [`append_tree_directory`] (SEC-14 / TASK-0811) so adversarial
/// `repository` values like `github:../../etc/passwd` cannot produce a
/// traversal-shaped URL in rendered About output.
fn scrub_path_segments(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..")
        .collect::<Vec<_>>()
        .join("/")
}

/// Scrub path traversal from a `host[/path]` body where the leading
/// segment is the authority (host[:port]) and must be preserved verbatim.
/// SEC-14 / TASK-1111: the `git://`, `git+git://`, and `git+<scheme>://`
/// branches of [`normalize_repo_url`] all carry an authority followed by a
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
/// SEC-14 / TASK-1111: the `git+<scheme>://` branch of [`normalize_repo_url`]
/// returns the URL with the scheme intact; we must only scrub the path
/// portion, leaving `scheme://` and the authority alone (otherwise `https://`
/// collapses to `https:/` because the empty segment between the two slashes
/// is filtered).
fn scrub_full_url_path(url: &str) -> String {
    if let Some((scheme, rest)) = url.split_once("://") {
        format!("{scheme}://{}", scrub_authority_and_path(rest))
    } else {
        scrub_authority_and_path(url)
    }
}

pub(crate) fn is_numeric_port_prefix(path: &str) -> bool {
    let port = path.split('/').next().unwrap_or("");
    !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PERF-3 / TASK-1257: a well-formed `https://github.com/...` URL with
    /// no rewrites required must pass through as `Cow::Borrowed`, leaving
    /// the per-call allocation count at zero on the dominant clean path.
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

    #[test]
    fn normalize_ssh_with_numeric_port_keeps_port() {
        assert_eq!(
            normalize_repo_url("ssh://git@host:22/path.git"),
            "https://host:22/path"
        );
    }

    /// SEC-14 / TASK-0811: a `directory` that escapes the repository root via
    /// `..` segments must be sanitized — the URL is rendered into About cards
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
        // The motivating case from the SEC-14 finding: an adversarial
        // package.json must not produce a URL whose path component contains
        // `../../etc/passwd` style traversal.
        let url = append_tree_directory("https://github.com/o/r", "../../../../etc/passwd");
        assert!(!url.contains(".."), "url still contains ..: {url}");
        assert_eq!(url, "https://github.com/o/r/tree/HEAD/etc/passwd");
    }

    /// PATTERN-1 / TASK-1049: `git+git://` must be rewritten to `https://`
    /// — otherwise the About card renders an unclickable `git://` URL.
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

    /// PATTERN-1 / TASK-1060: bare `owner/repo` npm shorthand must be
    /// rewritten to a GitHub URL — otherwise the About card emits a
    /// non-URL link.
    #[test]
    fn normalize_bare_owner_repo_shorthand() {
        assert_eq!(
            normalize_repo_url("expressjs/express"),
            "https://github.com/expressjs/express"
        );
    }

    /// SEC-14 / TASK-1205: the bare-shorthand branch must not surface a
    /// traversal-shaped URL. Pre-TASK-1205 a `package.json` with
    /// `"repository": "../etc"` produced `https://github.com/../etc`
    /// — sister branches (`github:`/`git://`/`git+*://`) all routed
    /// through `scrub_path_segments` per SEC-14 / TASK-1111, but the
    /// bare branch was added by PATTERN-1 / TASK-1060 without the same
    /// scrub. We pin both AC outcomes here:
    /// 1. The rendered URL contains no literal `..`.
    /// 2. `../etc` lands on `https://github.com/etc` (the `..` segment
    ///    is filtered, `etc` survives).
    #[test]
    fn normalize_bare_shorthand_strips_traversal() {
        let out = normalize_repo_url("../etc");
        assert!(!out.contains(".."), "url still contains ..: {out}");
        assert_eq!(out, "https://github.com/etc");
    }

    /// SEC-14 / TASK-1205: a bare shorthand whose every segment is
    /// `.`/`..` collapses to the bare host, mirroring the shape
    /// `normalize_github_shorthand_pure_traversal_collapses_to_host`
    /// already pins for the explicit `github:` branch.
    #[test]
    fn normalize_bare_shorthand_pure_traversal_collapses_to_host() {
        assert_eq!(normalize_repo_url("../.."), "https://github.com");
    }

    /// PATTERN-1 / TASK-1060: a scoped npm package name like `@scope/name`
    /// is NOT a repo shorthand and must fall through unchanged.
    #[test]
    fn normalize_scoped_npm_name_falls_through() {
        assert_eq!(normalize_repo_url("@scope/name"), "@scope/name");
    }

    /// SEC-2 / TASK-1080 + TASK-1165: an embedded LF inside a `github:`
    /// shorthand must not survive into the rendered URL — and per
    /// TASK-1165 the field is now dropped entirely so the silent
    /// concatenation `repo\nINJECT → repoINJECT` (a clickable
    /// attacker-chosen URL) cannot reach About cards / markdown / HTML.
    #[test]
    fn normalize_drops_field_on_embedded_lf_in_shorthand() {
        let out = normalize_repo_url("github:owner/repo\nINJECT");
        assert!(!out.contains('\n'), "url still contains LF: {out:?}");
        assert!(
            out.is_empty(),
            "field must be dropped on control byte: {out:?}"
        );
    }

    /// SEC-2 / TASK-1080 + TASK-1165: a CR inside a git+https URL
    /// (Object{url} shape) drops the field. Pins the behaviour for the
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

    /// SEC-2 / TASK-1080 + TASK-1165: ANSI escape (U+001B) bytes drop
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

    /// SEC-2 / TASK-1080 + TASK-1165: the Text shape
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

    /// SEC-2 / TASK-1165: a tampered URL containing a control byte must
    /// NOT produce a syntactically valid URL pointing at attacker-chosen
    /// path segments. Pins the broader contract directly so future
    /// changes that re-introduce silent concatenation regress here.
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

    /// SEC-14 / TASK-1111: a `github:` shorthand carrying `..` segments
    /// must not produce a traversal-shaped URL — the same threat model as
    /// [`append_tree_directory`] (TASK-0811). The scrub drops every empty,
    /// `.`, and `..` segment before interpolation.
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

    /// SEC-14 / TASK-1111: a shorthand whose suffix is purely traversal
    /// collapses to the bare host — same shape as
    /// `append_tree_directory` returning the base URL when every
    /// component filters out.
    #[test]
    fn normalize_github_shorthand_pure_traversal_collapses_to_host() {
        assert_eq!(normalize_repo_url("github:../../.."), "https://github.com");
    }

    /// SEC-14 / TASK-1111: the bare `git://` branch must scrub `..` too.
    #[test]
    fn normalize_git_scheme_strips_traversal() {
        let out = normalize_repo_url("git://github.com/../../etc/passwd");
        assert!(!out.contains(".."), "url still contains ..: {out}");
        assert_eq!(out, "https://github.com/etc/passwd");
    }

    /// SEC-14 / TASK-1111: `git+git://` shares the scrub policy with the
    /// bare `git://` branch (PATTERN-1 / TASK-1049 rewrites to https).
    #[test]
    fn normalize_git_plus_git_scheme_strips_traversal() {
        let out = normalize_repo_url("git+git://github.com/../../etc/passwd");
        assert!(!out.contains(".."), "url still contains ..: {out}");
        assert_eq!(out, "https://github.com/etc/passwd");
    }

    /// SEC-14 / TASK-1111: `git+<scheme>://` (e.g. `git+https://`) also
    /// scrubs `..` from the path component before rendering into the
    /// About card.
    #[test]
    fn normalize_git_plus_https_strips_traversal() {
        let out = normalize_repo_url("git+https://github.com/o/../../etc/passwd.git");
        assert!(!out.contains(".."), "url still contains ..: {out}");
        assert_eq!(out, "https://github.com/o/etc/passwd");
    }

    /// DUP-3 / TASK-1122: both `normalize_repo_url`'s shorthand branch and
    /// `append_tree_directory` route segment scrubbing through
    /// [`scrub_path_segments`], so a future tightening of the SEC-14 filter
    /// (Unicode bidi controls, encoded `..`, etc.) only needs to land in
    /// one place. Pin equivalence on a `..`-laden input.
    #[test]
    fn append_tree_directory_and_shorthand_share_segment_filter() {
        // Both entry points must drop `..` and `.` segments identically.
        let tree = append_tree_directory("https://github.com/o/r", "a/../b/./c");
        assert_eq!(tree, "https://github.com/o/r/tree/HEAD/a/b/c");
        let shorthand = normalize_repo_url("github:a/../b/./c");
        assert_eq!(shorthand, "https://github.com/a/b/c");
    }

    /// SEC-2 / TASK-1080: a debug-log of the normalised URL must remain
    /// single-line — i.e. the `Debug`/`Display` rendering after
    /// normalisation contains no embedded newlines.
    #[test]
    fn normalize_debug_log_stays_single_line() {
        let out = normalize_repo_url("github:owner/repo\nINJECT\rMORE");
        let debug = format!("{out:?}");
        let display = out.to_string();
        assert!(!debug.contains('\n') && !debug.contains('\r'));
        assert!(!display.contains('\n') && !display.contains('\r'));
        // SEC-2 / TASK-1165: the dropped-field policy means the rendered
        // string is empty, not a silent rewrite to attacker-chosen segments.
        assert!(out.is_empty(), "field must be dropped on control byte");
    }
}
