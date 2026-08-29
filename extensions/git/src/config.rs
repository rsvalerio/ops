//! Read local `.git` directory metadata without shelling out to `git`.

use std::path::Path;

pub use ops_hook_common::find_git_dir;

/// ARCH-2 / SEC-13 / TASK-0894: type-system-enforced "this URL has been
/// scrubbed of `user[:password]@` userinfo".
///
/// The only ways to construct one are [`RedactedUrl::redact`] (runs
/// `redact_userinfo`) and the `From<&str>` impl that delegates to it.
/// Carrying a `RedactedUrl` through the call chain means a future
/// refactor cannot accidentally route a raw URL into
/// [`crate::GitInfo::remote_url`] / about cards / JSON output without a
/// visible `RedactedUrl::redact` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedUrl(String);

impl RedactedUrl {
    /// Construct from a raw URL by stripping `user[:password]@` userinfo.
    /// `redact_userinfo` is idempotent so calling this on an already-clean
    /// value is a no-op.
    ///
    /// SEC-2 / TASK-1102: returns `None` when `raw` contains any ASCII
    /// control byte (`\x00..=\x1f` or `\x7f`). A `.git/config` line with an
    /// embedded ANSI escape, raw newline, or NUL must not flow through to
    /// JSON / about cards / logs — the redacted form is treated as "no
    /// remote" instead. Mirrors the control-char hardening already applied
    /// to other log-bound fields (TASK-0937, TASK-0974).
    ///
    /// SEC-2 / TASK-1238: the policy is broadened to reject Unicode
    /// formatting / separator / control characters too. The bare ASCII
    /// filter let multibyte sequences for RIGHT-TO-LEFT OVERRIDE
    /// (U+202E), zero-width joiners (U+200B / U+200D), BOM (U+FEFF),
    /// other directional / formatting overrides (U+2066..U+2069), and
    /// Unicode line separators (U+2028 / U+2029) survive into operator-
    /// facing surfaces — bidi/homograph spoofing of remote host or owner
    /// in About cards / JSON / logs. Whole-codepoint policy:
    /// reject any char whose Unicode general category is Cc / Cf / Cs /
    /// Zl / Zp (matched directly via `char::is_control` and an explicit
    /// list of the most abused formatting codepoints), then redact
    /// userinfo on the cleaned value.
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
        if raw.bytes().any(is_ascii_control_byte) || raw.chars().any(is_unicode_format_or_separator)
        {
            return None;
        }
        Some(Self(redact_userinfo(raw)))
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

/// SEC-2 / TASK-1102: ASCII control bytes (`0x00..=0x1f` and `0x7f`) must
/// never reach JSON / about cards / logs through a [`RedactedUrl`]. Newlines,
/// NULs, and ANSI escapes from a hostile `.git/config` would otherwise
/// forge log lines or recolor terminal output downstream.
#[inline]
const fn is_ascii_control_byte(b: u8) -> bool {
    b < 0x20 || b == 0x7f
}

/// SEC-2 / TASK-1238: Unicode formatting / directional-override /
/// zero-width / line-separator codepoints must also be rejected before a
/// remote URL flows into operator-facing surfaces (About cards, JSON,
/// logs). The ASCII gate above only covers `<0x20` / `0x7f`, so multibyte
/// sequences for U+202E (RIGHT-TO-LEFT OVERRIDE), U+200B / U+200D /
/// U+200C (zero-width joiner family), U+FEFF (BOM), the bidi isolate
/// codepoints U+2066..U+2069, U+2028 / U+2029 (line / paragraph
/// separators), and the broader `char::is_control` set survive otherwise.
/// Used by [`RedactedUrl::redact`] alongside the ASCII filter to give a
/// single whole-codepoint policy, mirroring the SEC-2 hardening in
/// `extensions-node/about::repo_url::contains_control_chars` (TASK-1165)
/// and `extensions-python/about::contains_control_chars` (TASK-1207).
#[inline]
const fn is_unicode_format_or_separator(c: char) -> bool {
    if c.is_control() {
        return true;
    }
    matches!(
        c,
        // Zero-width family + ZWNJ / ZWJ + word joiner.
        '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}'
        // BOM / specials.
        | '\u{FEFF}'
        // Bidi formatting characters: LRM/RLM, LRE/RLE/PDF, LRO/RLO.
        | '\u{200E}' | '\u{200F}'
        | '\u{202A}' | '\u{202B}' | '\u{202C}' | '\u{202D}' | '\u{202E}'
        // Bidi isolates.
        | '\u{2066}' | '\u{2067}' | '\u{2068}' | '\u{2069}'
        // Unicode line / paragraph separators (Zl / Zp).
        | '\u{2028}' | '\u{2029}'
    )
}

impl std::fmt::Display for RedactedUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// SEC-33 / TASK-0910: hard cap on `.git/config` read size.
///
/// A real-world git config is well under 64 KiB; an adversarial repo
/// (cloned for inspection) could otherwise OOM the CLI through a
/// multi-GB file or a symlink to `/dev/zero`. Mirrors the
/// `ops_about::manifest_io::MAX_MANIFEST_BYTES` posture for project
/// manifests.
pub const MAX_GIT_CONFIG_BYTES: u64 = 4 * 1024 * 1024;

/// Read the URL of the `origin` remote from `<git_dir>/config`.
///
/// `NotFound` is silent (no remotes configured is normal). Other IO errors
/// (`PermissionDenied`, `IsADirectory`, etc.) log at `tracing::warn!` before
/// returning None, matching the policy of `try_read_manifest` (TASK-0548)
/// and `resolve_member_globs` (TASK-0517).
///
/// SEC-33 / TASK-0910: the read is capped at [`MAX_GIT_CONFIG_BYTES`]
/// via `File::open` + `Read::take`. An oversized config returns `None`
/// with a `tracing::warn!` rather than slurping the whole file.
///
/// READ-4 / TASK-1878: this function carried a `# Panics` section describing
/// a `String::from_utf8` invariant violation. ERR-1 / TASK-1244 replaced that
/// fallible decode with an explicit `Err` arm that falls back to
/// `String::from_utf8_lossy`, so the panic it documented became unreachable.
/// The section is removed rather than left to mislead callers deciding
/// whether a call needs isolating.
#[must_use]
pub fn read_origin_url(git_dir: &Path) -> Option<RedactedUrl> {
    use std::io::Read;
    let path = git_dir.join("config");
    let mut file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            // ERR-7 / TASK-1206: Debug-format the path so a hostile checkout
            // path with newlines / ANSI cannot forge log records. Mirrors
            // read_workspace_sidecar / manifest_io::read_optional_text policy.
            tracing::warn!(
                path = ?path.display(),
                error = %e,
                "failed to open .git/config; treating as no remote"
            );
            return None;
        }
    };
    // ERR-1 / TASK-1244: read raw bytes and lossy-decode so a single non-UTF-8
    // byte (BOM, latin-1 commit-template, hostile injection) anywhere in
    // .git/config does not poison remote detection. The previous
    // `read_to_string` required the whole file to be valid UTF-8 and
    // surfaced any failure as a generic IO warn, even when the
    // [remote "origin"] section was well-formed.
    let mut bytes = Vec::new();
    let limit = MAX_GIT_CONFIG_BYTES.saturating_add(1);
    if let Err(e) = (&mut file).take(limit).read_to_end(&mut bytes) {
        // ERR-7 / TASK-1206: Debug-format path; see comment above.
        tracing::warn!(
            path = ?path.display(),
            error = %e,
            "failed to read .git/config (within byte cap); treating as no remote"
        );
        return None;
    }
    // SEC-33 / TASK-1620: enforce the byte cap on the raw bytes, *before*
    // lossy UTF-8 decoding. `String::from_utf8_lossy` replaces each invalid
    // byte with U+FFFD (3 UTF-8 bytes), so checking `content.len()` after
    // decoding can spuriously exceed the cap for in-cap files containing
    // non-UTF-8 bytes — false-rejecting exactly the scenario
    // `read_origin_url_survives_non_utf8_byte_in_unrelated_section` exists
    // to support. `take(limit)` already returned ≤ limit bytes, so the file
    // is in-cap iff `bytes.len() <= MAX_GIT_CONFIG_BYTES`.
    // A length that does not fit in a `u64` is necessarily far above the
    // 4 MiB cap, so saturating to `u64::MAX` keeps this comparison exact for
    // every value the check can actually distinguish.
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_GIT_CONFIG_BYTES {
        // ERR-7 / TASK-1206: Debug-format path; see comment above.
        tracing::warn!(
            path = ?path.display(),
            cap = MAX_GIT_CONFIG_BYTES,
            "SEC-33: .git/config exceeds byte cap; refusing to parse and treating as no remote"
        );
        return None;
    }
    let content = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(err) => {
            // ERR-1 / TASK-1244: typed debug breadcrumb so operators chasing
            // "remote_url is None" can tell a non-UTF-8 config apart from a
            // generic IO error or a missing file.
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
/// READ-5 (TASK-1876): trailing comments on a *section header*
/// (`[remote "origin"] # primary`) are stripped too — see
/// [`strip_header_comment`]. The other looseness git allows on a header
/// line, a key sharing it (`[remote "origin"] url = https://…`), is
/// **not** supported: the trimmed line does not end in `]`, so the header
/// itself fails to parse and the section is skipped. Documented as a
/// limitation rather than implemented, since no tool writes that form in
/// practice; `is_origin_header` logs the rejection at debug so the absence
/// is discoverable under `RUST_LOG=ops_git=debug`.
///
/// # Userinfo redaction (SEC-13 / TASK-0894)
///
/// Returns a [`RedactedUrl`] — the type system enforces that any
/// `user[:password]@` userinfo is stripped before the value reaches a
/// caller. Callers cannot route the inner string into about-cards / JSON
/// without an explicit `into_string()` / `as_str()` call, which makes a
/// future credential-leak refactor visible at the call site instead of
/// silent.
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
            // READ-5 / TASK-1876: git treats `#` / `;` as starting a comment
            // anywhere outside a quoted value, so `[remote "origin"] # primary`
            // is an ordinary header git resolves `remote.origin.url` from.
            // `parse_section_header` requires the trimmed line to end in `]`,
            // so without this strip the header failed to parse, `in_origin`
            // went false, and *every* `url =` line in the section was
            // dropped — total loss of repository identity for a config shape
            // git accepts. The READ-2 / TASK-0726 comment stripping covers
            // value lines only; header lines never saw it.
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
                        // SEC-2 / TASK-1102: a `url = ...` line with embedded
                        // ASCII control bytes (raw newline, ANSI escape, NUL)
                        // is dropped rather than propagated.
                        // One increment per line of `content`; cannot saturate a `usize`.
                        rejected_count = rejected_count.saturating_add(1);
                    }
                }
            }
        }
    }
    // ERR-1 / TASK-1215: rejecting a control-byte url= line used to log only
    // at debug. Combined with the last-wins policy, a trailing control-byte
    // url= line was silently masked by an earlier valid value. Surface every
    // rejected origin url= line at warn so operators chasing "branch shows
    // but remote_url is stale" see one event per parse, with a count and
    // (when available) the originating path so a malformed config that drops
    // every value differs from one that drops only the latest.
    if rejected_count > 0 {
        if let Some(p) = path {
            tracing::warn!(
                path = ?p,
                rejected = rejected_count,
                "SEC-2 / TASK-1215: dropped origin url= line(s) containing ASCII control bytes"
            );
        } else {
            tracing::warn!(
                rejected = rejected_count,
                "SEC-2 / TASK-1215: dropped origin url= line(s) containing ASCII control bytes"
            );
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
    // READ-2 / TASK-1213, DUP-1 / TASK-1622: a leading `"` puts the value
    // in git-config's quoted form. Delegate decoding to the shared
    // [`decode_quoted_body`] helper so the `\\` / `\"` escape grammar is
    // single-source with [`parse_section_header`]. Any malformed quoted
    // value (unterminated, unbalanced, unknown escape) collapses to None
    // — caller's downstream redaction step sees no candidate.
    if let Some(body) = value.strip_prefix('"') {
        let (decoded, _rest) = decode_quoted_body(body).ok()?;
        return Some(std::borrow::Cow::Owned(decoded));
    }
    // READ-2 (TASK-0726): unquoted form — drop trailing inline comments
    // (`#`, `;`) so the returned value matches `git config --get
    // remote.origin.url`.
    let uncommented = value
        .split_once(['#', ';'])
        .map_or(value, |(before, _comment)| before);
    Some(std::borrow::Cow::Borrowed(uncommented.trim()))
}

/// READ-5 / TASK-1876: drop a trailing `#` / `;` comment from a section
/// header line.
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
                // SEC-21 / ERR-7 / TASK-1871: Debug-format the raw header
                // line. Unlike a `url = …` value it never passes through
                // `RedactedUrl::redact`, so a `.git/config` section header
                // carrying ANSI escapes or an interior `\r` would otherwise
                // reach the log sink verbatim and repaint the operator's
                // terminal — the same forging risk TASK-1206 closed for the
                // config path.
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

/// DUP-1 / TASK-1622: typed reason for [`decode_quoted_body`] failures.
/// Maps onto [`SectionHeaderError`] at the section-header call site and is
/// collapsed to `None` at the `url = "..."` call site — keeping the
/// git-config quoted-string escape grammar single-source between both.
#[derive(Debug)]
enum QuotedBodyError {
    Unterminated,
    UnknownEscape,
    UnterminatedEscape,
}

/// DUP-1 / TASK-1622: shared decoder for git-config quoted-string bodies.
///
/// Input is the substring *after* an opening `"` (the opening quote is
/// already stripped by the caller). The decoder consumes characters,
/// applying the two escapes git-config(1) honours (`\\` → `\`, `\"` → `"`),
/// until it hits an unescaped closing `"`. On success it returns the
/// decoded body and the leftover `&str` after the closing quote so the
/// caller can decide whether to tolerate trailing content (`strip_url_key`)
/// or require an empty tail (`parse_section_header`).
///
/// Errors are typed so the section-header path can preserve its existing
/// `SectionHeaderError::{UnknownEscape, UnterminatedEscape, UnbalancedQuotes}`
/// surface; the url= path collapses every error to `None`. Keeping both
/// behaviours wrapped around a single decoder means future tightening of
/// the escape grammar (or hardening against an attacker-shaped value) only
/// has to land in one place.
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
    // DUP-1 / TASK-1622: share the quoted-body decoder with `strip_url_key`.
    // The closing `"` must terminate the body — any trailing content after
    // it is a malformed header (e.g. `[remote "origin"trailing]`), and a
    // body that never closes is treated the same as the pre-refactor
    // `strip_suffix('"')` failure.
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
/// ERR-1 / TASK-0887: mirrors the policy already applied to
/// [`read_origin_url`] — silent on `NotFound` (legitimately absent for some
/// repository states), `tracing::warn!` on every other IO error so an
/// operator chasing "branch keeps showing as detached" sees the underlying
/// permission/EIO problem instead of a `None` that pretends HEAD is detached.
///
/// SEC-33 / TASK-1866 (superseding the falsely-closed TASK-0927, which was
/// marked Done with every acceptance criterion ticked while no code landed):
/// the read is capped at [`MAX_HEAD_BYTES`] with the same `File::open` +
/// `Read::take` shape [`read_origin_url`] uses, and the cap is enforced on
/// raw bytes *before* decoding, matching the TASK-1620 ordering fix. A
/// multi-gigabyte `HEAD`, or one symlinked to `/dev/zero`, previously forced
/// an unbounded allocation on every `ops about` invocation.
///
/// SEC-2 / SEC-11 / TASK-1863: the returned branch is subjected to the same
/// whole-codepoint policy [`RedactedUrl::redact`] applies to the remote URL
/// ([`is_ascii_control_byte`] + [`is_unicode_format_or_separator`]), plus the
/// dot-only-segment rejection `remote::is_valid_path_segment` applies to
/// owner/repo (SEC-13 / TASK-0929). `git_info.branch` is rendered on About
/// cards and emitted in provider JSON exactly like `remote_url`, but only
/// the URL reader was hardened: a `.git/HEAD` of
/// `ref: refs/heads/main\x1b[2J\x1b[31mFAKE` — writable by any tarball,
/// mounted volume, submodule, or third-party checkout — repainted the
/// operator's terminal, and a U+202E ref spoofed the branch name outright.
/// A rejected ref returns `None` (never a partially-sanitised branch) and
/// emits one `tracing::warn!` naming the reason, mirroring the
/// `read_origin_url` rejected-line breadcrumb (TASK-1215).
#[must_use]
pub fn read_head_branch(git_dir: &Path) -> Option<String> {
    use std::io::Read;
    let head_path = git_dir.join("HEAD");
    let mut file = match std::fs::File::open(&head_path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            // ERR-7 / TASK-1206, TASK-1871: Debug-format the path so a
            // hostile checkout path with newlines / ANSI cannot forge log
            // records. This arm logged with `%` (Display) while the three
            // `read_origin_url` arms in this file already used `?`.
            tracing::warn!(
                path = ?head_path.display(),
                error = %e,
                "failed to open .git/HEAD; reporting branch as None"
            );
            return None;
        }
    };
    let mut bytes = Vec::new();
    let limit = MAX_HEAD_BYTES.saturating_add(1);
    if let Err(e) = (&mut file).take(limit).read_to_end(&mut bytes) {
        tracing::warn!(
            path = ?head_path.display(),
            error = %e,
            "failed to read .git/HEAD (within byte cap); reporting branch as None"
        );
        return None;
    }
    // SEC-33 / TASK-1866: enforce the cap on raw bytes before any decoding.
    // `take(limit)` returned at most `limit` bytes, so the file is in-cap iff
    // `bytes.len() <= MAX_HEAD_BYTES`. A length that does not fit in a `u64`
    // is necessarily far above the cap, so saturating keeps the comparison
    // exact for every value it can distinguish.
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_HEAD_BYTES {
        tracing::warn!(
            path = ?head_path.display(),
            cap = MAX_HEAD_BYTES,
            "SEC-33: .git/HEAD exceeds byte cap; refusing to parse and reporting branch as None"
        );
        return None;
    }
    let Ok(content) = String::from_utf8(bytes) else {
        // A refname is ASCII in practice; non-UTF-8 here is corruption or
        // injection, and there is no lossy form worth surfacing as a branch.
        tracing::warn!(
            path = ?head_path.display(),
            "SEC-2: .git/HEAD is not valid UTF-8; reporting branch as None"
        );
        return None;
    };
    let trimmed = content.trim();
    let rest = trimmed.strip_prefix("ref:")?.trim();
    let branch = rest.strip_prefix("refs/heads/")?;
    if branch.is_empty() {
        return None;
    }
    // SEC-2 / TASK-1863: reuse the `RedactedUrl::redact` predicates rather
    // than growing a third copy of the policy.
    if branch.bytes().any(is_ascii_control_byte)
        || branch.chars().any(is_unicode_format_or_separator)
    {
        tracing::warn!(
            path = ?head_path.display(),
            "SEC-2 / TASK-1863: .git/HEAD ref contains a control or Unicode formatting codepoint; reporting branch as None"
        );
        return None;
    }
    // SEC-13 / TASK-1863: a ref that resolves to a traversal shape
    // (`refs/heads/../../../etc`) must not reach operator-facing surfaces —
    // the same rejection `remote::is_valid_path_segment` applies to
    // owner/repo (TASK-0929).
    if branch
        .split('/')
        .any(|seg| !seg.is_empty() && seg.bytes().all(|b| b == b'.'))
    {
        tracing::warn!(
            path = ?head_path.display(),
            "SEC-13 / TASK-1863: .git/HEAD ref contains a dot-only path segment; reporting branch as None"
        );
        return None;
    }
    Some(branch.to_string())
}

/// SEC-33 / TASK-1866 (supersedes the falsely-closed TASK-0927): hard cap on
/// the `.git/HEAD` read size.
///
/// A real `HEAD` is ~30 bytes (`ref: refs/heads/<name>\n`); 4 KiB is ample
/// for any refname git will accept and still bounds the allocation an
/// adversarial repository can force. Mirrors the
/// [`MAX_GIT_CONFIG_BYTES`] posture for `.git/config`.
pub const MAX_HEAD_BYTES: u64 = 4 * 1024;

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

    /// TASK-0966: a `[remote "origin"]` section that exists but has no valid
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

    /// ERR-1 / TASK-1215: when last-wins picks up a trailing `url = ...`
    /// line that gets dropped for embedded ASCII control bytes (e.g. an
    /// ANSI escape), the previous valid URL must still be returned AND the
    /// drop must surface as a warn-level event with a rejected-line count
    /// so the operator can tell "stale URL" from "all URLs malformed".
    #[test]
    fn read_origin_url_warns_on_control_byte_drop_keeping_prior_valid() {
        // Two `url = ...` lines: a valid one, then a trailing line with an
        // embedded ANSI escape. Pre-fix: silent debug-only breadcrumb, the
        // valid earlier URL is returned (last-wins is *masked*). Post-fix:
        // the valid URL is still returned (we have no later valid value)
        // AND a warn fires with the rejected count.
        let cfg = "\
[remote \"origin\"]
\turl = https://github.com/real/repo.git
\turl = https://example.com/\u{001b}[31mrogue\u{001b}[0m
";
        // DUP-3 / TASK-2014: the shared harness pins a global dispatcher for
        // us. TEST-15 / TASK-1664: this test open-coded the capture scaffold
        // without that pin and failed 1 run in 30 under 16-core load,
        // reporting an empty buffer while the parser assertion below passed.
        let (logged, url) = ops_core::test_utils::capture_tracing(tracing::Level::WARN, || {
            read_origin_url_from(cfg).map(RedactedUrl::into_string)
        });
        assert_eq!(
            url,
            Some("https://github.com/real/repo.git".to_string()),
            "must fall back to the previous valid url= line"
        );

        assert!(
            logged.contains("WARN") && logged.contains("TASK-1215"),
            "expected one TASK-1215 warn-level event; got: {logged}"
        );
        assert!(
            logged.contains("rejected=1"),
            "warn must include rejected-line count; got: {logged}"
        );
    }

    /// READ-2 / TASK-1213: a quoted `url = "..."` value containing an
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

    /// READ-2 / TASK-1213: quoted form decodes the same `\\\\` / `\\"` escapes
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

    /// ERR-1 / TASK-1244: a single non-UTF-8 byte anywhere in `.git/config`
    /// (BOM, latin-1 commit-template, hostile injection in an unrelated
    /// section) used to fail the whole-file `read_to_string` decode and
    /// surface as a generic IO warn — remote detection silently zeroed out
    /// even when the `[remote "origin"]` block was well-formed UTF-8. The
    /// helper now lossy-decodes per-byte so the well-formed url= line
    /// survives.
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

    /// ERR-1 / TASK-1620: the SEC-33 size cap is checked on raw bytes
    /// *before* lossy UTF-8 decoding. A `.git/config` whose raw size is at
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
            "test payload must be within the SEC-33 cap"
        );
        std::fs::write(git_dir.join("config"), &bytes).unwrap();

        assert_eq!(
            read_origin_url(&git_dir).map(RedactedUrl::into_string),
            Some("https://github.com/o/r.git".to_string()),
            "in-cap config with non-UTF-8 bytes must still surface the origin URL"
        );
    }

    /// SEC-33 / TASK-0910: a `.git/config` larger than `MAX_GIT_CONFIG_BYTES`
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

    /// SEC-2 / TASK-1102: a `.git/config` `url = ...` value containing
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

    /// SEC-2 / TASK-1238: bidi / zero-width / line-separator codepoints
    /// must also be rejected before reaching About cards / JSON / logs
    /// through `RedactedUrl`. The ASCII gate alone (TASK-1102) was bypassed
    /// by multibyte sequences for U+202E (RTL OVERRIDE), U+200B / U+200D
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

    /// ERR-1 / TASK-0887: an unreadable HEAD must return `None` (matching
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

    /// ERR-7 / TASK-1206: `read_origin_url` logs the .git/config path through
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

    /// SEC-21 / ERR-7 / TASK-1871: `is_origin_header` logs the raw
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

    /// SEC-21 / ERR-7 / TASK-1871: the HEAD path takes the same Debug
    /// formatter as the `.git/config` path — this arm logged with `%`.
    #[test]
    fn read_head_branch_path_debug_escapes_control_characters() {
        let p = std::path::Path::new("/tmp/dir\n\u{1b}[31m/.git/HEAD");
        let rendered = format!("{:?}", p.display());
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

    /// TASK-1871 AC#4: no `tracing` call in this crate may Display-format a
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

    /// SEC-2 / TASK-1863: an ANSI escape in the ref repaints the operator's
    /// terminal wherever `git_info.branch` is rendered. `read_origin_url`
    /// dropped such values from day one; this reader shipped them raw.
    #[test]
    fn head_branch_with_ansi_escape_is_rejected() {
        let (_d, git_dir) = write_head("ref: refs/heads/main\u{1b}[2J\u{1b}[31mFAKE\n");
        assert_eq!(read_head_branch(&git_dir), None);
    }

    /// SEC-2 / TASK-1863: U+202E RIGHT-TO-LEFT OVERRIDE is the homograph
    /// surface TASK-1238 closed for the remote URL.
    #[test]
    fn head_branch_with_bidi_override_is_rejected() {
        let (_d, git_dir) = write_head("ref: refs/heads/ma\u{202e}in\n");
        assert_eq!(read_head_branch(&git_dir), None);
    }

    /// SEC-2 / TASK-1863: `trim` only removes leading / trailing whitespace,
    /// so an interior CR survives into the branch string.
    #[test]
    fn head_branch_with_interior_carriage_return_is_rejected() {
        let (_d, git_dir) = write_head("ref: refs/heads/main\rfake\n");
        assert_eq!(read_head_branch(&git_dir), None);
    }

    /// SEC-13 / TASK-1863: a traversal-shaped ref must not reach
    /// `git_info.branch` — the shape `remote::is_valid_path_segment`
    /// rejects on the remote side (TASK-0929).
    #[test]
    fn head_branch_with_dot_only_segment_is_rejected() {
        let (_d, git_dir) = write_head("ref: refs/heads/../../../etc\n");
        assert_eq!(read_head_branch(&git_dir), None);
        let (_d2, git_dir2) = write_head("ref: refs/heads/feature/./foo\n");
        assert_eq!(read_head_branch(&git_dir2), None);
    }

    /// SEC-2 / TASK-1863: the hardening must not cost ordinary branches —
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

    /// SEC-33 / TASK-1866 (supersedes the falsely-closed TASK-0927): a HEAD
    /// one byte over the cap must return `None` rather than allocating the
    /// file. `read_origin_url` has had this bound since TASK-0910.
    #[test]
    fn head_branch_over_byte_cap_is_rejected() {
        let cap = usize::try_from(MAX_HEAD_BYTES).unwrap_or(usize::MAX);
        let prefix = "ref: refs/heads/";
        let oversized = format!("{prefix}{}", "a".repeat(cap + 1 - prefix.len()));
        assert_eq!(oversized.len(), cap + 1);
        let (_d, git_dir) = write_head(&oversized);
        assert_eq!(read_head_branch(&git_dir), None);
    }

    /// SEC-33 / TASK-1866: exactly at the cap still parses, so the bound is
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

    /// READ-5 / TASK-1876: git starts a comment at an unquoted `#` / `;`
    /// anywhere on the line, so these are ordinary headers. Before the fix
    /// `strip_suffix(']')` failed, `in_origin` went false, and every
    /// `url =` line in the section was silently dropped.
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

    /// READ-5 / TASK-1876 AC#2: a `;` or `#` *inside* the quoted subsection
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

    /// READ-5 / TASK-1876: an origin section whose *name* carries the
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

    /// READ-5 / TASK-1876 AC#3: the header-line key form stays unsupported
    /// and documented — pin the behaviour so the limitation list stays true.
    #[test]
    fn header_line_key_form_remains_unsupported() {
        let cfg = "[remote \"origin\"] url = https://github.com/o/r.git\n";
        assert_eq!(read_origin_url_from(cfg), None);
    }
}
