//! Read local `.git` directory metadata without shelling out to `git`.

use std::path::Path;

pub use ops_hook_common::find_git_dir;

/// ARCH-2 / SEC-13 / TASK-0894: type-system-enforced "this URL has been
/// scrubbed of `user[:password]@` userinfo". The only ways to construct
/// one are [`RedactedUrl::redact`] (runs `redact_userinfo`) and the
/// `From<&str>` impl that delegates to it. Carrying a `RedactedUrl`
/// through the call chain means a future refactor cannot accidentally
/// route a raw URL into [`crate::GitInfo::remote_url`] / about cards /
/// JSON output without a visible `RedactedUrl::redact` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedUrl(String);

impl RedactedUrl {
    /// Construct from a raw URL by stripping `user[:password]@` userinfo.
    /// `redact_userinfo` is idempotent so calling this on an already-clean
    /// value is a no-op.
    ///
    /// ```
    /// use ops_git::config::RedactedUrl;
    /// let r = RedactedUrl::redact("https://alice:secret@github.com/o/r.git");
    /// assert_eq!(r.as_str(), "https://github.com/o/r.git");
    /// // Idempotent: re-redacting an already-clean value is a no-op.
    /// let r2 = RedactedUrl::redact(r.as_str());
    /// assert_eq!(r2.as_str(), r.as_str());
    /// ```
    #[must_use]
    pub fn redact(raw: &str) -> Self {
        Self(redact_userinfo(raw))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<&str> for RedactedUrl {
    fn from(raw: &str) -> Self {
        Self::redact(raw)
    }
}

impl std::fmt::Display for RedactedUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// SEC-33 / TASK-0910: hard cap on `.git/config` read size. A real-world
/// git config is well under 64 KiB; an adversarial repo (cloned for
/// inspection) could otherwise OOM the CLI through a multi-GB file or a
/// symlink to `/dev/zero`. Mirrors the
/// `ops_about::manifest_io::MAX_MANIFEST_BYTES` posture for project
/// manifests.
pub const MAX_GIT_CONFIG_BYTES: u64 = 4 * 1024 * 1024;

/// Read the URL of the `origin` remote from `<git_dir>/config`.
///
/// NotFound is silent (no remotes configured is normal). Other IO errors
/// (PermissionDenied, IsADirectory, etc.) log at `tracing::warn!` before
/// returning None, matching the policy of `try_read_manifest` (TASK-0548)
/// and `resolve_member_globs` (TASK-0517).
///
/// SEC-33 / TASK-0910: the read is capped at [`MAX_GIT_CONFIG_BYTES`]
/// via `File::open` + `Read::take`. An oversized config returns `None`
/// with a `tracing::warn!` rather than slurping the whole file.
pub fn read_origin_url(git_dir: &Path) -> Option<RedactedUrl> {
    use std::io::Read;
    let path = git_dir.join("config");
    let mut file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to open .git/config; treating as no remote"
            );
            return None;
        }
    };
    let mut content = String::new();
    let limit = MAX_GIT_CONFIG_BYTES.saturating_add(1);
    if let Err(e) = (&mut file).take(limit).read_to_string(&mut content) {
        tracing::warn!(
            path = %path.display(),
            error = %e,
            "failed to read .git/config (within byte cap); treating as no remote"
        );
        return None;
    }
    if content.len() as u64 > MAX_GIT_CONFIG_BYTES {
        tracing::warn!(
            path = %path.display(),
            cap = MAX_GIT_CONFIG_BYTES,
            "SEC-33: .git/config exceeds byte cap; refusing to parse and treating as no remote"
        );
        return None;
    }
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
///
/// ERR-4 (TASK-0594): git-config keys are multi-valued and the *last*
/// assignment wins (templated includes routinely rewrite `url` after an
/// initial value). Returning the first match silently disagreed with what
/// `git config --get remote.origin.url` reports. The scanner now collects
/// every `url` line inside the `origin` section across the file and returns
/// the final one so the parser matches git-config last-wins semantics.
///
/// READ-2 (TASK-0726): inline trailing comments (`url = … ; old`) are
/// stripped from unquoted values, matching `git config --get`. Quoted
/// values are not yet honoured by this minimal scanner.
///
/// # Userinfo redaction (SEC-13 / TASK-0894)
///
/// Returns a [`RedactedUrl`] — the type system enforces that any
/// `user[:password]@` userinfo is stripped before the value reaches a
/// caller. Callers cannot route the inner string into about-cards / JSON
/// without an explicit `into_string()` / `as_str()` call, which makes a
/// future credential-leak refactor visible at the call site instead of
/// silent.
pub fn read_origin_url_from(content: &str) -> Option<RedactedUrl> {
    let mut in_origin = false;
    let mut origin_seen = false;
    let mut last: Option<RedactedUrl> = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with('[') {
            in_origin = is_origin_header(trimmed);
            if in_origin {
                origin_seen = true;
            }
            continue;
        }
        if in_origin {
            if let Some(value) = strip_url_key(trimmed) {
                last = Some(RedactedUrl::redact(value));
            }
        }
    }
    // TASK-0966: distinguish "no [remote \"origin\"] section" (silent) from
    // "section present but every url= line was malformed / empty" (one-line
    // breadcrumb). Operators chasing "branch shows but remote_url is None"
    // otherwise get no signal pointing at the corrupted config.
    if origin_seen && last.is_none() {
        tracing::debug!(
            section = "remote \"origin\"",
            "git-config: origin section present but no extractable url= line"
        );
    }
    last
}

/// Strip a `user[:password]@` segment from a URL-like value.
///
/// Git supports embedding HTTP credentials directly in remote URLs. We never
/// want those reaching logs, error messages, or data-provider output, so any
/// raw value coming out of `.git/config` is scrubbed at the source.
///
/// Both scheme-form (`https://user:tok@host/path`) and scp-form
/// (`user@host:owner/repo`) inputs are scrubbed; scp-form is detected as a
/// non-`://` value containing `@` before the first `/`.
pub(crate) fn redact_userinfo(value: &str) -> String {
    if let Some((scheme, after)) = value.split_once("://") {
        let (authority, rest) = match after.split_once('/') {
            Some((a, r)) => (a, Some(r)),
            None => (after, None),
        };
        let host = authority.rsplit('@').next().unwrap_or(authority);
        return match rest {
            Some(r) => format!("{scheme}://{host}/{r}"),
            None => format!("{scheme}://{host}"),
        };
    }
    // scp-style: strip a `user[:password]@` prefix that appears before the
    // first `/`. Past the first `/` the `@` belongs to a path component, not
    // userinfo.
    let head_end = value.find('/').unwrap_or(value.len());
    let head = &value[..head_end];
    if let Some(at_idx) = head.rfind('@') {
        let mut redacted = String::with_capacity(value.len() - at_idx);
        redacted.push_str(&value[at_idx + 1..]);
        return redacted;
    }
    value.to_string()
}

fn strip_url_key(line: &str) -> Option<&str> {
    let (key, value) = line.split_once('=')?;
    if !key.trim().eq_ignore_ascii_case("url") {
        return None;
    }
    // READ-2 (TASK-0726): drop trailing inline comments (`#`, `;`) so the
    // returned value matches `git config --get remote.origin.url`. The
    // minimal scanner does not yet support quoted values; in the unquoted
    // case any `#`/`;` ends the value.
    let comment_start = value.find(['#', ';']).unwrap_or(value.len());
    Some(value[..comment_start].trim())
}

fn is_origin_header(line: &str) -> bool {
    match parse_section_header(line) {
        Ok((section, subsection)) => {
            // Section names in git-config(1) are case-insensitive:
            // `[Remote "origin"]` and `[REMOTE "origin"]` are valid and
            // accepted by git itself, so the matcher must not require
            // lowercase. Subsection names *are* case-sensitive per git, so
            // leave that comparison exact. The bare-word form
            // `[remote origin]` is malformed and rejected by git itself, so
            // this helper requires the canonical quoted form.
            section.eq_ignore_ascii_case("remote") && subsection.as_deref() == Some("origin")
        }
        Err(reason) => {
            // READ-5 / TASK-1006: a malformed header for a section we
            // would otherwise care about (e.g. an attacker-shaped
            // subsection escape, an unbalanced quote) used to drop the
            // entire section silently — operators saw "remote URL not
            // detected" and no log entry. Surface the specific failure
            // category at debug so a `RUST_LOG=ops_git=debug` rerun
            // explains the absence.
            if line
                .trim_start_matches('[')
                .starts_with(|c: char| c.eq_ignore_ascii_case(&'r'))
            {
                tracing::debug!(
                    line,
                    reason = ?reason,
                    "git-config: rejected section header that looks like remote.*"
                );
            }
            false
        }
    }
}

/// READ-5 / TASK-1006: typed reason for a [`parse_section_header`] reject so
/// callers can surface the specific failure category in their logs instead
/// of collapsing every malformed header into a silent `None`.
#[derive(Debug)]
enum SectionHeaderError {
    NotASectionHeader,
    UnbalancedQuotes,
    UnknownEscape,
    UnterminatedEscape,
}

/// Parse a git-config section header `[section "subsection"]` into its parts.
///
/// Decodes the two escapes git recognises inside subsection names (`\\` → `\`,
/// `\"` → `"`) and rejects the bare-word form `[section subsection]` that
/// git itself does not honour. Returns a typed [`SectionHeaderError`] so
/// callers can log the specific failure category.
fn parse_section_header(line: &str) -> Result<(&str, Option<String>), SectionHeaderError> {
    let inner = line
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or(SectionHeaderError::NotASectionHeader)?
        .trim();
    let (section, rest) = match inner.split_once(char::is_whitespace) {
        Some((s, r)) => (s, r.trim()),
        None => return Ok((inner, None)),
    };
    let body = rest
        .strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .ok_or(SectionHeaderError::UnbalancedQuotes)?;
    let mut decoded = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('\\') => decoded.push('\\'),
                Some('"') => decoded.push('"'),
                Some(_) => return Err(SectionHeaderError::UnknownEscape),
                None => return Err(SectionHeaderError::UnterminatedEscape),
            }
        } else {
            decoded.push(c);
        }
    }
    Ok((section, Some(decoded)))
}

/// Read the current branch from `<git_dir>/HEAD`. Returns `None` on detached HEAD.
///
/// ERR-1 / TASK-0887: mirrors the policy already applied to
/// [`read_origin_url`] — silent on `NotFound` (legitimately absent for some
/// repository states), `tracing::warn!` on every other IO error so an
/// operator chasing "branch keeps showing as detached" sees the underlying
/// permission/EIO problem instead of a `None` that pretends HEAD is detached.
pub fn read_head_branch(git_dir: &Path) -> Option<String> {
    let head_path = git_dir.join("HEAD");
    let content = match std::fs::read_to_string(&head_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            tracing::warn!(
                path = %head_path.display(),
                error = %e,
                "failed to read .git/HEAD; reporting branch as None"
            );
            return None;
        }
    };
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
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("https://github.com/openbao/openbao.git".to_string())
        );
    }

    #[test]
    fn origin_url_ssh() {
        let cfg = "\
[remote \"origin\"]
\turl = git@github.com:openbao/openbao.git
";
        // SEC-13 (TASK-0664): redact_userinfo now strips the `user@` prefix
        // from scp-style URLs as well. The conventional `git@` is treated as
        // userinfo for redaction purposes; downstream `parse_remote_url`
        // accepts the trimmed scp form.
        assert_eq!(
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("github.com:openbao/openbao.git".to_string())
        );
    }

    /// SEC-13 (TASK-0664): scp-style remotes that fall through unparseable
    /// must not surface embedded credentials. `read_origin_url_from` now
    /// redacts the `user[:tok]@` prefix on non-`://` values too.
    #[test]
    fn scp_style_credentials_are_redacted() {
        let cfg = "[remote \"origin\"]\n\turl = user:tok@host:weird/garbage\n";
        let url = read_origin_url_from(cfg)
            .map(RedactedUrl::into_string)
            .expect("origin url");
        assert!(!url.contains("user:tok"), "leaked credentials: {url}");
        assert!(!url.contains('@'), "retained userinfo: {url}");
        assert_eq!(url, "host:weird/garbage");
    }

    /// TASK-0966: a `[remote "origin"]` section that exists but has no valid
    /// `url = ...` line returns None and emits one `tracing::debug` breadcrumb.
    /// A genuinely-missing origin section stays silent. The breadcrumb itself
    /// is verified via `tracing-test`-free assertion: we only pin the return
    /// value here and rely on the inline `tracing::debug!` survival in the
    /// source — call-site presence is guarded by code review.
    #[test]
    /// READ-5 / TASK-1006: a malformed escape in a `[remote "…"]` header
    /// returns a typed `SectionHeaderError` rather than collapsing the
    /// whole section silently. The behaviour-pinning assertion is that
    /// `parse_section_header` reports a typed error so `is_origin_header`
    /// can log a debug breadcrumb naming the failure category.
    #[test]
    fn parse_section_header_unknown_escape_returns_typed_error() {
        let line = r#"[remote "ori\nin"]"#;
        let err = parse_section_header(line).unwrap_err();
        assert!(
            matches!(err, SectionHeaderError::UnknownEscape),
            "expected UnknownEscape, got: {err:?}"
        );
    }

    #[test]
    fn parse_section_header_unbalanced_quotes_returns_typed_error() {
        let line = r#"[remote "origin]"#;
        let err = parse_section_header(line).unwrap_err();
        assert!(
            matches!(err, SectionHeaderError::UnbalancedQuotes),
            "expected UnbalancedQuotes, got: {err:?}"
        );
    }

    #[test]
    fn parse_section_header_well_formed_round_trips() {
        let (section, sub) = parse_section_header(r#"[remote "origin"]"#).unwrap();
        assert_eq!(section, "remote");
        assert_eq!(sub.as_deref(), Some("origin"));
    }

    fn origin_section_present_but_no_url_returns_none() {
        let cfg = "[remote \"origin\"]\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n";
        assert!(read_origin_url_from(cfg).is_none());
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
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("https://github.com/real/repo.git".to_string())
        );
    }

    #[test]
    fn origin_section_header_is_case_insensitive() {
        // git-config(1) treats section names as case-insensitive; tools other
        // than git itself sometimes write `[Remote "origin"]` etc. The
        // matcher must accept those.
        let cfg = "\
[REMOTE \"origin\"]
\turl = https://github.com/upper/repo.git
";
        assert_eq!(
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("https://github.com/upper/repo.git".to_string())
        );

        let cfg_mixed = "\
[Remote \"origin\"]
\turl = https://github.com/mixed/repo.git
";
        assert_eq!(
            read_origin_url_from(cfg_mixed).map(RedactedUrl::into_string),
            Some("https://github.com/mixed/repo.git".to_string())
        );
    }

    #[test]
    fn unquoted_origin_subsection_is_not_treated_as_origin() {
        // `[remote origin]` (no quotes) is malformed per git-config(1) and git
        // itself ignores it; we must not silently honour what git would not.
        let cfg = "[remote origin]\n\turl = https://github.com/bare/repo.git\n";
        assert!(read_origin_url_from(cfg)
            .map(RedactedUrl::into_string)
            .is_none());
    }

    #[test]
    fn escaped_subsection_is_not_treated_as_origin() {
        // `[remote "or\"igin"]` decodes to subsection `or"igin`, not `origin`.
        let cfg = "[remote \"or\\\"igin\"]\n\turl = https://github.com/escaped/repo.git\n";
        assert!(read_origin_url_from(cfg)
            .map(RedactedUrl::into_string)
            .is_none());
    }

    #[test]
    fn whitespace_inside_origin_quotes_is_not_origin() {
        // Subsection names are case-sensitive and exact; `" origin "` is not
        // the same subsection as `"origin"`.
        let cfg = "[remote \" origin \"]\n\turl = https://github.com/spaced/repo.git\n";
        assert!(read_origin_url_from(cfg)
            .map(RedactedUrl::into_string)
            .is_none());
    }

    #[test]
    fn no_origin_section_returns_none() {
        let cfg = "\
[remote \"upstream\"]
\turl = https://example.com/other/repo.git
";
        assert!(read_origin_url_from(cfg)
            .map(RedactedUrl::into_string)
            .is_none());
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
            read_origin_url(&git_dir).map(RedactedUrl::into_string),
            Some("https://github.com/o/r.git".to_string())
        );
    }

    /// SEC-33 / TASK-0910: a `.git/config` larger than MAX_GIT_CONFIG_BYTES
    /// must NOT be parsed; the helper bails with a tracing::warn! and
    /// returns None instead of slurping the whole file into memory.
    #[test]
    fn read_origin_url_bails_on_oversized_config() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        // Build a payload ≥ MAX_GIT_CONFIG_BYTES + 1. The extra trailing
        // bytes are arbitrary `; comment` padding; the cap check fires
        // before the parser ever sees them.
        let header = "[remote \"origin\"]\n\turl = https://github.com/o/r.git\n";
        let pad_size = (MAX_GIT_CONFIG_BYTES as usize)
            .saturating_sub(header.len())
            .saturating_add(64);
        let mut body = String::with_capacity(header.len() + pad_size);
        body.push_str(header);
        // Use a comment line so well-formed parsing would still match
        // the URL above, *if* the cap weren't enforced.
        body.push_str(&"# pad\n".repeat(pad_size / 6));
        std::fs::write(git_dir.join("config"), body.as_bytes()).unwrap();
        assert!(
            read_origin_url(&git_dir).is_none(),
            "oversized .git/config must not yield an origin URL"
        );
    }

    #[test]
    fn embedded_credentials_are_redacted() {
        let cfg = "[remote \"origin\"]\n\turl = https://user:token@github.com/o/r.git\n";
        let url = read_origin_url_from(cfg)
            .map(RedactedUrl::into_string)
            .expect("origin url");
        assert!(!url.contains("user:token"), "leaked credentials: {url}");
        assert!(!url.contains('@'), "retained userinfo: {url}");
        assert_eq!(url, "https://github.com/o/r.git");
    }

    #[test]
    fn url_key_is_case_insensitive() {
        let cfg = "[remote \"origin\"]\n\tURL = https://github.com/o/r.git\n";
        assert_eq!(
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("https://github.com/o/r.git".to_string())
        );
    }

    /// ERR-4 (TASK-0594): git-config returns the *last* value when a key is
    /// set multiple times. A config that rewrites `url` after an initial
    /// value (templated includes do this) must report the rewritten URL,
    /// matching `git config --get remote.origin.url`.
    #[test]
    fn origin_url_returns_last_value_when_set_twice() {
        let cfg = "\
[remote \"origin\"]
\turl = https://github.com/old/repo.git
\turl = https://github.com/new/repo.git
";
        assert_eq!(
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("https://github.com/new/repo.git".to_string())
        );
    }

    /// Last-wins must hold even across an intervening section: a later
    /// `[remote "origin"]` block that re-assigns `url` overrides the earlier
    /// one, mirroring git-config(1)'s flat key-resolution model.
    #[test]
    fn origin_url_returns_last_value_across_sections() {
        let cfg = "\
[remote \"origin\"]
\turl = https://github.com/first/repo.git
[core]
\trepositoryformatversion = 0
[remote \"origin\"]
\turl = https://github.com/second/repo.git
";
        assert_eq!(
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("https://github.com/second/repo.git".to_string())
        );
    }

    /// READ-2 (TASK-0726): git-config also supports trailing inline
    /// comments. The scanner must strip them so the returned value matches
    /// `git config --get remote.origin.url`.
    #[test]
    fn inline_trailing_comment_is_stripped() {
        let cfg = "[remote \"origin\"]\n\turl = https://x.example/r.git ; comment\n";
        assert_eq!(
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("https://x.example/r.git".to_string())
        );

        let hash_cfg = "[remote \"origin\"]\n\turl = https://x.example/r.git # other comment\n";
        assert_eq!(
            read_origin_url_from(hash_cfg).map(RedactedUrl::into_string),
            Some("https://x.example/r.git".to_string())
        );
    }

    #[test]
    fn comment_lines_are_skipped() {
        let cfg = "[remote \"origin\"]\n# url = https://commented.example/x.git\n\turl = https://real.example/y.git\n";
        assert_eq!(
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
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

    /// Non-NotFound IO errors (e.g. unreadable config) must return None but
    /// emit a tracing::warn so operators can diagnose ACL / permission drift.
    #[cfg(unix)]
    #[test]
    fn read_origin_url_unreadable_config_returns_none() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        let config = git_dir.join("config");
        std::fs::write(
            &config,
            "[remote \"origin\"]\n\turl = https://github.com/o/r.git\n",
        )
        .unwrap();
        let mut perms = std::fs::metadata(&config).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&config, perms).unwrap();

        let result = read_origin_url(&git_dir).map(RedactedUrl::into_string);
        assert!(result.is_none(), "unreadable config should return None");

        // Restore so tempdir cleanup works.
        let mut restore = std::fs::metadata(&config).unwrap().permissions();
        restore.set_mode(0o644);
        std::fs::set_permissions(&config, restore).unwrap();
    }

    /// ERR-1 / TASK-0887: an unreadable HEAD must return `None` (matching
    /// detached-HEAD behaviour) rather than panicking. The warn-log emission
    /// itself is verified by the `tracing::warn!` shape — covering it
    /// requires a subscriber and is out of scope for this regression test;
    /// pinning the `None` result is enough to catch a future ".ok()?" regression.
    #[cfg(unix)]
    #[test]
    fn read_head_branch_returns_none_on_unreadable_head() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        let head = git_dir.join("HEAD");
        std::fs::write(&head, "ref: refs/heads/main\n").unwrap();
        let mut perms = std::fs::metadata(&head).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&head, perms).unwrap();

        let result = read_head_branch(&git_dir);
        assert!(result.is_none(), "unreadable HEAD should return None");

        // Restore so tempdir cleanup works.
        let mut restore = std::fs::metadata(&head).unwrap().permissions();
        restore.set_mode(0o644);
        std::fs::set_permissions(&head, restore).unwrap();
    }
}
