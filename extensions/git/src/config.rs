//! Read local `.git` directory metadata without shelling out to `git`.

use std::path::Path;

// Crate-private alias: `provider.rs` shares the import without the foreign
// ops-hook-common item gaining a second public path.
pub(crate) use ops_hook_common::find_git_dir;

/// A URL that has been scrubbed of `user[:password]@` userinfo.
///
/// The only way to construct one is [`RedactedUrl::redact`], which runs
/// `redact_userinfo`. Carrying a `RedactedUrl` through the call chain means
/// a raw URL cannot reach [`crate::provider::GitInfo::remote_url`], about
/// cards or JSON output without a visible `RedactedUrl::redact` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedUrl(String);

impl RedactedUrl {
    /// Construct from a raw URL by stripping `user[:password]@` userinfo.
    ///
    /// `redact_userinfo` is idempotent, so calling this on an already-clean
    /// value is a no-op.
    ///
    /// Returns `None` when `raw` contains any character that is unsafe to
    /// display: a control character (ANSI escape, raw newline, NUL) or a
    /// Unicode formatting / separator character (bidi overrides and isolates,
    /// zero-width characters, BOM, line and paragraph separators). Such a
    /// value could repaint a terminal or spoof the remote host or owner in
    /// about cards, JSON and logs, so it is treated as "no remote" instead.
    /// The policy is whole-codepoint — any char whose Unicode general
    /// category is Cc / Cf / Cs / Zl / Zp — and is the shared predicate
    /// `ops_core::text::is_unsafe_display_char`, so every About provider
    /// rejects the same set.
    ///
    /// ```
    /// use ops_git::config::RedactedUrl;
    /// let r = RedactedUrl::redact("https://alice:secret@github.com/o/r.git").unwrap();
    /// assert_eq!(r.as_str(), "https://github.com/o/r.git");
    /// // Idempotent: re-redacting an already-clean value is a no-op.
    /// let r2 = RedactedUrl::redact(r.as_str()).unwrap();
    /// assert_eq!(r2.as_str(), r.as_str());
    /// // Control bytes (ANSI escape, raw newline) cause the value to be
    /// // dropped entirely.
    /// assert!(RedactedUrl::redact("https://host/repo\u{1b}[31m\nfake").is_none());
    /// // Unicode RTL override / zero-width / BOM are also rejected.
    /// assert!(RedactedUrl::redact("https://host/\u{202e}fake/repo").is_none());
    /// assert!(RedactedUrl::redact("https://host/\u{200b}repo").is_none());
    /// ```
    #[must_use]
    pub fn redact(raw: &str) -> Option<Self> {
        if ops_core::text::contains_unsafe_display_chars(raw) {
            return None;
        }
        Some(Self(redact_userinfo(raw)))
    }

    /// Returns the redacted URL; the value is userinfo-free and
    /// control-character-free by construction.
    ///
    /// Callers must not re-introduce a raw, unredacted URL alongside this
    /// value — the type exists to make that refactor visible.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the newtype, returning the redacted URL as an owned string.
    ///
    /// The value is userinfo-free and control-character-free by construction;
    /// callers must not re-introduce a raw, unredacted URL alongside it.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for RedactedUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Hard cap on the `.git/config` read size.
///
/// A real-world git config is well under 64 KiB; without a cap an
/// adversarial repo (cloned for inspection) could exhaust memory through a
/// multi-GB file or a symlink to `/dev/zero`. Mirrors the
/// `ops_about::manifest_io::MAX_MANIFEST_BYTES` posture for project
/// manifests.
pub const MAX_GIT_CONFIG_BYTES: u64 = 4 * 1024 * 1024;

/// Hard cap on the `.git/HEAD` read size.
///
/// A real `HEAD` is ~30 bytes (`ref: refs/heads/<name>\n`); 4 KiB is ample
/// for any refname git will accept and still bounds the allocation an
/// adversarial repository can force. Mirrors the
/// [`MAX_GIT_CONFIG_BYTES`] posture for `.git/config`.
pub const MAX_HEAD_BYTES: u64 = 4 * 1024;

/// Read at most `cap` bytes of the git metadata file at `path`.
///
/// `label` names the file in log events (`.git/config`, `.git/HEAD`).
/// Returns `None` when the file is absent (silently — both files are
/// legitimately missing in some repository states), when it cannot be opened
/// or read, or when it is larger than `cap`; every case but absence logs one
/// `tracing::warn!`. The path is Debug-formatted so a hostile checkout path
/// with newlines or ANSI escapes cannot forge log records.
fn read_capped(path: &Path, cap: u64, label: &str) -> Option<Vec<u8>> {
    use std::io::Read;
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            tracing::warn!(
                file = label,
                path = ?path.display(),
                error = %e,
                "failed to open git metadata file; treating it as absent"
            );
            return None;
        }
    };
    let mut bytes = Vec::new();
    // Read one byte past the cap so an over-cap file is distinguishable from
    // one exactly at it.
    let limit = cap.saturating_add(1);
    if let Err(e) = file.take(limit).read_to_end(&mut bytes) {
        tracing::warn!(
            file = label,
            path = ?path.display(),
            error = %e,
            "failed to read git metadata file (within byte cap); treating it as absent"
        );
        return None;
    }
    // The cap is enforced on the raw bytes, before any decoding: lossy UTF-8
    // decoding expands each invalid byte to U+FFFD (3 bytes), so a decoded
    // length can exceed the cap for a file that is within it. A length that
    // does not fit in a `u64` is necessarily far above the cap, so saturating
    // keeps the comparison exact for every value it can distinguish.
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > cap {
        tracing::warn!(
            file = label,
            path = ?path.display(),
            cap,
            "git metadata file exceeds byte cap; refusing to parse and treating it as absent"
        );
        return None;
    }
    Some(bytes)
}

/// Read the URL of the `origin` remote from `<git_dir>/config`.
///
/// `NotFound` is silent (no remotes configured is normal). Other IO errors
/// (`PermissionDenied`, `IsADirectory`, etc.) log at `tracing::warn!` before
/// returning `None`.
///
/// The read is capped at [`MAX_GIT_CONFIG_BYTES`]; an oversized config
/// returns `None` with a `tracing::warn!` rather than being read in full.
///
/// The file is decoded lossily, so a non-UTF-8 byte anywhere in it (a BOM, a
/// latin-1 value in an unrelated section) does not prevent remote detection.
#[must_use]
pub fn read_origin_url(git_dir: &Path) -> Option<RedactedUrl> {
    let path = git_dir.join("config");
    let bytes = read_capped(&path, MAX_GIT_CONFIG_BYTES, ".git/config")?;
    let content = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(err) => {
            // Lets an operator chasing "remote_url is None" tell a non-UTF-8
            // config apart from an IO error or a missing file.
            tracing::debug!(
                path = ?path.display(),
                "git-config: non-UTF-8 bytes detected; decoding lossily so remote detection survives"
            );
            String::from_utf8_lossy(err.as_bytes()).into_owned()
        }
    };
    parse_origin_url_inner(&content, Some(&path))
}

/// Parse a git-config body and return the `[remote "origin"]` url.
///
/// Limitations: this is a minimal line scanner, not a conformant git-config
/// parser. It does **not** honour `[url "<base>"] insteadOf = ...` rewrites,
/// continuation lines, or `include.path` directives. Comments
/// (`#` / `;`) starting a line are skipped; everything else falls through.
/// Section headers and the `url` key are matched case-insensitively, since
/// git-config keys are case-insensitive.
///
/// git-config keys are multi-valued and the *last* assignment wins
/// (templated includes routinely rewrite `url` after an initial value), so
/// the scanner collects every `url` line inside the `origin` section across
/// the file and returns the final valid one, matching
/// `git config --get remote.origin.url`.
///
/// Inline trailing comments (`url = … ; old`) are stripped from unquoted
/// values, matching `git config --get`. A quoted value (`url = "…"`) is
/// decoded with the `\\` / `\"` escapes and keeps any `#` / `;` inside it.
///
/// Trailing comments on a *section header* (`[remote "origin"] # primary`)
/// are stripped too — see [`strip_header_comment`]. The other looseness git
/// allows on a header line, a key sharing it
/// (`[remote "origin"] url = https://…`), is **not** supported: the trimmed
/// line does not end in `]`, so the header itself fails to parse and the
/// section is skipped. No tool writes that form in practice;
/// `is_origin_header` logs the rejection at debug so the absence is
/// discoverable under `RUST_LOG=ops_git=debug`.
///
/// # Userinfo redaction
///
/// Returns a [`RedactedUrl`] — the type system enforces that any
/// `user[:password]@` userinfo is stripped before the value reaches a
/// caller. Callers cannot route the inner string into about-cards / JSON
/// without an explicit `into_string()` / `as_str()` call, which keeps a
/// credential leak visible at the call site.
#[must_use]
pub fn read_origin_url_from(content: &str) -> Option<RedactedUrl> {
    parse_origin_url_inner(content, None)
}

fn parse_origin_url_inner(content: &str, path: Option<&Path>) -> Option<RedactedUrl> {
    let mut in_origin = false;
    let mut origin_seen = false;
    let mut last: Option<RedactedUrl> = None;
    let mut rejected_count: usize = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with('[') {
            // git treats `#` / `;` as starting a comment anywhere outside a
            // quoted value, so `[remote "origin"] # primary` is an ordinary
            // header git resolves `remote.origin.url` from.
            // `parse_section_header` requires the line to end in `]`, so the
            // comment is stripped first.
            in_origin = is_origin_header(strip_header_comment(trimmed));
            if in_origin {
                origin_seen = true;
            }
            continue;
        }
        if in_origin {
            if let Some(value) = strip_url_key(trimmed) {
                match RedactedUrl::redact(value.as_ref()) {
                    Some(r) => last = Some(r),
                    None => {
                        // A `url = ...` line with an embedded control or
                        // Unicode formatting codepoint (raw newline, ANSI
                        // escape, NUL, bidi override, zero-width space) is
                        // dropped rather than propagated.
                        // One increment per line of `content`; cannot saturate a `usize`.
                        rejected_count = rejected_count.saturating_add(1);
                    }
                }
            }
        }
    }
    // Under last-wins, a rejected trailing `url =` line is masked by an
    // earlier valid value. One warn per parse, with a count and (when known)
    // the originating path, lets an operator chasing a stale remote_url tell
    // a config that drops only the latest value from one that drops them all.
    if rejected_count > 0 {
        tracing::warn!(
            path = ?path,
            rejected = rejected_count,
            "dropped origin url= line(s) containing a control or Unicode formatting codepoint"
        );
    }
    // Distinguish "no [remote \"origin\"] section" (silent) from "section
    // present but every url= line was malformed / empty" (one breadcrumb),
    // so "branch shows but remote_url is None" points at the config.
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
        return rest.map_or_else(
            || format!("{scheme}://{host}"),
            |r| format!("{scheme}://{host}/{r}"),
        );
    }
    // scp-style: strip a `user[:password]@` prefix that appears before the
    // first `/`. Past the first `/` the `@` belongs to a path component, not
    // userinfo.
    let (head, rest) = match value.split_once('/') {
        Some((h, r)) => (h, Some(r)),
        None => (value, None),
    };
    if let Some((_userinfo, host)) = head.rsplit_once('@') {
        return rest.map_or_else(|| host.to_string(), |r| format!("{host}/{r}"));
    }
    value.to_string()
}

fn strip_url_key(line: &str) -> Option<std::borrow::Cow<'_, str>> {
    let (key, value) = line.split_once('=')?;
    if !key.trim().eq_ignore_ascii_case("url") {
        return None;
    }
    let value = value.trim_start();
    // A leading `"` puts the value in git-config's quoted form. Decoding is
    // delegated to [`decode_quoted_body`] so the `\\` / `\"` escape grammar
    // is single-source with [`parse_section_header`]. Any malformed quoted
    // value (unterminated, unbalanced, unknown escape) collapses to None, so
    // the caller's redaction step sees no candidate.
    if let Some(body) = value.strip_prefix('"') {
        let (decoded, _rest) = decode_quoted_body(body).ok()?;
        return Some(std::borrow::Cow::Owned(decoded));
    }
    // Unquoted form: drop trailing inline comments (`#`, `;`) so the
    // returned value matches `git config --get remote.origin.url`.
    let uncommented = value
        .split_once(['#', ';'])
        .map_or(value, |(before, _comment)| before);
    Some(std::borrow::Cow::Borrowed(uncommented.trim()))
}

/// Drop a trailing `#` / `;` comment from a section header line.
///
/// git-config(1) starts a comment at an unquoted `#` or `;` anywhere on the
/// line, so `[remote "origin"] # primary` and `[remote "origin"] ; mirror`
/// are valid headers. The scan tracks quoting and the `\\` / `\"` escapes
/// git honours inside a subsection name, so a subsection that legitimately
/// contains `;` or `#` (`[remote "a;b"]`) is left intact.
///
/// Only the comment is stripped: `[remote "origin"] url = …`, git's
/// header-line key form, remains unsupported and is listed in the
/// [`read_origin_url_from`] limitation list.
fn strip_header_comment(line: &str) -> &str {
    let mut in_quotes = false;
    let mut escaped = false;
    for (i, b) in line.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match b {
            b'\\' if in_quotes => escaped = true,
            b'"' => in_quotes = !in_quotes,
            b'#' | b';' if !in_quotes => {
                // `#` / `;` are ASCII, so `i` is always a char boundary and
                // `get` always succeeds; it is used over `line[..i]` to keep
                // the panicking index form (clippy::string_slice) out.
                return line.get(..i).unwrap_or(line).trim_end();
            }
            _ => {}
        }
    }
    line
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
            // A malformed header for a section that looks like `remote.*`
            // (an attacker-shaped subsection escape, an unbalanced quote)
            // drops the whole section. Log the failure category at debug so
            // a `RUST_LOG=ops_git=debug` rerun explains the missing remote.
            if line
                .trim_start_matches('[')
                .starts_with(|c: char| c.eq_ignore_ascii_case(&'r'))
            {
                // Debug-format the raw header line. Unlike a `url = …` value
                // it never passes through `RedactedUrl::redact`, so a section
                // header carrying ANSI escapes or an interior `\r` would
                // otherwise reach the log sink verbatim and repaint the
                // operator's terminal.
                tracing::debug!(
                    line = ?line,
                    reason = ?reason,
                    "git-config: rejected section header that looks like remote.*"
                );
            }
            false
        }
    }
}

/// Typed reason for a [`parse_section_header`] reject, so callers can log
/// the specific failure category instead of collapsing every malformed
/// header into a silent `None`.
#[derive(Debug)]
enum SectionHeaderError {
    NotASectionHeader,
    UnbalancedQuotes,
    UnknownEscape,
    UnterminatedEscape,
}

/// Typed reason for [`decode_quoted_body`] failures.
///
/// Maps onto [`SectionHeaderError`] at the section-header call site and is
/// collapsed to `None` at the `url = "..."` call site.
#[derive(Debug)]
enum QuotedBodyError {
    Unterminated,
    UnknownEscape,
    UnterminatedEscape,
}

/// Shared decoder for git-config quoted-string bodies.
///
/// Input is the substring *after* an opening `"` (the opening quote is
/// already stripped by the caller). The decoder consumes characters,
/// applying the two escapes git-config(1) honours (`\\` → `\`, `\"` → `"`),
/// until it hits an unescaped closing `"`. On success it returns the
/// decoded body and the leftover `&str` after the closing quote so the
/// caller can decide whether to tolerate trailing content (`strip_url_key`)
/// or require an empty tail (`parse_section_header`).
///
/// Errors are typed so the section-header path can report
/// `SectionHeaderError::{UnknownEscape, UnterminatedEscape, UnbalancedQuotes}`;
/// the url= path collapses every error to `None`. Both callers share this
/// one decoder so the escape grammar has a single definition.
fn decode_quoted_body(body: &str) -> Result<(String, &str), QuotedBodyError> {
    let mut decoded = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                // `Chars::as_str` yields the remainder after the closing
                // quote without re-slicing `body` by byte index.
                let rest = chars.as_str();
                return Ok((decoded, rest));
            }
            '\\' => match chars.next() {
                Some('\\') => decoded.push('\\'),
                Some('"') => decoded.push('"'),
                Some(_) => return Err(QuotedBodyError::UnknownEscape),
                None => return Err(QuotedBodyError::UnterminatedEscape),
            },
            other => decoded.push(other),
        }
    }
    Err(QuotedBodyError::Unterminated)
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
        .ok_or(SectionHeaderError::UnbalancedQuotes)?;
    // The closing `"` must terminate the body: trailing content after it
    // (e.g. `[remote "origin"trailing]`) is a malformed header, and so is a
    // body that never closes.
    let (decoded, rest) = decode_quoted_body(body).map_err(|e| match e {
        QuotedBodyError::Unterminated => SectionHeaderError::UnbalancedQuotes,
        QuotedBodyError::UnknownEscape => SectionHeaderError::UnknownEscape,
        QuotedBodyError::UnterminatedEscape => SectionHeaderError::UnterminatedEscape,
    })?;
    if !rest.is_empty() {
        return Err(SectionHeaderError::UnbalancedQuotes);
    }
    Ok((section, Some(decoded)))
}

/// Read the current branch from `<git_dir>/HEAD`. Returns `None` on detached HEAD.
///
/// A missing `HEAD` is silent (legitimately absent for some repository
/// states); every other IO error logs a `tracing::warn!`, so an operator
/// chasing "branch keeps showing as detached" sees the underlying
/// permission or IO problem.
///
/// The read is capped at [`MAX_HEAD_BYTES`], enforced on the raw bytes
/// before decoding, so a multi-gigabyte `HEAD` or one symlinked to
/// `/dev/zero` cannot force an unbounded allocation.
///
/// `.git/HEAD` is writable by any tarball, mounted volume, submodule or
/// third-party checkout, and the branch is rendered on About cards and
/// emitted in provider JSON exactly like the remote URL. The ref is
/// therefore held to the same whole-codepoint policy
/// [`RedactedUrl::redact`] applies
/// (`ops_core::text::is_unsafe_display_char`), and a ref with a dot-only
/// path segment (`refs/heads/../../../etc`) is rejected as
/// `remote::is_valid_path_segment` does for owner and repo. A rejected ref
/// returns `None` (never a partially-sanitised branch) and emits one
/// `tracing::warn!` naming the reason.
#[must_use]
pub fn read_head_branch(git_dir: &Path) -> Option<String> {
    let head_path = git_dir.join("HEAD");
    let bytes = read_capped(&head_path, MAX_HEAD_BYTES, ".git/HEAD")?;
    let Ok(content) = String::from_utf8(bytes) else {
        // A refname is ASCII in practice; non-UTF-8 here is corruption or
        // injection, and there is no lossy form worth surfacing as a branch.
        tracing::warn!(
            path = ?head_path.display(),
            ".git/HEAD is not valid UTF-8; reporting branch as None"
        );
        return None;
    };
    let trimmed = content.trim();
    let rest = trimmed.strip_prefix("ref:")?.trim();
    let branch = rest.strip_prefix("refs/heads/")?;
    if branch.is_empty() {
        return None;
    }
    if ops_core::text::contains_unsafe_display_chars(branch) {
        tracing::warn!(
            path = ?head_path.display(),
            ".git/HEAD ref contains a control or Unicode formatting codepoint; reporting branch as None"
        );
        return None;
    }
    // A ref that resolves to a traversal shape (`refs/heads/../../../etc`)
    // must not reach operator-facing surfaces.
    if branch
        .split('/')
        .any(|seg| !seg.is_empty() && seg.bytes().all(|b| b == b'.'))
    {
        tracing::warn!(
            path = ?head_path.display(),
            ".git/HEAD ref contains a dot-only path segment; reporting branch as None"
        );
        return None;
    }
    Some(branch.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `MAX_GIT_CONFIG_BYTES` as a `usize`, for sizing test payloads.
    ///
    /// The cap is 4 MiB, which fits every `usize` these tests run on. On a
    /// hypothetical platform whose `usize` were narrower, `usize::MAX` would
    /// itself be below the cap, so the saturating fallback still yields an
    /// allocatable size rather than an unwrap or a panic.
    fn cap_bytes_as_usize() -> usize {
        usize::try_from(MAX_GIT_CONFIG_BYTES).unwrap_or(usize::MAX)
    }

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
        // `redact_userinfo` strips the `user@` prefix from scp-style URLs
        // too. The conventional `git@` is treated as userinfo for redaction
        // purposes; downstream `parse_remote_url` accepts the trimmed scp
        // form.
        assert_eq!(
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("github.com:openbao/openbao.git".to_string())
        );
    }

    /// Scp-style remotes that fall through unparseable must not surface
    /// embedded credentials: `read_origin_url_from` redacts the
    /// `user[:tok]@` prefix on non-`://` values too.
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

    /// A malformed escape in a `[remote "…"]` header
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

    /// A `[remote "origin"]` section that exists but has no valid
    /// `url = ...` line returns None and emits one `tracing::debug` breadcrumb.
    /// A genuinely-missing origin section stays silent. The breadcrumb itself
    /// is verified via `tracing-test`-free assertion: we only pin the return
    /// value here and rely on the inline `tracing::debug!` survival in the
    /// source — call-site presence is guarded by code review.
    #[test]
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

    /// When last-wins picks up a trailing `url = ...` line that gets dropped
    /// for embedded ASCII control bytes (e.g. an ANSI escape), the earlier
    /// valid URL must still be returned AND the drop must surface as a
    /// warn-level event with a rejected-line count so the operator can tell
    /// "stale URL" from "all URLs malformed".
    #[test]
    fn read_origin_url_warns_on_control_byte_drop_keeping_prior_valid() {
        // Two `url = ...` lines: a valid one, then a trailing line with an
        // embedded ANSI escape. The valid URL is returned (there is no later
        // valid value) and a warn fires with the rejected count.
        let cfg = "\
[remote \"origin\"]
\turl = https://github.com/real/repo.git
\turl = https://example.com/\u{001b}[31mrogue\u{001b}[0m
";
        // The shared harness pins a global dispatcher; capturing without
        // that pin is flaky under parallel test load.
        let (logged, url) = ops_core::test_utils::capture_tracing(tracing::Level::WARN, || {
            read_origin_url_from(cfg).map(RedactedUrl::into_string)
        });
        assert_eq!(
            url,
            Some("https://github.com/real/repo.git".to_string()),
            "must fall back to the earlier valid url= line"
        );

        assert!(
            logged.contains("WARN") && logged.contains("dropped origin url= line"),
            "expected one warn-level rejected-url event; got: {logged}"
        );
        assert!(
            logged.contains("rejected=1"),
            "warn must include rejected-line count; got: {logged}"
        );
    }

    /// A quoted `url = "..."` value containing an
    /// embedded `;` (legal per git-config) must round-trip without being
    /// truncated by the inline-comment stripper that applies to unquoted
    /// values.
    #[test]
    fn origin_url_quoted_value_with_semicolon_round_trips() {
        let cfg = "\
[remote \"origin\"]
\turl = \"https://example.com/path;tag=v1\"
";
        assert_eq!(
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("https://example.com/path;tag=v1".to_string())
        );
    }

    /// Quoted form decodes the same `\\\\` / `\\"` escapes
    /// that `parse_section_header` honours. Unbalanced quotes return None
    /// rather than silently shipping a leading-quote string.
    #[test]
    fn origin_url_quoted_value_with_escapes() {
        let cfg = "\
[remote \"origin\"]
\turl = \"https://example.com/q\\\"path\"
";
        assert_eq!(
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("https://example.com/q\"path".to_string())
        );

        let cfg_unbalanced = "\
[remote \"origin\"]
\turl = \"https://example.com/path
";
        assert!(read_origin_url_from(cfg_unbalanced).is_none());
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

    /// A single non-UTF-8 byte anywhere in `.git/config` (BOM, latin-1
    /// commit-template, hostile injection in an unrelated section) must not
    /// zero out remote detection: the file is decoded lossily, so a
    /// well-formed `[remote "origin"]` url= line survives.
    #[test]
    fn read_origin_url_survives_non_utf8_byte_in_unrelated_section() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        // Construct a config whose `[user]` section contains a non-UTF-8
        // byte in the email field (a hostile / latin-1-encoded value), but
        // whose `[remote "origin"]` block is clean ASCII.
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(b"[user]\n\temail = bad-\xff-byte@example.com\n");
        bytes.extend_from_slice(b"[remote \"origin\"]\n\turl = https://github.com/o/r.git\n");
        std::fs::write(git_dir.join("config"), &bytes).unwrap();

        assert_eq!(
            read_origin_url(&git_dir).map(RedactedUrl::into_string),
            Some("https://github.com/o/r.git".to_string()),
            "the well-formed url= line must survive a non-UTF-8 byte elsewhere"
        );
    }

    /// The size cap is checked on raw bytes *before* lossy UTF-8 decoding. A `.git/config` whose raw size is at
    /// or under `MAX_GIT_CONFIG_BYTES` but contains an invalid UTF-8 byte
    /// (each replaced by U+FFFD = 3 bytes on the lossy path) must still
    /// surface the `[remote "origin"]` URL — checking `content.len()` after
    /// lossy decode would spuriously inflate the size and false-reject.
    #[test]
    fn read_origin_url_survives_in_cap_non_utf8_near_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        // Build a payload that is *within* the cap on the raw-byte axis but
        // would exceed it once every invalid byte expands to U+FFFD (3
        // bytes). Each `\xff` byte becomes 3 bytes after lossy decoding, so
        // a 1 KiB block of `\xff` becomes 3 KiB. Stay safely under the cap
        // raw while pushing the lossy-decoded length past it.
        let header = b"[remote \"origin\"]\n\turl = https://github.com/o/r.git\n[user]\n\temail = ";
        let trailer = b"@example.com\n";
        let raw_target = cap_bytes_as_usize() - header.len() - trailer.len() - 16;
        // Half of the trailing block is invalid bytes — lossy expansion 3x
        // takes total decoded length well above MAX_GIT_CONFIG_BYTES even
        // though the raw file is comfortably under the cap.
        let invalid_block = vec![0xffu8; raw_target / 2];
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(header);
        bytes.extend_from_slice(&invalid_block);
        bytes.extend_from_slice(trailer);
        assert!(
            u64::try_from(bytes.len()).unwrap_or(u64::MAX) <= MAX_GIT_CONFIG_BYTES,
            "test payload must be within the byte cap"
        );
        std::fs::write(git_dir.join("config"), &bytes).unwrap();

        assert_eq!(
            read_origin_url(&git_dir).map(RedactedUrl::into_string),
            Some("https://github.com/o/r.git".to_string()),
            "in-cap config with non-UTF-8 bytes must still surface the origin URL"
        );
    }

    /// A `.git/config` larger than `MAX_GIT_CONFIG_BYTES`
    /// must NOT be parsed; the helper bails with a `tracing::warn`! and
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
        let pad_size = cap_bytes_as_usize()
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

    /// A `.git/config` `url = ...` value containing
    /// ASCII control bytes (raw newline, ANSI escape, NUL) must be dropped
    /// rather than propagated through `RedactedUrl` into JSON / about cards
    /// / logs. The directly-affected helpers are covered here; the
    /// provider-level end-to-end is pinned in `provider::tests`.
    #[test]
    fn redact_rejects_control_bytes() {
        assert!(RedactedUrl::redact("https://host/repo\u{1b}[31m\nfake").is_none());
        assert!(RedactedUrl::redact("https://host/repo\nfake").is_none());
        assert!(RedactedUrl::redact("https://host/repo\u{0}fake").is_none());
        assert!(RedactedUrl::redact("https://host/repo\u{7f}fake").is_none());
        // A clean URL still round-trips.
        assert_eq!(
            RedactedUrl::redact("https://host/repo.git")
                .map(RedactedUrl::into_string)
                .as_deref(),
            Some("https://host/repo.git")
        );
    }

    /// Bidi / zero-width / line-separator codepoints
    /// must also be rejected before reaching About cards / JSON / logs
    /// through `RedactedUrl`. An ASCII-only gate would be bypassed
    /// by the multibyte sequences for U+202E (RTL OVERRIDE), U+200B / U+200D
    /// (zero-width joiners), U+FEFF (BOM), the bidi isolates U+2066..U+2069,
    /// and U+2028 / U+2029 (line / paragraph separators).
    #[test]
    fn redact_rejects_unicode_format_and_separator_codepoints() {
        for raw in [
            // Bidi formatting (homograph / spoofing surface).
            "https://host/\u{202e}fake/repo",
            "https://host\u{202d}/repo",
            "https://github.com/\u{2066}attacker\u{2069}/repo",
            // Zero-width family.
            "https://host/\u{200b}repo",
            "https://host/\u{200c}repo",
            "https://host/\u{200d}repo",
            "https://host/\u{2060}repo",
            // BOM / specials.
            "https://host/\u{feff}repo",
            // Unicode line / paragraph separators.
            "https://host/\u{2028}fake",
            "https://host/\u{2029}fake",
        ] {
            assert!(
                RedactedUrl::redact(raw).is_none(),
                "expected rejection for {raw:?}"
            );
        }
        // A URL containing only unrelated multibyte text (e.g. a Punycode-
        // encoded host or a UTF-8 path segment) still round-trips.
        assert_eq!(
            RedactedUrl::redact("https://例子.test/repo")
                .map(RedactedUrl::into_string)
                .as_deref(),
            Some("https://例子.test/repo")
        );
    }

    #[test]
    fn origin_url_with_control_bytes_is_dropped() {
        let cfg = "[remote \"origin\"]\n\turl = https://host/repo\u{1b}[31m fake\n";
        // The trailing literal `\n` ends the line, but the ANSI escape and
        // any other control bytes inside the value cause the line to be
        // dropped entirely.
        assert!(read_origin_url_from(cfg).is_none());
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

    /// Git-config returns the *last* value when a key is
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

    /// Git-config also supports trailing inline
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
    /// emit a `tracing::warn` so operators can diagnose ACL / permission drift.
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

    /// An unreadable HEAD must return `None` (matching
    /// detached-HEAD behaviour) rather than panicking. The warn-log emission
    /// itself is verified by the `tracing::warn!` shape — covering it
    /// requires a subscriber and is out of scope for this regression test;
    /// pinning the `None` result is enough to catch a future ".`ok()`?" regression.
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

    /// `read_origin_url` logs the .git/config path through
    /// the `?` (Debug) formatter so a hostile checkout path containing
    /// newlines or ANSI escapes cannot forge log entries or repaint the
    /// operator terminal. Pin the value-level escape contract directly,
    /// mirroring the workspace-sidecar / `manifest_io` policy.
    #[test]
    fn read_origin_url_path_debug_escapes_control_characters() {
        let p = std::path::Path::new("/tmp/dir\n\u{1b}[31m/.git/config");
        let rendered = format!("{:?}", p.display());
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

    /// `is_origin_header` logs the raw
    /// `.git/config` section-header line, which — unlike a `url = …` value —
    /// never passes through `RedactedUrl::redact`. Debug-formatting it is
    /// what keeps ANSI escapes and an interior `\r` from reaching the log
    /// sink verbatim. Same value-level contract as
    /// `read_origin_url_path_debug_escapes_control_characters`.
    #[test]
    fn rejected_section_header_line_debug_escapes_control_characters() {
        let line = "[remote \u{1b}[31m\rorigin\"]";
        let rendered = format!("{line:?}");
        assert!(!rendered.contains('\u{1b}'));
        assert!(!rendered.contains('\r'));
        assert!(rendered.contains("\\r"));
        assert!(rendered.contains("\\u{1b}"));
    }

    /// The HEAD path takes the same Debug formatter as the `.git/config`
    /// path.
    #[test]
    fn read_head_branch_path_debug_escapes_control_characters() {
        let p = std::path::Path::new("/tmp/dir\n\u{1b}[31m/.git/HEAD");
        let rendered = format!("{:?}", p.display());
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

    /// No `tracing` call in this crate may Display-format a
    /// path or a raw `.git/config` line. Grep the sources so a future call
    /// site cannot quietly reintroduce the forging surface.
    #[test]
    fn no_tracing_call_display_formats_a_path_or_config_line() {
        for name in ["config.rs", "provider.rs", "remote.rs", "lib.rs"] {
            let src = std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("src")
                    .join(name),
            )
            .expect("read source");
            for (n, line) in src.lines().enumerate() {
                let trimmed = line.trim();
                assert!(
                    !(trimmed.starts_with("path = %")
                        || trimmed.starts_with("line = %")
                        || trimmed == "line,"),
                    "{name}:{}: Display-formatted path / raw config line in a tracing call",
                    n + 1
                );
            }
        }
    }

    fn write_head(contents: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(git_dir.join("HEAD"), contents).unwrap();
        (dir, git_dir)
    }

    /// An ANSI escape in the ref would repaint the operator's terminal
    /// wherever `git_info.branch` is rendered, so the ref is rejected.
    #[test]
    fn head_branch_with_ansi_escape_is_rejected() {
        let (_d, git_dir) = write_head("ref: refs/heads/main\u{1b}[2J\u{1b}[31mFAKE\n");
        assert_eq!(read_head_branch(&git_dir), None);
    }

    /// U+202E RIGHT-TO-LEFT OVERRIDE is a homograph surface, rejected here
    /// as it is for the remote URL.
    #[test]
    fn head_branch_with_bidi_override_is_rejected() {
        let (_d, git_dir) = write_head("ref: refs/heads/ma\u{202e}in\n");
        assert_eq!(read_head_branch(&git_dir), None);
    }

    /// `trim` only removes leading / trailing whitespace,
    /// so an interior CR survives into the branch string.
    #[test]
    fn head_branch_with_interior_carriage_return_is_rejected() {
        let (_d, git_dir) = write_head("ref: refs/heads/main\rfake\n");
        assert_eq!(read_head_branch(&git_dir), None);
    }

    /// A traversal-shaped ref must not reach
    /// `git_info.branch` — the shape `remote::is_valid_path_segment`
    /// rejects on the remote side.
    #[test]
    fn head_branch_with_dot_only_segment_is_rejected() {
        let (_d, git_dir) = write_head("ref: refs/heads/../../../etc\n");
        assert_eq!(read_head_branch(&git_dir), None);
        let (_d2, git_dir2) = write_head("ref: refs/heads/feature/./foo\n");
        assert_eq!(read_head_branch(&git_dir2), None);
    }

    /// The hardening must not cost ordinary branches —
    /// including the `.`-containing and slash-containing names git allows.
    #[test]
    fn head_branch_normal_names_still_round_trip() {
        let (_d, git_dir) = write_head("ref: refs/heads/feature/foo\n");
        assert_eq!(read_head_branch(&git_dir), Some("feature/foo".to_string()));
        let (_d2, git_dir2) = write_head("ref: refs/heads/release-1.2.3\n");
        assert_eq!(
            read_head_branch(&git_dir2),
            Some("release-1.2.3".to_string())
        );
    }

    /// A HEAD one byte over the cap must return `None` rather than
    /// allocating the file.
    #[test]
    fn head_branch_over_byte_cap_is_rejected() {
        let cap = usize::try_from(MAX_HEAD_BYTES).unwrap_or(usize::MAX);
        let prefix = "ref: refs/heads/";
        let oversized = format!("{prefix}{}", "a".repeat(cap + 1 - prefix.len()));
        assert_eq!(oversized.len(), cap + 1);
        let (_d, git_dir) = write_head(&oversized);
        assert_eq!(read_head_branch(&git_dir), None);
    }

    /// Exactly at the cap still parses, so the bound is
    /// a cap and not an off-by-one rejection of large-but-legal refs.
    #[test]
    fn head_branch_exactly_at_byte_cap_is_accepted() {
        let cap = usize::try_from(MAX_HEAD_BYTES).unwrap_or(usize::MAX);
        let prefix = "ref: refs/heads/";
        let name = "a".repeat(cap - prefix.len());
        let at_cap = format!("{prefix}{name}");
        assert_eq!(at_cap.len(), cap);
        let (_d, git_dir) = write_head(&at_cap);
        assert_eq!(read_head_branch(&git_dir), Some(name));
    }

    /// Git starts a comment at an unquoted `#` / `;`
    /// anywhere on the line, so these are ordinary headers and the section's
    /// `url =` lines must be read.
    #[test]
    fn section_header_with_trailing_hash_comment_is_recognised() {
        let cfg = "[remote \"origin\"] # primary\n\turl = https://github.com/o/r.git\n";
        assert_eq!(
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("https://github.com/o/r.git".to_string())
        );
    }

    #[test]
    fn section_header_with_trailing_semicolon_comment_is_recognised() {
        let cfg = "[remote \"origin\"] ; upstream mirror\n\turl = https://github.com/o/r.git\n";
        assert_eq!(
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("https://github.com/o/r.git".to_string())
        );
    }

    /// A `;` or `#` *inside* the quoted subsection
    /// name is part of the name, not a comment — stripping must not cut it.
    #[test]
    fn quoted_subsection_containing_comment_chars_survives() {
        let cfg = "[remote \"a;b\"]\n\turl = https://github.com/o/wrong.git\n\
                   [remote \"origin\"]\n\turl = https://github.com/o/r.git\n";
        assert_eq!(
            read_origin_url_from(cfg).map(RedactedUrl::into_string),
            Some("https://github.com/o/r.git".to_string())
        );
        // And the `;`-bearing subsection is matched as itself, not truncated
        // to `[remote "a`.
        assert_eq!(strip_header_comment("[remote \"a;b\"]"), "[remote \"a;b\"]");
        assert_eq!(strip_header_comment("[remote \"a#b\"]"), "[remote \"a#b\"]");
        assert!(!is_origin_header("[remote \"a;b\"]"));
    }

    /// An origin section whose *name* carries the
    /// comment marker inside quotes still resolves.
    #[test]
    fn quoted_origin_subsection_with_trailing_comment() {
        assert_eq!(
            strip_header_comment("[remote \"origin\"] ; note"),
            "[remote \"origin\"]"
        );
        assert!(is_origin_header(strip_header_comment(
            "[remote \"origin\"] ; note"
        )));
    }

    /// The header-line key form stays unsupported
    /// and documented — pin the behaviour so the limitation list stays true.
    #[test]
    fn header_line_key_form_remains_unsupported() {
        let cfg = "[remote \"origin\"] url = https://github.com/o/r.git\n";
        assert_eq!(read_origin_url_from(cfg), None);
    }
}
