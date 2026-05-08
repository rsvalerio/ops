//! Repository URL normalisation for `package.json::repository` values.
//!
//! ARCH-1 / TASK-0848: the `repository.url` rewriting surface is the
//! highest-risk code in the package_json module — see SEC-14 / TASK-0811
//! for the path-traversal fix that motivated this split. Living in its
//! own module makes future adversarial-input fixes have a clear test
//! target and a documented boundary, separate from the serde model and
//! the parse orchestrator in [`super::package_json`].

/// Strip ASCII control characters (C0: U+0000..U+001F, plus DEL U+007F)
/// from a repository URL body. SEC-2 / TASK-1080: an adversarial
/// `package.json` `repository.url` like `"github:owner/repo\nINJECT"`
/// flows verbatim into About cards, markdown, HTML, and operator-facing
/// log lines. Sister policy to the SEC-14 traversal fix
/// ([`append_tree_directory`]) and the ERR-7 path-debug-escape pattern:
/// repository URLs are operator-facing surfaces and must be single-line
/// and free of ANSI escape (U+001B) injection.
fn strip_control_chars(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_control() && *c != '\u{007f}')
        .collect()
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
pub(crate) fn normalize_repo_url(raw: &str) -> String {
    /// (shorthand prefix, host) for npm hostname shortcuts.
    const HOST_PREFIXES: &[(&str, &str)] = &[
        ("github:", "github.com"),
        ("gitlab:", "gitlab.com"),
        ("bitbucket:", "bitbucket.org"),
    ];

    let sanitised = strip_control_chars(raw);
    let s = sanitised.trim();
    for (prefix, host) in HOST_PREFIXES {
        if let Some(rest) = s.strip_prefix(prefix) {
            return format!("https://{host}/{rest}");
        }
    }
    if let Some(rest) = s
        .strip_prefix("git+ssh://")
        .or_else(|| s.strip_prefix("ssh://"))
    {
        return ssh_to_https(rest);
    }
    if let Some(rest) = s.strip_prefix("git+") {
        let trimmed = rest.trim_end_matches(".git");
        // PATTERN-1 / TASK-1049: a `git+git://` URL must be rewritten to
        // `https://` like the bare `git://` branch below — otherwise the
        // About card renders an unclickable `git://` link.
        if let Some(after) = trimmed.strip_prefix("git://") {
            return format!("https://{after}");
        }
        return trimmed.to_string();
    }
    if let Some(rest) = s.strip_prefix("git://") {
        return format!("https://{}", rest.trim_end_matches(".git"));
    }
    // PATTERN-1 / TASK-1060: bare two-segment npm shorthand
    // (`owner/repo`) — no scheme, no colon, exactly one `/`, both
    // segments non-empty and identifier-shaped — is rewritten to a
    // GitHub URL. Otherwise the About card surfaces a non-URL string
    // as a link. Scoped npm names (`@scope/name`) are intentionally
    // excluded: they're package names, not repo shorthands.
    if is_bare_github_shorthand(s) {
        return format!("https://github.com/{s}");
    }
    s.trim_end_matches(".git").to_string()
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
    let normalized = directory.trim().trim_start_matches("./").replace('\\', "/");
    let cleaned = normalized
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..")
        .collect::<Vec<_>>()
        .join("/");
    if cleaned.is_empty() {
        return base.to_string();
    }
    let trimmed_base = base.trim_end_matches('/');
    format!("{trimmed_base}/tree/HEAD/{cleaned}")
}

pub(crate) fn is_numeric_port_prefix(path: &str) -> bool {
    let port = path.split('/').next().unwrap_or("");
    !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// PATTERN-1 / TASK-1060: a scoped npm package name like `@scope/name`
    /// is NOT a repo shorthand and must fall through unchanged.
    #[test]
    fn normalize_scoped_npm_name_falls_through() {
        assert_eq!(normalize_repo_url("@scope/name"), "@scope/name");
    }

    /// SEC-2 / TASK-1080: an embedded LF inside a `github:` shorthand must
    /// not survive into the rendered URL — it would otherwise inject a
    /// newline into About cards, markdown, HTML, and debug log lines.
    #[test]
    fn normalize_strips_embedded_lf_in_shorthand() {
        let out = normalize_repo_url("github:owner/repo\nINJECT");
        assert!(!out.contains('\n'), "url still contains LF: {out:?}");
        assert_eq!(out, "https://github.com/owner/repoINJECT");
    }

    /// SEC-2 / TASK-1080: a CR inside a git+https URL (Object{url} shape)
    /// is stripped before normalisation. Pins the behaviour for the
    /// `repository: { url: "..." }` parse path, which routes through the
    /// same `normalize_repo_url` entry point.
    #[test]
    fn normalize_strips_embedded_cr_in_git_https() {
        let out = normalize_repo_url("git+https://github.com/o/r\r.git");
        assert!(!out.contains('\r'), "url still contains CR: {out:?}");
        assert_eq!(out, "https://github.com/o/r");
    }

    /// SEC-2 / TASK-1080: ANSI escape (U+001B) bytes embedded in the URL
    /// body must not flow into operator-facing surfaces (About cards,
    /// log lines) where they would be interpreted as terminal escapes.
    #[test]
    fn normalize_strips_embedded_ansi_escape() {
        let out = normalize_repo_url("https://github.com/o/\u{1b}[31mr");
        assert!(!out.contains('\u{1b}'), "url still contains ESC: {out:?}");
        assert_eq!(out, "https://github.com/o/[31mr");
    }

    /// SEC-2 / TASK-1080: the Text shape (`repository: "github:..."`)
    /// must also be sanitised. Pins the regression for both repository
    /// field shapes — Text and Object{url} — feeding the same helper.
    #[test]
    fn normalize_strips_control_chars_in_text_shape() {
        let out = normalize_repo_url("github:owner/repo\rINJECT\nMORE");
        assert!(
            !out.contains('\r') && !out.contains('\n'),
            "url still contains control chars: {out:?}"
        );
        assert_eq!(out, "https://github.com/owner/repoINJECTMORE");
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
    }
}
