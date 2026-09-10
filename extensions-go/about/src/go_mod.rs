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
/// modfile gives *every* verb a block form, not just `replace`: `module (`,
/// `go (` and `replace (` all open a block, and the parser must recognise
/// the opener before the verb matcher so `module (` is not read as the
/// module path `(`.
#[derive(Clone, Copy)]
enum Block {
    Replace,
    Module,
    Go,
}

/// Parsed contents of a `go.mod` file.
///
/// The fields are spelled `pub` to match the type: `mod go_mod` is private,
/// so the private module — not the field spelling — is the visibility
/// boundary, and `pub(crate)` inside it is what
/// `clippy::redundant_pub_crate` denies workspace-wide.
#[derive(Debug, Default)]
pub struct GoMod {
    /// Module path from the `module` directive; `None` when absent.
    pub module: Option<String>,
    /// Toolchain version from the `go` directive; `None` when absent.
    pub go_version: Option<String>,
    /// Filesystem targets of local `replace` directives, in file order.
    pub local_replaces: Vec<String>,
}

pub fn parse(dir: &Path) -> Option<GoMod> {
    let path = dir.join("go.mod");
    let content = ops_about::manifest_io::read_optional_text(&path, "go.mod")?;

    let mut out = GoMod::default();
    let mut block: Option<Block> = None;
    // Snapshot of the fields a block can mutate, taken when the block opens.
    // A block whose `)` never arrives is malformed and has no boundary
    // between entry and prose, so at EOF the values it absorbed are rolled
    // back to this snapshot and one warn is emitted.
    let mut block_snapshot: Option<(Option<String>, Option<String>, usize)> = None;

    for raw in content.lines() {
        let line = strip_line_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(open) = block {
            if is_block_terminator(line) {
                block = None;
                block_snapshot = None;
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
        let opener = if is_block_opener(line, "replace") {
            Some(Block::Replace)
        } else if is_block_opener(line, "module") {
            Some(Block::Module)
        } else if is_block_opener(line, "go") {
            Some(Block::Go)
        } else {
            None
        };
        if let Some(verb) = opener {
            // Record the pre-block state so an unterminated block can be
            // reported and its absorbed values rolled back at EOF.
            block_snapshot = Some((
                out.module.clone(),
                out.go_version.clone(),
                out.local_replaces.len(),
            ));
            block = Some(verb);
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

    // A block still open at EOF means the file is truncated or hand-mangled.
    // Report it once — naming the manifest and the unterminated directive —
    // and roll back the values the block absorbed, so an unterminated
    // `replace (` block neither half-trusts its own entries nor silently
    // swallows the directives that follow it.
    if let Some(open) = block {
        let directive = match open {
            Block::Replace => "replace",
            Block::Module => "module",
            Block::Go => "go",
        };
        tracing::warn!(
            manifest = "go.mod",
            directive = directive,
            "go.mod: unterminated block directive at end of file; dropping values absorbed by the block"
        );
        if let Some((module, go_version, replaces_len)) = block_snapshot {
            out.module = module;
            out.go_version = go_version;
            out.local_replaces.truncate(replaces_len);
        }
    }

    Some(out)
}

/// The directive-value policy the `module` and `go` setters share: the first
/// directive wins (cmd/go allows "only one such directive"), the token is
/// unquoted, and an empty value leaves the slot `None`.
///
/// Dropping an empty value is what lets a `module ""` or `module    ` line
/// fall through to the directory-name fallback in `lib.rs`, matching the
/// `trim_nonempty` policy the Node and Python identity providers apply.
fn set_first_wins_unquoted_nonempty(slot: &mut Option<String>, rest: &str) {
    if slot.is_some() {
        return;
    }
    let value = unquote_token(rest.trim());
    if !value.is_empty() {
        *slot = Some(value.into_owned());
    }
}

fn set_module(out: &mut GoMod, rest: &str) {
    set_first_wins_unquoted_nonempty(&mut out.module, rest);
}

fn set_go_version(out: &mut GoMod, rest: &str) {
    set_first_wins_unquoted_nonempty(&mut out.go_version, rest);
}

fn parse_replace_directive(rest: &str) -> Option<String> {
    let (_, target) = rest.split_once("=>")?;
    // cmd/go *requires* quoting for a target containing a space, so the
    // token is unquoted before the `./`, `../`, `/` prefix arms below look
    // at it — a quoted target otherwise starts with `"` and matches none.
    let target = unquote_token(target.trim());
    let target = target.as_ref();
    if target.is_empty() {
        return None;
    }
    // cmd/go requires the replacement to omit a version when the target is a
    // filesystem path; anything carrying a whitespace-separated `vX.Y.Z` is a
    // remote module replacement. Only a *version-shaped* second token marks
    // the replace as remote, so a path containing whitespace (legal on disk)
    // such as `./has space/sub` still counts as a local target.
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
        // Reject embedded `..` segments past the leading `./` / `../` prefix
        // the arms above accept, so a fixture cannot smuggle traversal
        // through a local-replace target. The predicate lives in `go_syntax`
        // and is shared with the `go.work` `use` directive path so both
        // directives enforce one traversal policy.
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

/// Match cmd/go's module version token shape: `v<MAJOR>.<MINOR>` with
/// all-digit components, followed optionally by `.<PATCH>` and an arbitrary
/// pseudo-version / pre-release tail.
///
/// The numeric `MAJOR.MINOR` prefix is the whole requirement: loose enough to
/// accept everything cmd/go emits (`v1.2`, `v1.2.3`,
/// `v0.0.0-20240101000000-abcdef`), strict enough that a path token merely
/// shaped like a version (`v1.foo`, `v9.local`, `v0.x`) is not mistaken for
/// one and does not turn a local replace into a remote one.
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
    // PATCH and anything after it are optional and free-form, so cmd/go
    // pseudo-versions like `v0.0.0-20240101000000-abcdef` match.
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

    /// A trailing comment on a `replace (` block opener still opens the
    /// block and its entries are collected.
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
        // `./has space/sub` is a legal filesystem path, so it stays a local
        // replace target despite the embedded whitespace.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace ex.com/m => ./has space/sub\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["./has space/sub"]);
    }

    /// A local target whose second whitespace token merely starts `v<digit>.`
    /// without being a version (`./root v1.snapshot`) stays in
    /// `local_replaces`.
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

    /// Only a `vMAJOR.MINOR(.PATCH)?` numeric prefix qualifies as a module
    /// version; non-numeric components (`v1.foo`, `v9.local`, `v0.x`) do not.
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

    /// A module path containing a literal `//` is preserved whole: `//`
    /// delimits a comment only at start-of-line or after whitespace.
    #[test]
    fn module_path_with_literal_double_slash_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module example.com/foo//bar\n").unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/foo//bar"));
    }

    /// `replace(// note` — no whitespace before the inline comment — opens
    /// the block, and the directives inside it are collected.
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

    /// A replace target carrying an embedded `..` segment past the leading
    /// prefix (`./foo/../../etc`) is dropped from `local_replaces`.
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

    /// A leading run of `..` segments is accepted (cmd/go allows
    /// `../../shared`); only `..` past a real path segment is rejected.
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

    /// A traversal-carrying replace target is scrubbed at the parse level,
    /// and a `go.mod`-only project reports no module count whatever its
    /// replaces — the two rules hold together.
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
        assert_eq!(crate::compute_module_count(None), None);
    }

    /// Quoted `module`, `go` and `replace` tokens are unquoted before use, so
    /// the About-card name derives from the bare module path.
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

    /// Verb and argument are separated by arbitrary whitespace, so
    /// tab-separated `module`, `go` and `replace` directives parse.
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

    /// Every verb has a block form: `module (` and `go (` yield the entry
    /// inside the block, not the literal `(`.
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

    /// A `)` terminator carrying a trailing comment closes the block, so the
    /// directives after it are still parsed.
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
        // The block closed, so the trailing `go` line is a top-level
        // directive and parses.
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

    /// An unterminated `replace (` block absorbs every following line, so
    /// its payload is rolled back: exactly one warn fires, the block's own
    /// entries and the lines it swallowed are dropped, and the directives
    /// before the block survive.
    #[test]
    fn unterminated_replace_block_warns_once_and_rolls_back_absorbed_values() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace ex.com/a => ./api\n\nreplace (\n\tex.com/sdk => ./sdk\ngo 1.22\n",
        )
        .unwrap();

        let (m, warn_count) = ops_about::test_support::count_warnings(|| parse(dir.path()));

        let m = m.unwrap();
        // The single-line replace before the block survives; the block's own
        // absorbed entry does not.
        assert_eq!(m.local_replaces, vec!["./api"]);
        // The `go 1.22` line swallowed by the block is gone with it.
        assert!(m.go_version.is_none());
        assert_eq!(warn_count, 1);
    }

    /// The rendered diagnostic names the manifest and the unterminated
    /// directive.
    #[test]
    fn unterminated_replace_block_warn_names_manifest_and_directive() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace (\n\tex.com/sdk => ./sdk\n",
        )
        .unwrap();
        let rendered = ops_about::test_support::capture_warn(|| {
            parse(dir.path());
        });
        assert!(
            rendered.contains("go.mod"),
            "warn should name the manifest: {rendered}"
        );
        assert!(
            rendered.contains("unterminated"),
            "warn should say what is wrong: {rendered}"
        );
        assert!(
            rendered.contains("replace"),
            "warn should name the directive: {rendered}"
        );
    }
}
