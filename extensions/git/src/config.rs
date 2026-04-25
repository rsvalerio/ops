//! Read local `.git` directory metadata without shelling out to `git`.

use std::path::Path;

pub use ops_hook_common::find_git_dir;

/// Read the URL of the `origin` remote from `<git_dir>/config`.
pub fn read_origin_url(git_dir: &Path) -> Option<String> {
    let content = std::fs::read_to_string(git_dir.join("config")).ok()?;
    read_origin_url_from(&content)
}

/// Parse a git-config body and return the `[remote "origin"]` url.
///
/// Limitations: this is a minimal line scanner, not a conformant git-config
/// parser. It does **not** honour `[url "<base>"] insteadOf = ...` rewrites,
/// continuation lines, escaped quotes, or `include.path` directives. Comments
/// (`#` / `;`) starting a line are skipped; everything else falls through.
/// Section headers and the `url` key are matched case-insensitively, since
/// git-config keys are case-insensitive.
pub fn read_origin_url_from(content: &str) -> Option<String> {
    let mut in_origin = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with('[') {
            in_origin = is_origin_header(trimmed);
            continue;
        }
        if in_origin {
            if let Some(value) = strip_url_key(trimmed) {
                return Some(redact_userinfo(value));
            }
        }
    }
    None
}

/// Strip a `user[:password]@` segment from a URL-like value.
///
/// Git supports embedding HTTP credentials directly in remote URLs. We never
/// want those reaching logs, error messages, or data-provider output, so any
/// raw value coming out of `.git/config` is scrubbed at the source.
pub(crate) fn redact_userinfo(value: &str) -> String {
    let Some((scheme, after)) = value.split_once("://") else {
        return value.to_string();
    };
    let (authority, rest) = match after.split_once('/') {
        Some((a, r)) => (a, Some(r)),
        None => (after, None),
    };
    let host = authority.rsplit('@').next().unwrap_or(authority);
    match rest {
        Some(r) => format!("{scheme}://{host}/{r}"),
        None => format!("{scheme}://{host}"),
    }
}

fn strip_url_key(line: &str) -> Option<&str> {
    let (key, value) = line.split_once('=')?;
    if key.trim().eq_ignore_ascii_case("url") {
        Some(value.trim())
    } else {
        None
    }
}

fn is_origin_header(line: &str) -> bool {
    // Accept `[remote "origin"]` with any surrounding whitespace.
    let inner = line.trim_start_matches('[').trim_end_matches(']').trim();
    let mut parts = inner.splitn(2, char::is_whitespace);
    let section = parts.next().unwrap_or("");
    let subsection = parts.next().unwrap_or("").trim();
    section == "remote" && (subsection == "\"origin\"" || subsection == "origin")
}

/// Read the current branch from `<git_dir>/HEAD`. Returns `None` on detached HEAD.
pub fn read_head_branch(git_dir: &Path) -> Option<String> {
    let content = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let trimmed = content.trim();
    let rest = trimmed.strip_prefix("ref:")?.trim();
    let branch = rest.strip_prefix("refs/heads/")?;
    if branch.is_empty() {
        None
    } else {
        Some(branch.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_git_dir_in_current() {
        let dir = tempfile::tempdir().unwrap();
        let git = dir.path().join(".git");
        std::fs::create_dir(&git).unwrap();
        let expected = std::fs::canonicalize(&git).unwrap();
        assert_eq!(find_git_dir(dir.path()), Some(expected));
    }

    #[test]
    fn find_git_dir_in_parent() {
        let dir = tempfile::tempdir().unwrap();
        let git = dir.path().join(".git");
        std::fs::create_dir(&git).unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let expected = std::fs::canonicalize(&git).unwrap();
        assert_eq!(find_git_dir(&sub), Some(expected));
    }

    #[test]
    fn find_git_dir_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(find_git_dir(dir.path()).is_none());
    }

    #[test]
    fn origin_url_https() {
        let cfg = "\
[core]
\trepositoryformatversion = 0
[remote \"origin\"]
\turl = https://github.com/openbao/openbao.git
\tfetch = +refs/heads/*:refs/remotes/origin/*
";
        assert_eq!(
            read_origin_url_from(cfg),
            Some("https://github.com/openbao/openbao.git".to_string())
        );
    }

    #[test]
    fn origin_url_ssh() {
        let cfg = "\
[remote \"origin\"]
\turl = git@github.com:openbao/openbao.git
";
        assert_eq!(
            read_origin_url_from(cfg),
            Some("git@github.com:openbao/openbao.git".to_string())
        );
    }

    #[test]
    fn origin_section_skipped_when_other_remote() {
        let cfg = "\
[remote \"upstream\"]
\turl = https://example.com/other/repo.git
[remote \"origin\"]
\turl = https://github.com/real/repo.git
";
        assert_eq!(
            read_origin_url_from(cfg),
            Some("https://github.com/real/repo.git".to_string())
        );
    }

    #[test]
    fn no_origin_section_returns_none() {
        let cfg = "\
[remote \"upstream\"]
\turl = https://example.com/other/repo.git
";
        assert!(read_origin_url_from(cfg).is_none());
    }

    #[test]
    fn read_origin_url_reads_file() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = https://github.com/o/r.git\n",
        )
        .unwrap();
        assert_eq!(
            read_origin_url(&git_dir),
            Some("https://github.com/o/r.git".to_string())
        );
    }

    #[test]
    fn embedded_credentials_are_redacted() {
        let cfg = "[remote \"origin\"]\n\turl = https://user:token@github.com/o/r.git\n";
        let url = read_origin_url_from(cfg).expect("origin url");
        assert!(!url.contains("user:token"), "leaked credentials: {url}");
        assert!(!url.contains('@'), "retained userinfo: {url}");
        assert_eq!(url, "https://github.com/o/r.git");
    }

    #[test]
    fn url_key_is_case_insensitive() {
        let cfg = "[remote \"origin\"]\n\tURL = https://github.com/o/r.git\n";
        assert_eq!(
            read_origin_url_from(cfg),
            Some("https://github.com/o/r.git".to_string())
        );
    }

    #[test]
    fn comment_lines_are_skipped() {
        let cfg = "[remote \"origin\"]\n# url = https://commented.example/x.git\n\turl = https://real.example/y.git\n";
        assert_eq!(
            read_origin_url_from(cfg),
            Some("https://real.example/y.git".to_string())
        );
    }

    #[test]
    fn head_branch_from_ref() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        assert_eq!(read_head_branch(&git_dir), Some("main".to_string()));
    }

    #[test]
    fn head_branch_with_slashes() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/feature/foo\n").unwrap();
        assert_eq!(read_head_branch(&git_dir), Some("feature/foo".to_string()));
    }

    #[test]
    fn head_detached_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("HEAD"),
            "0123456789abcdef0123456789abcdef01234567\n",
        )
        .unwrap();
        assert!(read_head_branch(&git_dir).is_none());
    }
}
