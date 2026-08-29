//! Terraform stack `project_identity` provider.
//!
//! Parses `.tf` files for `required_version` constraints and counts local
//! modules under `modules/*/`. No terraform subprocess — purely filesystem.
//!
//! # Manifest IO policy
//!
//! ERR-1 / TASK-0851: every `.tf` read goes through
//! [`ops_about::manifest_io::read_optional_text`] so the project-wide rule
//! "missing manifest is silent, real IO error is `tracing::warn!`-and-fall-
//! back" applies here the same way it does in the Python / Go siblings.
//! A permission-denied / EIO / "is a directory" failure on `versions.tf`
//! is therefore distinguishable from "no version declared" in the logs.
//! The directory enumeration in [`find_required_version`] mirrors the
//! same policy — non-NotFound `read_dir` failures are logged at `warn`,
//! and ERR-1 / TASK-1772 extends it to *per-entry* failures in both
//! [`fallback_tf_paths`] and [`count_local_modules`], which previously
//! dropped them through `flatten()` / `Path::exists()`.
//!
//! # Rendered values are untrusted
//!
//! SEC-11 / TASK-1775: `ops about` runs inside repositories the operator
//! cloned but did not audit, and `stack_detail` reaches the terminal with no
//! escaping layer in between. [`sanitize_required_version`] is the single
//! producing-side gate: a value carrying control characters is dropped, not
//! stripped, matching `ops_about::text_util`'s policy for manifest URL and
//! repository fields.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
use ops_core::project_identity::{base_about_fields, AboutFieldDef};
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};

const NAME: &str = "about-terraform";
const DESCRIPTION: &str = "Terraform project identity";
const SHORTNAME: &str = "about-terraform";
const DATA_PROVIDER_NAME: &str = "project_identity";

#[non_exhaustive]
pub struct AboutTerraformExtension;

ops_extension::impl_extension! {
    AboutTerraformExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Terraform),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        let _ = registry.register(DATA_PROVIDER_NAME, Box::new(TerraformIdentityProvider));
    },
    factory: TERRAFORM_ABOUT_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutTerraformExtension)))
    },
}

struct TerraformIdentityProvider;

impl DataProvider for TerraformIdentityProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn about_fields(&self) -> Vec<AboutFieldDef> {
        base_about_fields()
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        provide_identity_from_manifest(ctx.working_directory(), |root| {
            let required_version = find_required_version(root);
            let module_count = count_local_modules(root);

            let stack_detail = required_version.map(|v| format!("Terraform {v}"));

            ParsedManifest::build(|m| {
                m.stack_label = "Terraform";
                m.stack_detail = stack_detail;
                m.module_label = "modules";
                m.module_count = module_count;
            })
        })
    }
}

/// Well-known `.tf` filenames probed before the directory walk.
const CANDIDATE_FILES: [&str; 4] = ["versions.tf", "main.tf", "terraform.tf", "version.tf"];

/// Scan `.tf` files for `required_version` in a `terraform` block.
///
/// Looks for patterns like `required_version = ">= 1.5"` or
/// `required_version = "~> 1.0"`. Only the first match is used.
fn find_required_version(root: &Path) -> Option<String> {
    for candidate in CANDIDATE_FILES {
        let path = root.join(candidate);
        // ERR-1 / TASK-0851: route through the shared helper so a
        // permission-denied / EIO / "is a directory" failure surfaces as
        // tracing::warn! instead of silently degrading to "no version".
        if let Some(content) = ops_about::manifest_io::read_optional_text(&path, candidate) {
            if let Some(v) = extract_required_version(&content) {
                return Some(v);
            }
        }
    }
    for path in fallback_tf_paths(root) {
        let kind = path.file_name().map_or_else(
            || "<unnamed>.tf".to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        if let Some(content) = ops_about::manifest_io::read_optional_text(&path, &kind) {
            if let Some(v) = extract_required_version(&content) {
                return Some(v);
            }
        }
    }
    None
}

/// The `.tf` files in `root` that [`find_required_version`]'s fallback walk
/// should read, in a deterministic order.
///
/// CL-3 / TASK-0852: `read_dir` ordering is platform-dependent (ext4 hash
/// order, APFS insertion-ish order, Windows alphabetical) so the
/// first-match-wins fallback used to produce non-deterministic results across
/// operators when several `.tf` files declared different `required_version`
/// strings. Sorting by path makes the alphabetically-first `.tf` carrying a
/// constraint the documented, reproducible winner.
///
/// ERR-1 / TASK-1772: per-entry `read_dir` failures are logged rather than
/// dropped by `flatten()`, matching the module-level IO policy and
/// [`count_local_modules`].
///
/// PERF-3 / TASK-1782: files already probed by the named-candidate loop are
/// skipped, so the common "a `main.tf` with no constraint" project reads and
/// parses that file once instead of twice. The comparison is exact-name: on a
/// case-sensitive filesystem a `Main.TF` is a genuinely different file that
/// `root.join("main.tf")` never opened.
fn fallback_tf_paths(root: &Path) -> Vec<PathBuf> {
    // A non-NotFound read_dir failure deserves a warn — same rationale as the
    // per-candidate reads. NotFound on the workspace root is silent (the
    // caller falls back).
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            tracing::warn!(
                root = ?root.display(),
                error = %e,
                "failed to enumerate workspace root for .tf files"
            );
            return Vec::new();
        }
    };
    let mut tf_paths: Vec<PathBuf> = entries
        .filter_map(|res| match res {
            Ok(entry) => Some(entry.path()),
            Err(e) => {
                tracing::warn!(
                    root = ?root.display(),
                    error = %e,
                    "failed to read directory entry in workspace root"
                );
                None
            }
        })
        .filter(|p| has_tf_extension(p))
        .filter(|p| !is_named_candidate(p))
        .collect();
    tf_paths.sort();
    tf_paths
}

/// PATTERN-1 / TASK-1025: compare the extension ASCII-case-insensitively so a
/// `Custom.TF` / `Versions.Tf` file (preserved-case on macOS APFS, Windows
/// NTFS) is found by the fallback walk, matching the targeted candidate list
/// which already resolves case-insensitively via the filesystem.
fn has_tf_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("tf"))
}

/// Whether `path`'s file name is one the named-candidate loop already read.
fn is_named_candidate(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| CANDIDATE_FILES.contains(&n))
}

/// SEC-11 / TASK-0853: cap on the rendered `required_version` string.
/// HCL constraints in real configs are short (`~> 1.5`, `>= 1.0, < 2.0`,
/// etc.) — well under 64 chars. An adversarial `.tf` could otherwise
/// embed a long string that ends up rendered into the About card; we
/// truncate at this cap and log so the truncation is observable.
const REQUIRED_VERSION_MAX_LEN: usize = 64;

/// Extract the `required_version` value from a single `.tf` file's content.
///
/// FN-1 / TASK-1779: the scan is three separable stages, each independently
/// testable, rather than one loop body mixing all of them —
/// [`strip_comments`] blanks every comment form, [`scan_line`] tracks block
/// structure and locates the assignment, and [`sanitize_required_version`]
/// applies the SEC-11 policy to the extracted string. The three correctness
/// bugs this shape replaced (brace-stack desync, unrecognised block openers,
/// comment-unaware stripping) all lived in the seams between those stages.
fn extract_required_version(content: &str) -> Option<String> {
    // PATTERN-1 / TASK-1020 + TASK-1768 + TASK-1771: blank every comment form
    // up front so the structural scan below never has to reason about them.
    let stripped = strip_comments(content);
    // ERR-2 / TASK-0919: only accept `required_version = "…"` when it
    // appears at the top level of a `terraform { … }` block.
    let mut state = ScanState::new();
    for line in stripped.lines() {
        match scan_line(line, &mut state) {
            LineScan::Continue => {}
            LineScan::Found(value) => return sanitize_required_version(&value),
            LineScan::Malformed => {
                // PATTERN-1 / TASK-1765: a `}` with nothing to close means the
                // braces do not balance, so every depth judgement after it
                // would be guesswork. Refuse the file rather than render a
                // constraint read at an unknown nesting level.
                tracing::warn!("unbalanced closing brace in .tf content; skipping file");
                return None;
            }
        }
    }
    None
}

/// The HCL block nesting the scanner is currently inside.
///
/// PATTERN-1 / TASK-1765: an entry is `Some(ident)` for a named block opener
/// (`terraform {`, `provider "aws" {`) and `None` for any *other* brace — an
/// object-valued attribute (`aws = {`), a `default = { … }`, an expression.
/// The earlier implementation pushed only named openers while popping on every
/// `}`, so a single `aws = {` desynchronised the stack and closed the
/// enclosing `terraform` block early. Pushing a sentinel keeps pushes and pops
/// balanced whatever the brace was.
type BlockStack = Vec<Option<String>>;

/// Everything [`scan_line`] carries from one line to the next.
///
/// PATTERN-1 / TASK-2031: the block stack alone was not enough state. An HCL
/// heredoc (`<<EOT` / `<<-EOT` … `EOT`) is an unquoted multi-line string, so
/// its body is not HCL at all: a bare `}` in a shell snippet is not a
/// structural close, and `#` starts nothing. Tracking the pending terminator
/// beside the stack is what lets the scanner skip those lines instead of
/// reading them as structure.
struct ScanState {
    stack: BlockStack,
    /// `Some(open)` while the scanner is inside a heredoc body.
    heredoc: Option<Heredoc>,
}

/// An open heredoc body: the terminator that closes it, and whether the
/// `<<-` spelling was used.
///
/// HCL only allows the terminator of a `<<-` heredoc to be indented; for the
/// plain `<<` spelling it must start the line. Tracking which spelling opened
/// the body is what lets [`Heredoc::closes`] apply the right rule, so a body
/// line that merely *looks* like an indented terminator no longer ends an
/// ordinary heredoc early.
#[derive(Debug, Clone)]
struct Heredoc {
    terminator: String,
    indented: bool,
}

impl Heredoc {
    /// Does `line` close this heredoc?
    ///
    /// Trailing whitespace is ignored for both spellings so a CRLF file and a
    /// trailing space behave the same; leading whitespace is tolerated only
    /// for `<<-`.
    fn closes(&self, line: &str) -> bool {
        let candidate = if self.indented {
            line.trim()
        } else {
            line.trim_end()
        };
        candidate == self.terminator
    }
}

impl ScanState {
    const fn new() -> Self {
        Self {
            stack: BlockStack::new(),
            heredoc: None,
        }
    }
}

/// Outcome of scanning one already-comment-stripped line.
enum LineScan {
    /// Nothing of interest; `stack` has been updated for this line's braces.
    Continue,
    /// A `required_version` value at the top level of a `terraform` block.
    Found(String),
    /// A `}` appeared with an empty stack — the input is not balanced HCL.
    Malformed,
}

/// Walk one line, updating `stack` for every structural brace and reporting a
/// `required_version` assignment found at the top level of `terraform { … }`.
///
/// PATTERN-1 / TASK-1768: braces and assignments are located per *token*, not
/// per line, so `terraform { # comment`, `required_version = "…" }` and
/// `locals { x = 1 }` all track correctly. Quoted strings are skipped so a
/// brace or quote inside a value is never structural.
///
/// PATTERN-1 / TASK-2031: heredoc bodies are skipped whole. A `<<EOT` opener
/// outside a string switches the scanner into `state.heredoc` until a line
/// whose trimmed content is the terminator; nothing in between updates the
/// block stack or yields a value. Without that, a `}` in a shell snippet
/// popped a level the file never opened, and with the TASK-1765 balance check
/// in place that silently dropped the whole file.
fn scan_line(line: &str, state: &mut ScanState) -> LineScan {
    if let Some(open) = state.heredoc.as_ref() {
        // The opener's spelling decides whether an indented terminator counts
        // (`<<-`) or only one that starts the line (`<<`). Matching HCL here
        // matters because a body line whose *trimmed* text happens to equal
        // the terminator would otherwise end an ordinary heredoc early, and
        // the rest of that body would then be read as structure.
        if open.closes(line) {
            state.heredoc = None;
        }
        return LineScan::Continue;
    }
    let mut segment_start = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in line.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        // `{` and `}` are single-byte ASCII, so `idx + 1` is a char boundary.
        let after_brace = idx.saturating_add(1);
        match ch {
            '"' => in_string = true,
            '<' => {
                // `<<EOT` / `<<-EOT`: the body starts on the next line, and
                // HCL allows nothing after the opener on this one, so the
                // rest of the line holds no structure to track.
                if let Some((terminator, indented)) = line
                    .get(after_brace..)
                    .and_then(|rest| rest.strip_prefix('<'))
                    .and_then(heredoc_terminator)
                {
                    state.heredoc = Some(Heredoc {
                        terminator: terminator.to_owned(),
                        indented,
                    });
                    return LineScan::Continue;
                }
            }
            '{' => {
                let prefix = line.get(segment_start..idx).unwrap_or_default();
                state
                    .stack
                    .push(block_open_ident(prefix).map(ToOwned::to_owned));
                segment_start = after_brace;
            }
            '}' => {
                if let Some(found) = required_version_here(line, segment_start, idx, &state.stack) {
                    return LineScan::Found(found);
                }
                if state.stack.pop().is_none() {
                    return LineScan::Malformed;
                }
                segment_start = after_brace;
            }
            _ => {}
        }
    }
    required_version_here(line, segment_start, line.len(), &state.stack)
        .map_or(LineScan::Continue, LineScan::Found)
}

/// Is `c` valid as the first character of a heredoc terminator?
///
/// HCL identifiers are Unicode (`UAX #31` plus `-`), so `<<終端` is a legal
/// opener. `char::is_alphabetic` is used as the `ID_Start` approximation rather
/// than pulling in a `unicode-ident` dependency for one scanner: it accepts
/// every identifier HCL does, and over-accepting here only means recognising
/// a heredoc the parser would have rejected anyway.
fn is_heredoc_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

/// Is `c` valid inside a heredoc terminator, after the first character?
fn is_heredoc_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

/// The terminator of a heredoc opener, given the text following its `<<`.
///
/// Returns the terminator and whether the indent-stripping `<<-EOT` spelling
/// was used, or `None` when what follows is not an identifier, so an ordinary
/// comparison never opens a phantom heredoc. This is the single statement of
/// the rule; both [`scan_line`] and [`strip_comments`] recognise openers with
/// it.
fn heredoc_terminator(after_marker: &str) -> Option<(&str, bool)> {
    let indented = after_marker.starts_with('-');
    let rest = after_marker.strip_prefix('-').unwrap_or(after_marker);
    let mut end = 0usize;
    for (idx, ch) in rest.char_indices() {
        let ok = if idx == 0 {
            is_heredoc_ident_start(ch)
        } else {
            is_heredoc_ident_char(ch)
        };
        if !ok {
            break;
        }
        // Advance by the character's own width so a multi-byte identifier
        // char leaves `end` on a char boundary.
        end = idx.saturating_add(ch.len_utf8());
    }
    if end == 0 {
        None
    } else {
        rest.get(..end).map(|term| (term, indented))
    }
}

/// The `required_version` value declared in `line[start..end]`, if that
/// fragment is an assignment *and* `stack` says we are at the top level of a
/// `terraform` block.
///
/// ERR-2 / TASK-0919: anywhere else (top level, nested deeper, or inside a
/// `module` / `provider` block) the key is HCL-valid but is not the terraform
/// stack constraint we want to render.
fn required_version_here(
    line: &str,
    start: usize,
    end: usize,
    stack: &BlockStack,
) -> Option<String> {
    if !matches!(stack.as_slice(), [Some(name)] if name == "terraform") {
        return None;
    }
    let fragment = line.get(start..end)?;
    parse_required_version_assignment(fragment).map(ToOwned::to_owned)
}

/// Parse a `required_version = "…"` assignment out of a brace-free fragment.
///
/// SEC-11 / TASK-0853: HCL standardises the value as a double-quoted string,
/// so a bare or single-quoted value is rejected — surfacing it would mislead
/// the operator about what the manifest actually says. Comments are already
/// blanked by [`strip_comments`], so anything left after the closing quote is
/// genuine trailing content and disqualifies the line.
fn parse_required_version_assignment(fragment: &str) -> Option<&str> {
    let rest = fragment.trim().strip_prefix("required_version")?;
    let rest = rest.trim_start().strip_prefix('=')?;
    let rest = rest.trim_start().strip_prefix('"')?;
    let (value, after_close) = rest.split_once('"')?;
    if !after_close.trim().is_empty() {
        return None;
    }
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// SEC-11 / TASK-1775 + TASK-0853: make an extracted value safe to render.
///
/// Control characters are *dropped*, not stripped: the value reaches the
/// operator's terminal through `ProjectIdentity::stack_detail` with no
/// escaping layer in between, so `required_version = "1.0\u{1b}[2Jowned"` in
/// an unaudited checkout would clear the screen and print text the operator
/// never authored. Stripping would silently splice the attacker-controlled
/// tail onto the legitimate prefix; dropping surfaces the field as missing.
/// This is the same drop-not-strip policy `ops_about::text_util` applies to
/// manifest URL and repository fields, reusing its shared predicate.
fn sanitize_required_version(value: &str) -> Option<String> {
    if ops_about::text_util::contains_control_chars(value) {
        tracing::warn!(
            len = value.len(),
            "required_version contains control characters; dropping the value"
        );
        return None;
    }
    if value.chars().count() > REQUIRED_VERSION_MAX_LEN {
        let truncated: String = value.chars().take(REQUIRED_VERSION_MAX_LEN).collect();
        tracing::warn!(
            original_len = value.chars().count(),
            cap = REQUIRED_VERSION_MAX_LEN,
            "required_version value exceeds cap; truncating before rendering"
        );
        return Some(truncated);
    }
    Some(value.to_string())
}

/// ERR-2 / TASK-0919: extract the leading identifier of an HCL block opener
/// from the text preceding its `{` — `terraform`, `provider "aws"`,
/// `required_providers`. Returns `None` when the brace is not a named block
/// opener, which [`scan_line`] records as an anonymous stack entry.
fn block_open_ident(prefix: &str) -> Option<&str> {
    let prefix = prefix.trim();
    // An `=` before the brace means an object-valued attribute (`aws = {`),
    // not a block opener.
    if prefix.contains('=') {
        return None;
    }
    // Identifier = leading run of [A-Za-z_][A-Za-z0-9_-]*
    let bytes = prefix.as_bytes();
    let mut end = 0usize;
    while let Some(&b) = bytes.get(end) {
        let ok = if end == 0 {
            b.is_ascii_alphabetic() || b == b'_'
        } else {
            b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
        };
        if !ok {
            break;
        }
        // The loop guard keeps `end < bytes.len()`, so this increment stays
        // within `usize` and `saturating_add` is exactly `+ 1`.
        end = end.saturating_add(1);
    }
    if end == 0 {
        return None;
    }
    // `end` counts ASCII identifier bytes from the start, so it is always a
    // char boundary; `get` returning `None` is unreachable and degrades to
    // "not a block opener".
    prefix.get(..end)
}

/// PATTERN-1 / TASK-1020 + TASK-1771: blank every HCL comment — `#`, `//` and
/// `/* … */` — with spaces, preserving newlines, so the downstream scanner
/// sees a structurally-equivalent file with the comment bodies removed.
///
/// All three forms are resolved in the *same* pass, outside double-quoted
/// strings, because they interact: a `/*` inside a `# …` comment
/// (`# see https://example.com/*note`) must not open a block comment that
/// blanks the rest of the file, and an unbalanced `"` inside a comment
/// (`# don't use "old style`) must not put the scanner into string state for
/// everything that follows. Terraform's own lexer resolves `#` / `//` before
/// `/*` for the same reason.
///
/// A `/*`, `#` or `//` inside a quoted HCL string stays literal —
/// `required_version = "~> 1.5 # marker"` keeps its marker. An unterminated
/// `/*` runs to EOF, the same behaviour as terraform's own parser.
///
/// PATTERN-1 / TASK-2031: a heredoc body is passed through **verbatim**. It is
/// an unquoted string literal, so a `#` line inside it is shell or policy
/// text, not an HCL comment, and blanking it would corrupt the very content
/// the scanner is asked to reason about. [`scan_line`] recognises the same
/// openers and skips the body, so nothing downstream reads it as structure.
///
/// PERF-3 / TASK-1782: returns [`Cow::Borrowed`] when the content carries no
/// comment introducer at all, so the common case allocates nothing. A file
/// whose only "comments" live inside heredocs still takes the owned path —
/// the fast check is deliberately syntax-free — and comes back unchanged.
fn strip_comments(content: &str) -> Cow<'_, str> {
    if !content.contains("/*") && !content.contains('#') && !content.contains("//") {
        return Cow::Borrowed(content);
    }
    let mut out = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_string = false;
    // Heredoc state. `pending` holds the terminator of an opener seen on the
    // current line; it becomes `heredoc` once the newline that begins the body
    // has been emitted. `body_line` accumulates the current body line so the
    // terminator can be recognised on it.
    let mut pending: Option<Heredoc> = None;
    let mut heredoc: Option<Heredoc> = None;
    let mut body_line = String::new();
    while let Some(c) = chars.next() {
        if pending.is_some() && out.ends_with('\n') {
            heredoc = pending.take();
            body_line.clear();
        }
        if let Some(open) = heredoc.as_ref() {
            out.push(c);
            if c == '\n' {
                if open.closes(&body_line) {
                    heredoc = None;
                }
                body_line.clear();
            } else {
                body_line.push(c);
            }
            continue;
        }
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(next) = chars.next() {
                    // Preserve `\"` and other escapes verbatim — we only
                    // care about not exiting the string on an escaped quote.
                    out.push(next);
                }
                continue;
            }
            if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push('"');
            }
            '#' => blank_line_comment(&mut chars, &mut out, 1),
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                blank_line_comment(&mut chars, &mut out, 2);
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                blank_block_comment(&mut chars, &mut out);
            }
            '<' if chars.peek() == Some(&'<') => {
                out.push('<');
                let _ = chars.next();
                out.push('<');
                let indented = chars.peek() == Some(&'-');
                if indented {
                    let _ = chars.next();
                    out.push('-');
                }
                let mut terminator = String::new();
                while let Some(&next) = chars.peek() {
                    let ok = if terminator.is_empty() {
                        is_heredoc_ident_start(next)
                    } else {
                        is_heredoc_ident_char(next)
                    };
                    if !ok {
                        break;
                    }
                    terminator.push(next);
                    out.push(next);
                    let _ = chars.next();
                }
                if !terminator.is_empty() {
                    pending = Some(Heredoc {
                        terminator,
                        indented,
                    });
                }
            }
            _ => out.push(c),
        }
    }
    Cow::Owned(out)
}

/// Blank a `#` / `//` comment through to (and excluding) the newline, which is
/// emitted verbatim so line-based logic downstream stays aligned.
///
/// `marker_len` is the width of the introducer already consumed by the caller,
/// replaced with the same number of spaces so byte offsets do not shift.
fn blank_line_comment(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    out: &mut String,
    marker_len: usize,
) {
    for _ in 0..marker_len {
        out.push(' ');
    }
    for inner in chars.by_ref() {
        if inner == '\n' {
            out.push('\n');
            return;
        }
        out.push(' ');
    }
}

/// Blank a `/* … */` span, preserving newlines. The caller has already
/// consumed the `/*`, whose two bytes are re-emitted as spaces.
fn blank_block_comment(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, out: &mut String) {
    out.push(' ');
    out.push(' ');
    while let Some(inner) = chars.next() {
        if inner == '*' && chars.peek() == Some(&'/') {
            chars.next();
            out.push(' ');
            out.push(' ');
            return;
        }
        if inner == '\n' {
            out.push('\n');
        } else {
            out.push(' ');
        }
    }
}

/// Count local modules under `modules/*/`.
///
/// PATTERN-1 / TASK-1796: a subdirectory counts as a module when it contains
/// at least one `.tf` (or `.tf.json`) file. Terraform's own definition is
/// exactly that — `main.tf` is a convention, not a requirement — so the
/// previous `modules/*/main.tf` probe reported `modules/network/network.tf`
/// and `modules/vpc/{variables,outputs,resources}.tf` layouts as zero modules,
/// which renders as no `modules` line at all rather than an undercount.
///
/// ERR-1 / TASK-1018 + TASK-1772: distinguish a missing `modules/` directory
/// (the expected "no local modules" case) from a real IO failure (permission
/// denied, EIO, "is not a directory") so operators see a `tracing::warn!`
/// instead of silently rendering "no modules". Per-entry and per-subdirectory
/// failures are logged too, rather than folded into "not a module".
fn count_local_modules(root: &Path) -> Option<usize> {
    let modules_dir = root.join("modules");
    let entries = match std::fs::read_dir(&modules_dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            tracing::warn!(
                modules_dir = ?modules_dir.display(),
                error = %e,
                "failed to enumerate modules directory"
            );
            return None;
        }
    };
    let count = entries
        .filter_map(|res| match res {
            Ok(entry) => Some(entry),
            Err(e) => {
                tracing::warn!(
                    modules_dir = ?modules_dir.display(),
                    error = %e,
                    "failed to read directory entry under modules/"
                );
                None
            }
        })
        .filter(is_module_dir)
        .count();
    if count > 0 {
        Some(count)
    } else {
        None
    }
}

/// Whether a `modules/` child is a directory holding terraform sources.
///
/// Uses `fs::metadata` (which follows symlinks) rather than
/// `DirEntry::file_type` (which does not): a `modules/` entry that is a
/// symlink to a real module directory is a normal terraform layout —
/// shared modules vendored once and linked per stack — and `file_type`
/// reported it as `Symlink`, not `Dir`, so the count silently omitted it.
/// This function only *counts* modules for the about report; it never opens
/// or writes anything under the target, and `contains_terraform_source`
/// likewise only lists names, so following the link grants no capability an
/// operator running `ops about` in their own checkout did not already have.
fn is_module_dir(entry: &std::fs::DirEntry) -> bool {
    match std::fs::metadata(entry.path()) {
        Ok(m) if !m.is_dir() => return false,
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return false,
        Err(e) => {
            // ERR-1 / TASK-1772: an unreadable entry is an IO failure, not
            // evidence that it is not a module. A dangling symlink resolves
            // to `NotFound` above and is simply not a module.
            tracing::warn!(
                entry = ?entry.path().display(),
                error = %e,
                "failed to stat entry under modules/"
            );
            return false;
        }
    }
    contains_terraform_source(&entry.path())
}

/// Whether `dir` directly contains at least one `.tf` / `.tf.json` file.
///
/// One `read_dir` answers the question the old `main.tf` `exists()` probe
/// could only approximate, and `Path::exists()` folded every non-NotFound
/// error into `false` by contract — this reports them.
fn contains_terraform_source(dir: &Path) -> bool {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return false,
        Err(e) => {
            tracing::warn!(
                module_dir = ?dir.display(),
                error = %e,
                "failed to enumerate module directory"
            );
            return false;
        }
    };
    entries
        .filter_map(|res| match res {
            Ok(entry) => Some(entry),
            Err(e) => {
                tracing::warn!(
                    module_dir = ?dir.display(),
                    error = %e,
                    "failed to read directory entry in module directory"
                );
                None
            }
        })
        .any(|entry| is_terraform_source_name(&entry.file_name()))
}

/// PATTERN-1 / TASK-1796: `.tf` and `.tf.json` (terraform's native JSON
/// syntax), compared ASCII-case-insensitively for consistency with
/// [`has_tf_extension`].
fn is_terraform_source_name(name: &std::ffi::OsStr) -> bool {
    let path = Path::new(name);
    if has_tf_extension(path) {
        return true;
    }
    // `<name>.tf.json` — terraform's native JSON syntax. `Path` exposes only
    // the last extension, so the `.tf` half is read off the file stem.
    let is_json = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json"));
    is_json
        && path
            .file_stem()
            .is_some_and(|stem| has_tf_extension(Path::new(stem)))
}

#[cfg(test)]
mod tests {
    use super::*;
    // DUP-1 / TASK-1788: the six-line `write` fixture helper used to be
    // copied verbatim here and in three sibling about crates. One shared
    // definition keeps a future tightening (error propagation, a setup
    // message) from having to land four times.
    use ops_about::test_support::{capture_warn, write_file as write};
    use ops_core::project_identity::ProjectIdentity;
    use ops_extension::{DataRegistry, Extension, Stack};

    // TEST-5 / TASK-1792: the crate's public surface is the `Extension` impl,
    // and nothing used to touch it — every test drove the private provider.
    // A drift between `DATA_PROVIDER_NAME` and the registration key produces
    // an extension that compiles, links into the `linkme` registry and
    // silently provides nothing.
    ops_extension::test_datasource_extension!(
        AboutTerraformExtension,
        name: "about-terraform",
        data_provider: "project_identity"
    );

    /// TEST-5 / TASK-1792: the remaining half of the host contract — the
    /// metadata the extension host dispatches on.
    #[test]
    fn extension_metadata_matches_contract() {
        let ext = AboutTerraformExtension;
        assert_eq!(ext.description(), "Terraform project identity");
        assert_eq!(ext.shortname(), "about-terraform");
        assert_eq!(ext.types(), ExtensionType::DATASOURCE);
        assert_eq!(ext.stack(), Some(Stack::Terraform));
        assert_eq!(ext.data_provider_name(), Some("project_identity"));
    }

    /// TEST-5 / TASK-1792: the registered provider must be the terraform one,
    /// not merely *a* provider under the right key.
    #[test]
    fn registered_provider_is_the_terraform_identity_provider() {
        let mut registry = DataRegistry::new();
        AboutTerraformExtension.register_data_providers(&mut registry);
        let provider = registry.get("project_identity").expect("provider");

        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("versions.tf"),
            "terraform {\n  required_version = \">= 1.5\"\n}\n",
        );
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).expect("provide");
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();
        assert_eq!(id.stack_label, "Terraform");
        assert_eq!(id.stack_detail.as_deref(), Some("Terraform >= 1.5"));
    }

    #[test]
    fn provider_name() {
        let provider = TerraformIdentityProvider;
        assert_eq!(provider.name(), "project_identity");
    }

    #[test]
    fn about_fields_match_base() {
        let provider = TerraformIdentityProvider;
        let fields = provider.about_fields();
        let base = base_about_fields();
        assert_eq!(fields.len(), base.len());
        for (a, b) in fields.iter().zip(base.iter()) {
            assert_eq!(a.id, b.id);
        }
    }

    #[test]
    fn provide_simple_terraform_project() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("main.tf"),
            "resource \"null_resource\" \"test\" {}\n",
        );

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        let expected = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(id.name, expected);
        assert_eq!(id.stack_label, "Terraform");
        assert_eq!(id.module_label, "modules");
        assert!(id.stack_detail.is_none());
    }

    #[test]
    fn provide_with_required_version() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("versions.tf"),
            r#"terraform {
  required_version = ">= 1.5"
}
"#,
        );
        write(&dir.path().join("main.tf"), "");

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(id.stack_detail.as_deref(), Some("Terraform >= 1.5"));
    }

    #[test]
    fn provide_with_modules() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("main.tf"), "");
        write(&dir.path().join("modules").join("api").join("main.tf"), "");
        write(
            &dir.path().join("modules").join("network").join("main.tf"),
            "",
        );
        // Not a module (no main.tf)
        std::fs::create_dir_all(dir.path().join("modules").join("empty")).unwrap();

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(id.module_count, Some(2));
    }

    #[test]
    fn provide_no_manifest_falls_back_to_dir_name() {
        let dir = tempfile::tempdir().unwrap();

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        let expected = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(id.name, expected);
        assert_eq!(id.stack_label, "Terraform");
        assert!(id.module_count.is_none());
        assert!(id.stack_detail.is_none());
    }

    #[test]
    fn provide_populates_repository_from_git_remote() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("main.tf"), "");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = https://github.com/o/r.git\n",
        )
        .unwrap();

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(id.repository.as_deref(), Some("https://github.com/o/r"));
    }

    /// CL-3 / TASK-0852: when none of the well-known candidates matches
    /// and the fallback walks every `*.tf` in the workspace root, the
    /// chosen winner must be deterministic across platforms — we sort the
    /// directory listing by filename so the alphabetically-first .tf
    /// file that carries a `required_version` is always the one returned.
    #[test]
    fn find_required_version_fallback_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        // Pick filenames that fall outside the named-candidate list
        // (versions.tf, main.tf, terraform.tf, version.tf) so we exercise
        // the read_dir fallback. The alphabetically-first should win.
        write(
            &dir.path().join("a-providers.tf"),
            "terraform {\n  required_version = \"~> 1.5\"\n}\n",
        );
        write(
            &dir.path().join("z-extras.tf"),
            "terraform {\n  required_version = \">= 99.0\"\n}\n",
        );
        let v = find_required_version(dir.path());
        assert_eq!(
            v,
            Some("~> 1.5".to_string()),
            "alphabetically-first .tf file with a constraint must win"
        );
    }

    /// PATTERN-1 / TASK-1025: a `.tf` file with a mixed-case extension
    /// (e.g. `Custom.TF`) carrying a `required_version` must be picked up
    /// by the fallback walk. Pre-fix the `OsStr` comparison was case-sensitive
    /// and silently skipped these files on case-preserving filesystems.
    #[test]
    fn find_required_version_fallback_matches_uppercase_extension() {
        let dir = tempfile::tempdir().unwrap();
        // Use a name outside the targeted candidate list so we exercise
        // the read_dir fallback specifically.
        write(
            &dir.path().join("Custom.TF"),
            "terraform {\n  required_version = \"~> 1.7\"\n}\n",
        );
        let v = find_required_version(dir.path());
        assert_eq!(
            v,
            Some("~> 1.7".to_string()),
            "uppercase .TF extension must be matched case-insensitively"
        );
    }

    /// ERR-1 / TASK-0851: a non-NotFound IO failure on `versions.tf`
    /// (e.g. the path is a directory) must surface a `tracing::warn!`
    /// instead of silently degrading to "no version declared".
    #[test]
    fn find_required_version_warns_when_versions_tf_is_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        // Create `versions.tf` as a directory, so read_to_string fails
        // with a non-NotFound error (IsADirectory / Other on most OSes).
        std::fs::create_dir(dir.path().join("versions.tf")).unwrap();

        // DUP-3 / TASK-1794: the shared tracing-capture harness replaces the
        // per-crate `BufWriter` + `MakeWriter` shim this test used to inline.
        let mut found = None;
        let logs = capture_warn(|| found = find_required_version(dir.path()));
        assert!(found.is_none(), "no required_version should be returned");
        assert!(
            logs.contains("failed to read manifest") && logs.contains("versions.tf"),
            "warn should name versions.tf and the read failure, got: {logs}"
        );
    }

    /// ERR-1 / TASK-1772: the counterpart for `count_local_modules` — a
    /// `modules` path that is a *file* fails `read_dir` with a non-NotFound
    /// error, which must be reported rather than folded into "no modules".
    #[test]
    fn count_local_modules_warns_when_modules_is_a_file() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("modules"), "not a directory\n");

        let mut count = Some(0);
        let logs = capture_warn(|| count = count_local_modules(dir.path()));
        assert!(count.is_none(), "a failed enumeration reports no modules");
        assert!(
            logs.contains("failed to enumerate modules directory"),
            "warn should name the enumeration failure, got: {logs}"
        );
    }

    #[test]
    fn extract_required_version_from_content() {
        let content = r#"terraform {
  required_version = "~> 1.0"
}
"#;
        assert_eq!(
            extract_required_version(content),
            Some("~> 1.0".to_string())
        );
    }

    #[test]
    fn extract_required_version_skips_comments() {
        let content = r#"terraform {
# required_version = "skip"
// required_version = "also skip"
required_version = ">= 1.5"
}
"#;
        assert_eq!(
            extract_required_version(content),
            Some(">= 1.5".to_string())
        );
    }

    #[test]
    fn extract_required_version_none_when_absent() {
        assert_eq!(
            extract_required_version("resource \"test\" \"x\" {}\n"),
            None
        );
    }

    /// ERR-2 / TASK-0919: a `required_version` declared inside a non-
    /// terraform block (e.g. a `module` or `provider`) must be ignored.
    /// Pre-fix the parser would happily return that string and the
    /// About card would advertise a stack version that wasn't actually
    /// the project's terraform constraint.
    #[test]
    fn extract_required_version_ignores_non_terraform_blocks() {
        let content = r#"
module "spurious" {
  required_version = ">= 99.0"
  source = "./modules/x"
}

terraform {
  required_version = "~> 1.5"
}
"#;
        assert_eq!(
            extract_required_version(content),
            Some("~> 1.5".to_string())
        );
    }

    /// ERR-2 / TASK-0919: when the only `required_version` lives in a
    /// non-terraform block, we surface "no version" (None) — the About
    /// card should fall back rather than report the wrong constraint.
    #[test]
    fn extract_required_version_returns_none_when_only_inside_non_terraform_block() {
        let content = r#"
provider "aws" {
  required_version = ">= 1.0"
}
"#;
        assert_eq!(extract_required_version(content), None);
    }

    /// ERR-2 / TASK-0919: `required_version` nested deeper than depth 1
    /// inside terraform (e.g. inside `required_providers` { … }) is also
    /// rejected — the top-level depth-1 declaration is the only valid
    /// shape.
    #[test]
    fn extract_required_version_rejects_nested_inside_terraform() {
        let content = r#"
terraform {
  required_providers {
    required_version = "5.0"
  }
}
"#;
        assert_eq!(extract_required_version(content), None);
    }

    /// SEC-11 / TASK-0853: a trailing `# ...` comment after the quoted
    /// value must be stripped before rendering — previously the entire
    /// remainder including the comment was returned verbatim.
    #[test]
    fn extract_required_version_strips_trailing_hash_comment() {
        assert_eq!(
            extract_required_version(
                "terraform {\nrequired_version = \">= 1.5\" # patch needed\n}\n"
            ),
            Some(">= 1.5".to_string())
        );
    }

    /// SEC-11 / TASK-0853: same for `// …`.
    #[test]
    fn extract_required_version_strips_trailing_slash_comment() {
        assert_eq!(
            extract_required_version("terraform {\nrequired_version = \">= 1.5\" // note\n}\n"),
            Some(">= 1.5".to_string())
        );
    }

    /// SEC-11 / TASK-0853: a `#` inside the quoted value is part of the
    /// value, not a comment introducer.
    #[test]
    fn extract_required_version_keeps_hash_inside_quotes() {
        assert_eq!(
            extract_required_version("terraform {\nrequired_version = \">= 1.5 # marker\"\n}\n"),
            Some(">= 1.5 # marker".to_string())
        );
    }

    /// SEC-11 / TASK-0853: HCL standardises double-quoted; a bare value
    /// (`required_version = >= 1.5 # comment`) must NOT be returned —
    /// surfacing it would mislead the operator about what the manifest
    /// actually says.
    #[test]
    fn extract_required_version_rejects_bare_value() {
        assert_eq!(
            extract_required_version("terraform {\nrequired_version = >= 1.5 # comment\n}\n"),
            None
        );
    }

    /// SEC-11 / TASK-0853: single-quoted values are not standard HCL and
    /// are rejected.
    #[test]
    fn extract_required_version_rejects_single_quoted() {
        assert_eq!(
            extract_required_version("terraform {\nrequired_version = '>= 1.5'\n}\n"),
            None
        );
    }

    /// SEC-11 / TASK-0853: an excessively long value is truncated to
    /// `REQUIRED_VERSION_MAX_LEN` before being rendered into the About card.
    #[test]
    fn extract_required_version_caps_overlong_value() {
        let long = "v".repeat(200);
        let content = format!("terraform {{\nrequired_version = \"{long}\"\n}}\n");
        let v = extract_required_version(&content).expect("Some");
        assert_eq!(v.len(), REQUIRED_VERSION_MAX_LEN);
        assert!(v.chars().all(|c| c == 'v'));
    }

    /// PATTERN-1 / TASK-1020: a `required_version` declaration that lives
    /// entirely inside an HCL `/* … */` block comment must NOT be
    /// extracted — block-commented declarations are by definition not
    /// the active terraform constraint. Pre-fix the parser ignored
    /// only `#` and `//` line comments and would surface ">= 99.0".
    #[test]
    fn extract_required_version_skips_block_comment() {
        let content = r#"terraform {
  /* required_version = ">= 99.0" */
}
"#;
        assert_eq!(extract_required_version(content), None);
    }

    /// PATTERN-1 / TASK-1020: when a block comment wraps a stale
    /// declaration but a live `required_version` follows on the same
    /// (or a subsequent) line, the live value must win. The block
    /// comment also must not corrupt block-depth tracking — the
    /// trailing `}` still has to close the `terraform` block.
    #[test]
    fn extract_required_version_uses_live_value_after_block_comment() {
        let content = "terraform {\n  /* TODO bump\n     required_version = \">= 99\" */ required_version = \"~> 1.5\"\n}\n";
        assert_eq!(
            extract_required_version(content),
            Some("~> 1.5".to_string())
        );
    }

    /// PATTERN-1 / TASK-1020: a `/*` that appears inside a quoted HCL
    /// string is part of the value, not a comment introducer — strip
    /// must not eat it.
    #[test]
    fn extract_required_version_keeps_block_comment_marker_inside_quotes() {
        let content = "terraform {\nrequired_version = \"~> 1.5 /* not a comment */\"\n}\n";
        assert_eq!(
            extract_required_version(content),
            Some("~> 1.5 /* not a comment */".to_string())
        );
    }

    /// PATTERN-1 / TASK-1765: the canonical terraform block — object-valued
    /// providers in `required_providers`, then `required_version`. Each
    /// `aws = {` opens a brace that is not a named block; pre-fix the stack
    /// popped one level too many on its `}` and closed `terraform` early, so
    /// this returned `None`.
    #[test]
    fn extract_required_version_after_object_valued_required_providers() {
        let content = r#"terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    random = {
      source = "hashicorp/random"
    }
  }
  required_version = ">= 1.5"
}
"#;
        assert_eq!(
            extract_required_version(content),
            Some(">= 1.5".to_string())
        );
    }

    /// PATTERN-1 / TASK-1765: the same file with the declaration order
    /// reversed. Both orders must work — pre-fix only this one did, which
    /// made the bug look like flakiness.
    #[test]
    fn extract_required_version_before_object_valued_required_providers() {
        let content = r#"terraform {
  required_version = ">= 1.5"
  required_providers {
    aws = {
      source = "hashicorp/aws"
    }
  }
}
"#;
        assert_eq!(
            extract_required_version(content),
            Some(">= 1.5".to_string())
        );
    }

    /// PATTERN-1 / TASK-1765: nested object-valued attributes at any depth
    /// keep the brace stack balanced — `cloud { workspaces = { … } }` is the
    /// other shape that used to desynchronise it.
    #[test]
    fn extract_required_version_after_nested_object_attributes() {
        let content = r#"terraform {
  cloud {
    organization = "acme"
    workspaces = { name = "prod" }
  }
  required_version = "~> 1.6"
}
"#;
        assert_eq!(
            extract_required_version(content),
            Some("~> 1.6".to_string())
        );
    }

    /// PATTERN-1 / TASK-1765: a `}` with nothing to close means the braces do
    /// not balance, so no depth judgement after it is trustworthy — the file
    /// is refused rather than yielding a constraint read at unknown nesting.
    #[test]
    fn extract_required_version_rejects_unbalanced_closing_brace() {
        let content = "}\nterraform {\n  required_version = \">= 1.5\"\n}\n";
        assert_eq!(extract_required_version(content), None);
    }

    /// PATTERN-1 / TASK-2031 AC#3/#4: a heredoc body containing a bare `}` is
    /// a string, not structure. Pre-fix the `}` popped a level the file never
    /// opened, which — with the TASK-1765 balance check — emptied the stack and
    /// refused the whole file, so the `terraform` block declared afterwards
    /// lost its constraint.
    #[test]
    fn extract_required_version_after_heredoc_with_a_bare_closing_brace() {
        let content = concat!(
            "locals {\n",
            "  script = <<-EOT\n",
            "    if [ -f x ]; then\n",
            "      echo hi\n",
            "    }\n",
            "  EOT\n",
            "}\n",
            "terraform {\n",
            "  required_version = \">= 1.5\"\n",
            "}\n",
        );
        assert_eq!(
            extract_required_version(content),
            Some(">= 1.5".to_string())
        );
    }

    /// PATTERN-1 / TASK-2031 AC#4: the `#` half of the same shape. The `#`
    /// line is shell text inside a string value; treating it as an HCL comment
    /// would blank the `}` that closes the heredoc's own `locals` block.
    #[test]
    fn extract_required_version_after_heredoc_with_a_hash_line() {
        let content = concat!(
            "locals {\n",
            "  script = <<EOT\n",
            "# not an HCL comment - a shell comment inside a string value\n",
            "EOT\n",
            "}\n",
            "terraform {\n",
            "  required_version = \"~> 1.6\"\n",
            "}\n",
        );
        assert_eq!(
            extract_required_version(content),
            Some("~> 1.6".to_string())
        );
    }

    /// PATTERN-1 / TASK-2031 AC#1: `strip_comments` passes a heredoc body
    /// through verbatim. The body is an unquoted string literal, so blanking
    /// its `#` line or opening a block comment on its `/*` would corrupt the
    /// content rather than remove a comment.
    #[test]
    fn strip_comments_leaves_heredoc_bodies_verbatim() {
        let content = concat!(
            "locals {\n",
            "  script = <<-EOT\n",
            "    # shell comment\n",
            "    glob=/*   // not a comment either\n",
            "  EOT\n",
            "}\n",
            "# a real comment\n",
        );
        let stripped = strip_comments(content);
        assert!(
            stripped.contains("    # shell comment\n")
                && stripped.contains("    glob=/*   // not a comment either\n"),
            "heredoc body must survive intact; got: {stripped}"
        );
        assert!(
            !stripped.contains("a real comment"),
            "an HCL comment outside the heredoc must still be blanked; got: {stripped}"
        );
    }

    /// PATTERN-1 / TASK-2031: an unbalanced `{` inside a heredoc is inert too,
    /// so the `terraform` block after it is still read at depth 1 rather than
    /// one level deeper.
    #[test]
    fn extract_required_version_after_heredoc_with_a_bare_opening_brace() {
        let content = concat!(
            "locals {\n",
            "  policy = <<EOT\n",
            "{ \"Statement\": [] \n",
            "EOT\n",
            "}\n",
            "terraform {\n",
            "  required_version = \">= 1.9\"\n",
            "}\n",
        );
        assert_eq!(
            extract_required_version(content),
            Some(">= 1.9".to_string())
        );
    }

    /// HCL identifiers are Unicode, so `<<終端` opens a heredoc like any other.
    /// Pre-fix the ASCII-only ident test rejected the opener, the body was
    /// read as structure, and its `{` / `}` / `#` unbalanced the file.
    #[test]
    fn extract_required_version_after_a_unicode_heredoc_terminator() {
        let content = concat!(
            "locals {\n",
            "  script = <<終端\n",
            "# not an HCL comment\n",
            "if [ -f x ]; then }\n",
            "終端\n",
            "}\n",
            "terraform {\n",
            "  required_version = \">= 1.7\"\n",
            "}\n",
        );
        assert_eq!(
            extract_required_version(content),
            Some(">= 1.7".to_string())
        );
    }

    /// An *indented* line equal to the terminator does not close a plain `<<`
    /// heredoc — only `<<-` permits that. Pre-fix both spellings matched on a
    /// trimmed line, so this body ended four lines early and the shell `}`
    /// after it popped a block the file never opened.
    #[test]
    fn a_plain_heredoc_is_not_closed_by_an_indented_terminator() {
        let content = concat!(
            "locals {\n",
            "  script = <<EOT\n",
            "  EOT\n",
            "if [ -f x ]; then }\n",
            "EOT\n",
            "}\n",
            "terraform {\n",
            "  required_version = \">= 1.8\"\n",
            "}\n",
        );
        assert_eq!(
            extract_required_version(content),
            Some(">= 1.8".to_string())
        );
    }

    /// The `<<-` half of the same rule still accepts the indented terminator
    /// it exists for.
    #[test]
    fn an_indented_heredoc_is_closed_by_an_indented_terminator() {
        let content = concat!(
            "locals {\n",
            "  script = <<-EOT\n",
            "    echo hi }\n",
            "  EOT\n",
            "}\n",
            "terraform {\n",
            "  required_version = \">= 1.9\"\n",
            "}\n",
        );
        assert_eq!(
            extract_required_version(content),
            Some(">= 1.9".to_string())
        );
    }

    /// `strip_comments` applies the same spelling rule, so a `#` line after an
    /// indented look-alike terminator inside a plain heredoc stays verbatim.
    #[test]
    fn strip_comments_keeps_a_plain_heredoc_open_past_an_indented_terminator() {
        let content = concat!(
            "x = <<EOT\n",
            "  EOT\n",
            "# still inside the body\n",
            "EOT\n",
        );
        let stripped = strip_comments(content);
        assert!(
            stripped.contains("# still inside the body"),
            "body must survive intact; got: {stripped}"
        );
    }

    /// PATTERN-1 / TASK-2031: `a < <b` and other non-openers must not put the
    /// scanner into heredoc state — that would swallow the rest of the file.
    #[test]
    fn extract_required_version_after_a_non_heredoc_less_than() {
        let content = concat!(
            "locals {\n",
            "  cmp = 1 < 2\n",
            "}\n",
            "terraform {\n",
            "  required_version = \">= 1.4\"\n",
            "}\n",
        );
        assert_eq!(
            extract_required_version(content),
            Some(">= 1.4".to_string())
        );
    }

    /// PATTERN-1 / TASK-1768: a trailing `#` comment on the block opener is
    /// ordinary human-written HCL. Pre-fix `terraform` was never pushed
    /// because the line did not *end* with `{`, and the whole file went dark.
    #[test]
    fn extract_required_version_with_commented_block_opener() {
        let content =
            "terraform { # pinned for the shared modules\n  required_version = \">= 1.5\"\n}\n";
        assert_eq!(
            extract_required_version(content),
            Some(">= 1.5".to_string())
        );
    }

    /// PATTERN-1 / TASK-1768: the `//` spelling of the same shape.
    #[test]
    fn extract_required_version_with_slash_commented_block_opener() {
        let content = "terraform { // pinned\n  required_version = \"~> 1.5\"\n}\n";
        assert_eq!(
            extract_required_version(content),
            Some("~> 1.5".to_string())
        );
    }

    /// PATTERN-1 / TASK-1768: a closing brace on the same line as the value.
    #[test]
    fn extract_required_version_with_same_line_closing_brace() {
        let content = "terraform {\n  required_version = \">= 1.5\" }\n";
        assert_eq!(
            extract_required_version(content),
            Some(">= 1.5".to_string())
        );
    }

    /// PATTERN-1 / TASK-1771: a `/*` inside a `#` line comment — a URL with a
    /// glob is enough — must not open a block comment. Pre-fix it blanked the
    /// remainder of the file and the constraint disappeared.
    #[test]
    fn extract_required_version_ignores_block_marker_inside_line_comment() {
        let content =
            "terraform {\n  # see https://example.com/*note\n  required_version = \"~> 1.5\"\n}\n";
        assert_eq!(
            extract_required_version(content),
            Some("~> 1.5".to_string())
        );
    }

    /// PATTERN-1 / TASK-1771: an unbalanced `"` inside a line comment must not
    /// put the scanner into string state for the rest of the file — pre-fix a
    /// subsequent `/* … */` was then no longer stripped.
    #[test]
    fn extract_required_version_ignores_unbalanced_quote_in_line_comment() {
        let content = concat!(
            "terraform {\n",
            "  # don't use \"old style\n",
            "  /* required_version = \">= 99.0\" */\n",
            "  required_version = \"~> 1.5\"\n",
            "}\n"
        );
        assert_eq!(
            extract_required_version(content),
            Some("~> 1.5".to_string())
        );
    }

    /// SEC-11 / TASK-1775: an ANSI escape sequence well under the 64-char cap
    /// must never reach `stack_detail` — `ops about` runs inside repositories
    /// the operator cloned but did not audit, and nothing between the `.tf`
    /// file and stdout escapes it. Drop, do not strip.
    #[test]
    fn extract_required_version_drops_value_with_ansi_escape() {
        let content =
            "terraform {\n  required_version = \"1.0\u{1b}[2J\u{1b}[31mCOMPROMISED\"\n}\n";
        assert!(content.len() < 128, "the payload fits under the cap");
        assert_eq!(extract_required_version(content), None);
    }

    /// SEC-11 / TASK-1775: carriage return and BEL are control bytes too.
    #[test]
    fn extract_required_version_drops_value_with_cr_and_bel() {
        assert_eq!(
            extract_required_version("terraform {\n  required_version = \"1.0\rfake\u{7}\"\n}\n"),
            None
        );
    }

    /// SEC-11 / TASK-1775: the sanitiser must not reject ordinary constraints.
    #[test]
    fn extract_required_version_keeps_ordinary_constraint() {
        assert_eq!(
            extract_required_version("terraform {\n  required_version = \">= 1.0, < 2.0\"\n}\n"),
            Some(">= 1.0, < 2.0".to_string())
        );
    }

    /// PERF-3 / TASK-1782: the no-comment fast path must borrow rather than
    /// allocate a second full copy of every `.tf` file read.
    #[test]
    fn strip_comments_borrows_when_no_comment_present() {
        let content = "terraform {\n  required_version = \">= 1.5\"\n}\n";
        assert!(matches!(strip_comments(content), Cow::Borrowed(_)));
        assert!(matches!(
            strip_comments("terraform { # note\n}\n"),
            Cow::Owned(_)
        ));
    }

    /// PERF-3 / TASK-1782: the fallback walk must not re-read a file the
    /// named-candidate loop already opened.
    #[test]
    fn fallback_tf_paths_skips_named_candidates() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["main.tf", "versions.tf", "terraform.tf", "version.tf"] {
            write(&dir.path().join(name), "");
        }
        write(&dir.path().join("extra.tf"), "");
        let paths = fallback_tf_paths(dir.path());
        assert_eq!(paths, vec![dir.path().join("extra.tf")]);
    }

    /// ERR-1 / TASK-1018: a missing `modules/` dir is the expected "no
    /// local modules" case and returns None silently.
    #[test]
    fn count_local_modules_missing_dir_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(count_local_modules(dir.path()), None);
    }

    /// ERR-1 / TASK-1018: counts only subdirectories that hold terraform
    /// sources. Empty dirs and stray files are ignored.
    #[test]
    fn count_local_modules_counts_module_dirs() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("modules").join("a").join("main.tf"), "");
        write(&dir.path().join("modules").join("b").join("main.tf"), "");
        std::fs::create_dir_all(dir.path().join("modules").join("empty")).unwrap();
        write(&dir.path().join("modules").join("stray.txt"), "x");
        assert_eq!(count_local_modules(dir.path()), Some(2));
    }

    /// PATTERN-1 / TASK-1796: `main.tf` is a terraform convention, not a
    /// requirement — any `.tf` file makes the directory a module. Pre-fix
    /// these layouts rendered no `modules` line at all.
    #[test]
    fn count_local_modules_counts_modules_without_main_tf() {
        let dir = tempfile::tempdir().unwrap();
        let modules = dir.path().join("modules");
        write(&modules.join("network").join("network.tf"), "");
        write(&modules.join("vpc").join("variables.tf"), "");
        write(&modules.join("vpc").join("outputs.tf"), "");
        write(&modules.join("json").join("main.tf.json"), "{}");
        // Case-insensitive, matching find_required_version's `.TF` handling.
        write(&modules.join("shouty").join("MAIN.TF"), "");
        // Not a module: no terraform sources at all.
        write(&modules.join("docs").join("README.md"), "");
        std::fs::create_dir_all(modules.join("empty")).unwrap();
        assert_eq!(count_local_modules(dir.path()), Some(4));
    }

    /// Vendoring a shared module once and symlinking it into each stack's
    /// `modules/` is a normal terraform layout. `DirEntry::file_type` does
    /// not follow symlinks, so such an entry reported `Symlink` and was
    /// dropped from the count; `fs::metadata` resolves it. A dangling link
    /// resolves to `NotFound` and is still not a module.
    #[cfg(unix)]
    #[test]
    fn count_local_modules_follows_symlinked_module_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let modules = dir.path().join("modules");
        write(&modules.join("real").join("main.tf"), "");

        let vendored = dir.path().join("vendored").join("shared");
        write(&vendored.join("main.tf"), "");
        std::os::unix::fs::symlink(&vendored, modules.join("linked")).unwrap();

        // A symlink to a directory with no terraform sources is followed and
        // then rejected on its contents, not on its link-ness.
        let empty = dir.path().join("vendored").join("no-sources");
        std::fs::create_dir_all(&empty).unwrap();
        std::os::unix::fs::symlink(&empty, modules.join("linked-empty")).unwrap();

        std::os::unix::fs::symlink(
            dir.path().join("vendored").join("missing"),
            modules.join("dangling"),
        )
        .unwrap();

        assert_eq!(count_local_modules(dir.path()), Some(2));
    }
}
