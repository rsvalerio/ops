//! Shared `go.mod` parser used by identity and modules providers.
//!
//! Produces module path, Go toolchain version, and the list of local
//! `replace` targets (any filesystem path: `./sub`, `../shared`, `/abs`, or a
//! Windows-style `C:\path`). Single-line and block-form `replace ( ... )`
//! directives are both recognized. Trailing `// ...` comments are stripped
//! from each line before further parsing.

use std::path::Path;

use crate::go_syntax::{
    has_embedded_parent_dir_segment, is_block_opener, is_block_terminator, strip_line_comment,
    strip_verb, unquote_token,
};

/// Which block-form directive is currently open.
///
/// PATTERN-1 (TASK-1727): modfile parses *every* verb in block form, not just
/// `replace`. Before this, `module (` fell through to the prefix matcher and
/// set the module name to the literal `"("`.
#[derive(Clone, Copy)]
enum Block {
    Replace,
    Module,
    Go,
}

#[derive(Debug, Default)]
pub struct GoMod {
    pub(crate) module: Option<String>,
    pub(crate) go_version: Option<String>,
    pub(crate) local_replaces: Vec<String>,
}

pub fn parse(dir: &Path) -> Option<GoMod> {
    let path = dir.join("go.mod");
    let content = ops_about::manifest_io::read_optional_text(&path, "go.mod")?;

    let mut out = GoMod::default();
    let mut block: Option<Block> = None;

    for raw in content.lines() {
        let line = strip_line_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(open) = block {
            if is_block_terminator(line) {
                block = None;
                continue;
            }
            match open {
                Block::Replace => {
                    if let Some(target) = parse_replace_directive(line) {
                        out.local_replaces.push(target);
                    }
                }
                // A block-form `module (` / `go (` holds a single entry; keep
                // the first, matching cmd/go's "only one such directive".
                Block::Module => set_module(&mut out, line),
                Block::Go => set_go_version(&mut out, line),
            }
            continue;
        }
        // Block openers must be tested before the verb matcher: `module (`
        // otherwise parses as the module path `(`.
        if is_block_opener(line, "replace") {
            block = Some(Block::Replace);
        } else if is_block_opener(line, "module") {
            block = Some(Block::Module);
        } else if is_block_opener(line, "go") {
            block = Some(Block::Go);
        } else if let Some(rest) = strip_verb(line, "module") {
            set_module(&mut out, rest);
        } else if let Some(rest) = strip_verb(line, "go") {
            set_go_version(&mut out, rest);
        } else if let Some(rest) = strip_verb(line, "replace") {
            if let Some(target) = parse_replace_directive(rest) {
                out.local_replaces.push(target);
            }
        }
    }

    Some(out)
}

/// ERR-2 / TASK-1167: a `module ""` or `module    ` line must drop to None so
/// the directory-name fallback in `lib.rs` fires, matching the `trim_nonempty`
/// policy applied by the Node and Python identity providers.
fn set_module(out: &mut GoMod, rest: &str) {
    if out.module.is_some() {
        return;
    }
    let value = unquote_token(rest.trim());
    if !value.is_empty() {
        out.module = Some(value.into_owned());
    }
}

fn set_go_version(out: &mut GoMod, rest: &str) {
    if out.go_version.is_some() {
        return;
    }
    let value = unquote_token(rest.trim());
    if !value.is_empty() {
        out.go_version = Some(value.into_owned());
    }
}

fn parse_replace_directive(rest: &str) -> Option<String> {
    let (_, target) = rest.split_once("=>")?;
    // PATTERN-1 (TASK-1727): cmd/go *requires* quoting for a target containing
    // a space, and a quoted target starts with `"` — so none of the `./`,
    // `../`, `/` prefix arms below matched and the local replace was dropped.
    let target = unquote_token(target.trim());
    let target = target.as_ref();
    if target.is_empty() {
        return None;
    }
    // cmd/go requires the replacement to omit a version when the target is a
    // filesystem path; anything carrying a whitespace-separated `vX.Y.Z` is a
    // remote module replacement.
    //
    // PATTERN-1 / TASK-0815: only the version-shaped second token marks a
    // remote replace — a path containing whitespace (legal on disk) such as
    // `./has space/sub` must still be recognised as a local target.
    let mut tokens = target.split_whitespace();
    if let (Some(_first), Some(second)) = (tokens.next(), tokens.next()) {
        if looks_like_module_version(second) {
            return None;
        }
    }
    if target.starts_with("./")
        || target.starts_with("../")
        || target.starts_with(".\\")
        || target.starts_with("..\\")
        || target.starts_with('/')
        || is_windows_absolute(target)
    {
        // PATTERN-1 (TASK-1212): reject embedded `..` cancellation segments
        // past the leading `./` / `../` prefix the matcher already accepts, so
        // adversarial fixtures cannot smuggle traversal through a local-replace
        // target. SEC-14 (TASK-1721): the predicate lives in `go_syntax` and is
        // shared with the `go.work` `use` directive path, which enforced only
        // the *first* component and so let `./api/../../../etc` through.
        if has_embedded_parent_dir_segment(target) {
            tracing::warn!(
                target = %target,
                "go.mod replace target contains embedded `..` traversal segment past the leading prefix; skipping"
            );
            return None;
        }
        return Some(target.to_string());
    }
    None
}

/// Match cmd/go's module version token shape: a leading `v` followed by an
/// `X.Y(.Z)?` numeric prefix. PATTERN-1 / TASK-0976: the previous
/// `v<digit> + contains('.')` heuristic accepted any non-numeric trailing
/// junk (`v1.foo.com/path`, `v9.local`, `v0.x`), so a local replace target
/// whose second whitespace token happened to begin with `v<digit>.` was
/// silently misclassified as a remote replace and dropped from
/// `local_replaces`.
///
/// Now require `v<MAJOR>.<MINOR>` with all-digit components (and an optional
/// `.<PATCH>` plus arbitrary pseudo-version / pre-release suffix after the
/// numeric prefix). This is still loose enough to accept everything cmd/go
/// emits while rejecting "looks vaguely like vX.Y" path tokens.
fn looks_like_module_version(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('v') else {
        return false;
    };
    // Require at least MAJOR.MINOR with both components all-digit.
    let mut parts = rest.splitn(3, '.');
    let major = parts.next().unwrap_or("");
    let minor = parts.next().unwrap_or("");
    if major.is_empty() || !major.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    if minor.is_empty() || !minor.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    // PATCH (and anything after) is optional and free-form: cmd/go pseudo-
    // versions like `v0.0.0-20240101000000-abcdef` need the pre-release tail
    // to flow through. We only require the numeric MAJOR.MINOR prefix.
    true
}

const fn is_windows_absolute(s: &str) -> bool {
    matches!(
        s.as_bytes(),
        [drive, b':', b'\\' | b'/', ..] if drive.is_ascii_alphabetic()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_module_and_go_version() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\ngo 1.22\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/m"));
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
    }

    #[test]
    fn strips_trailing_comments() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m // toolchain note\n\ngo 1.22 // toolchain hint\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/m"));
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
    }

    #[test]
    fn parses_block_form_replace_directives() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\ngo 1.22\n\nreplace (\n\texample.com/m/api => ./api\n\texample.com/m/sdk => ./sdk\n\texample.com/m/x => github.com/fork/x v1.0.0\n)\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["./api", "./sdk"]);
    }

    #[test]
    fn accepts_windows_style_backslash_replace_target() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace example.com/m/api => .\\api\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec![".\\api"]);
    }

    #[test]
    fn parses_single_line_replace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace example.com/m/api => ./api\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["./api"]);
    }

    /// TASK-0994: a trailing comment on a `replace (` block opener must not
    /// suppress the block.
    #[test]
    fn replace_block_opener_accepts_trailing_comment() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace ( // local fork pins\n\texample.com/m/api => ./api\n\texample.com/m/sdk => ./sdk\n)\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["./api", "./sdk"]);
    }

    #[test]
    fn replace_block_opener_accepts_no_space_before_paren() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace(\n\texample.com/m/api => ./api\n)\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["./api"]);
    }

    #[test]
    fn missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(parse(dir.path()).is_none());
    }

    #[test]
    fn no_go_version_yields_none_field() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module example.com/foo\n").unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/foo"));
        assert!(m.go_version.is_none());
    }

    #[test]
    fn accepts_parent_relative_replace_target() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace example.com/m/shared => ../shared\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["../shared"]);
    }

    #[test]
    fn accepts_absolute_replace_target() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace example.com/m/shared => /abs/path\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["/abs/path"]);
    }

    #[test]
    fn accepts_local_replace_target_with_whitespace() {
        // PATTERN-1 / TASK-0815: `./has space/sub` is a legal filesystem path
        // and must be retained as a local replace target.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace ex.com/m => ./has space/sub\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["./has space/sub"]);
    }

    /// PATTERN-1 / TASK-0976: a local target whose second whitespace token
    /// happens to start `v<digit>.` but is not a valid semver must NOT be
    /// dropped from `local_replaces`. The previous lax heuristic treated
    /// `./root v1.snapshot` as a remote replace and silently lost the
    /// member from the workspace size in the About card.
    #[test]
    fn keeps_local_replace_with_pseudo_version_token() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace ex.com/m => ./root v1.snapshot\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["./root v1.snapshot"]);
    }

    /// PATTERN-1 / TASK-0976: the strict matcher unit-level — only the
    /// `vMAJOR.MINOR(.PATCH)?` numeric prefix qualifies. Non-numeric trailing
    /// junk (`v1.foo`, `v9.local`, `v0.x`) no longer false-matches.
    #[test]
    fn looks_like_module_version_requires_numeric_minor() {
        assert!(looks_like_module_version("v1.2.3"));
        assert!(looks_like_module_version("v1.2"));
        assert!(looks_like_module_version("v0.0.0-20240101000000-abcdef"));
        assert!(!looks_like_module_version("v1.foo"));
        assert!(!looks_like_module_version("v9.local"));
        assert!(!looks_like_module_version("v0.x"));
        assert!(!looks_like_module_version("v.1.2"));
        assert!(!looks_like_module_version("v1"));
        assert!(!looks_like_module_version("v"));
    }

    #[test]
    fn ignores_remote_replaces() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/foo\n\ngo 1.21\n\nreplace example.com/bar => github.com/fork/bar v1.2.3\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert!(m.local_replaces.is_empty());
    }

    #[test]
    fn whitespace_handling() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "  module   example.com/ws  \n\n  go   1.23  \n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/ws"));
        assert_eq!(m.go_version.as_deref(), Some("1.23"));
    }

    #[test]
    fn empty_file_yields_empty_struct() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "").unwrap();
        let m = parse(dir.path()).unwrap();
        assert!(m.module.is_none());
        assert!(m.go_version.is_none());
        assert!(m.local_replaces.is_empty());
    }

    #[test]
    fn no_module_line_yields_none_module() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "go 1.21\n").unwrap();
        let m = parse(dir.path()).unwrap();
        assert!(m.module.is_none());
        assert_eq!(m.go_version.as_deref(), Some("1.21"));
    }

    /// PATTERN-1 / TASK-1107: `//` is a comment delimiter only when it
    /// follows whitespace or starts the line. A module path containing a
    /// literal `//` must not be silently truncated.
    #[test]
    fn module_path_with_literal_double_slash_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module example.com/foo//bar\n").unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/foo//bar"));
    }

    /// PATTERN-1 (TASK-1255): `replace(// note` (no whitespace before the
    /// inline comment) is legal go.mod syntax cmd/go accepts. Both the
    /// `replace` block opener and its contained directives must surface.
    #[test]
    fn replace_block_with_inline_comment_no_whitespace_populates_list() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace(//local-pins\n\texample.com/m/api => ./api\n\texample.com/m/sdk => ./sdk\n)\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["./api", "./sdk"]);
    }

    /// PATTERN-1 (TASK-1212): replace targets carrying embedded `..` segments
    /// past the leading prefix (e.g. `./foo/../../etc`) are dropped from
    /// `local_replaces` rather than flowing through verbatim.
    #[test]
    fn replace_target_with_embedded_parent_dir_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace example.com/m/api => ./foo/../../etc/passwd\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert!(
            m.local_replaces.is_empty(),
            "expected scrubbed/skipped, got {:?}",
            m.local_replaces
        );
    }

    /// PATTERN-1 (TASK-1212): a leading run of `..` segments is allowed
    /// (cmd/go accepts `../../shared`); only `..` past a real path segment
    /// is rejected. Pin both behaviours together.
    #[test]
    fn replace_target_leading_parent_dirs_still_accepted() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace example.com/m/x => ../../shared/lib\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["../../shared/lib"]);
    }

    /// PATTERN-1 (TASK-1212): the scrub also affects `compute_module_count`
    /// — a sole adversarial replace yields 0 `local_replaces`, so the
    /// single-module count returns None rather than `Some(2)`.
    #[test]
    fn compute_module_count_does_not_double_count_scrubbed_replace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace example.com/m/api => ./foo/../../etc/passwd\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        // Single bare go.mod with no surviving local replaces ⇒ count is None.
        assert!(m.local_replaces.is_empty());
        // DUP-1 (TASK-1731): `lib.rs` consumes `go_mod::GoMod` directly, so
        // the test no longer hand-constructs a second identical struct.
        assert_eq!(crate::compute_module_count(None, Some(&m)), None);
    }

    /// PATTERN-1 (TASK-1727): modfile lexes Go string literals, and quoting is
    /// required for any token containing a space. Left quoted, the module name
    /// rendered as `m"` on the About card.
    #[test]
    fn parses_quoted_module_and_replace_target() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module \"example.com/m\"\n\ngo \"1.22\"\n\nreplace ex.com/m => \"./has space/sub\"\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/m"));
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
        assert_eq!(m.local_replaces, vec!["./has space/sub"]);
        // The About-card name derives from the unquoted path.
        assert_eq!(
            crate::modules::last_segment(m.module.as_deref()).as_deref(),
            Some("m")
        );
    }

    /// PATTERN-1 (TASK-1727): modfile splits verb from argument on arbitrary
    /// whitespace; `strip_prefix("module ")` dropped every tab-separated form
    /// silently, so the module name fell back to the directory and the Go
    /// version vanished from the card.
    #[test]
    fn parses_tab_separated_directives() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module\texample.com/m\n\ngo\t1.22\n\nreplace\tex.com/m => ./api\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/m"));
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
        assert_eq!(m.local_replaces, vec!["./api"]);
    }

    /// PATTERN-1 (TASK-1727): every verb has a block form. `module (` used to
    /// fall through to the prefix matcher and set the module to the literal
    /// `"("`, so the About card was titled `(`.
    #[test]
    fn parses_block_form_module_and_go_directives() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module (\n\texample.com/m\n)\n\ngo (\n\t1.22\n)\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/m"));
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
    }

    /// PATTERN-1 (TASK-1724): a `)` terminator carrying a trailing comment
    /// must close the block in go.mod too, in either spacing.
    #[test]
    fn replace_block_terminator_accepts_trailing_comment() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace (\n\tex.com/a => ./api\n) // local pins\n\ngo 1.22\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["./api"]);
        // The block closed, so the trailing `go` line was still parsed.
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
    }

    #[test]
    fn multiple_single_line_local_replaces() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/openbao/openbao\n\ngo 1.25.7\n\nreplace github.com/openbao/openbao/api/v2 => ./api\n\nreplace github.com/openbao/openbao/sdk/v2 => ./sdk\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["./api", "./sdk"]);
    }
}
