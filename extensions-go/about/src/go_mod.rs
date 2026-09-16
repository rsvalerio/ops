//! Shared `go.mod` parser used by identity and modules providers.
//!
//! Produces module path and Go toolchain version. `replace` directives —
//! single-line and block-form `replace ( ... )` — are recognized and skipped:
//! a replace is a dependency substitution, not a workspace member, and no
//! card reads its targets (TASK-2178 dropped replaces from `module_count`;
//! TASK-2254 removed the then-reader-less target collection). Trailing
//! `// ...` comments are stripped from each line before further parsing.

use std::path::Path;

use crate::go_syntax::{
    is_block_opener, is_block_terminator, strip_line_comment, strip_verb, unquote_token,
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
    let mut block_snapshot: Option<(Option<String>, Option<String>)> = None;

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
                // Replace entries carry no data any consumer reads; the
                // block is tracked only so its lines are consumed (and an
                // unterminated block still warns) instead of leaking into
                // the top-level verb matcher.
                Block::Replace => {}
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
            block_snapshot = Some((out.module.clone(), out.go_version.clone()));
            block = Some(verb);
        } else if let Some(rest) = strip_verb(line, "module") {
            set_module(&mut out, rest);
        } else if let Some(rest) = strip_verb(line, "go") {
            set_go_version(&mut out, rest);
        } else if strip_verb(line, "replace").is_some() {
            // Skipped: a replace is a dependency substitution, not a
            // workspace member, and nothing reads its target.
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
        if let Some((module, go_version)) = block_snapshot {
            out.module = module;
            out.go_version = go_version;
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

    /// Single-line and block-form `replace` directives are skipped without
    /// disturbing the directives the card does read.
    #[test]
    fn replace_directives_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\ngo 1.22\n\nreplace example.com/m/api => ./api\n\nreplace (\n\texample.com/m/sdk => ./sdk\n\texample.com/m/x => github.com/fork/x v1.0.0\n)\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/m"));
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
    }

    /// A trailing comment on a `replace (` block opener still opens the
    /// block, so the `go` directive after the terminator parses.
    #[test]
    fn replace_block_opener_accepts_trailing_comment() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace ( // local fork pins\n\texample.com/m/api => ./api\n)\n\ngo 1.22\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/m"));
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
    }

    #[test]
    fn replace_block_opener_accepts_no_space_before_paren() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace(\n\texample.com/m/api => ./api\n)\n\ngo 1.22\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/m"));
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
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

    /// `replace(// note` — no whitespace before the inline comment — still
    /// opens the block, so its entries are consumed rather than parsed as
    /// top-level directives.
    #[test]
    fn replace_block_with_inline_comment_no_whitespace_is_skipped() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace(//local-pins\n\texample.com/m/api => ./api\n)\n\ngo 1.22\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/m"));
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
    }

    /// Quoted `module` and `go` tokens are unquoted before use, so the
    /// About-card name derives from the bare module path.
    #[test]
    fn parses_quoted_module_directives() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module \"example.com/m\"\n\ngo \"1.22\"\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/m"));
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
        // The About-card name derives from the unquoted path.
        assert_eq!(
            crate::modules::last_segment(m.module.as_deref()).as_deref(),
            Some("m")
        );
    }

    /// Verb and argument are separated by arbitrary whitespace, so
    /// tab-separated `module` and `go` directives parse.
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
        // The block closed, so the trailing `go` line is a top-level
        // directive and parses.
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
    }

    /// An unterminated `replace (` block absorbs every following line, so
    /// its payload is rolled back: exactly one warn fires, the lines it
    /// swallowed are dropped, and the directives before the block survive.
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
        // The directive before the block survives.
        assert_eq!(m.module.as_deref(), Some("example.com/m"));
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
