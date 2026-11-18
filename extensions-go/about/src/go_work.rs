//! Shared `go.work` parser used by identity and modules providers.
//!
//! Returns the list of `use` directives (block or single-line form) verbatim
//! from the file. Comment-only and empty lines are skipped. Returns `None` if
//! the file is missing or no `use` entries are found.
//!
//! A `go.work` takes precedence over the root `go.mod` throughout the crate:
//! when this returns `Some`, its directives are the project's modules and
//! the root `go.mod` supplies neither the unit list nor the module count.
//!
//! Nested `use(` openers inside an already-open block are not legal go.work
//! syntax — cmd/go rejects them — so a fresh `use(` / `use (` line inside an
//! open block is logged at `tracing::warn!` and dropped rather than taken as
//! a directory named `use(`.

use std::path::Path;

use crate::go_syntax::{
    is_block_opener, is_block_terminator, strip_line_comment, strip_verb, unquote_token,
};

/// Parse the `go.work` in `root`, returning its `use` directive directories.
///
/// Returns `None` when the file is absent or unreadable, or when no `use`
/// entries are found; block and single-line forms are both accepted.
pub fn parse_use_dirs(root: &Path) -> Option<Vec<String>> {
    let path = root.join("go.work");
    let content = ops_about::manifest_io::read_optional_text(&path, root, "go.work")?;
    parse_use_dirs_content(&content, &path)
}

/// Parse `go.work` text for its `use` directive directories.
///
/// The pure grammar behind [`parse_use_dirs`]: `path` is used only to name
/// the manifest in diagnostics. Returns `None` when no `use` entries are
/// found.
fn parse_use_dirs_content(content: &str, path: &Path) -> Option<Vec<String>> {
    let mut dirs = Vec::new();
    // The 1-based line of the currently open block's opener, `None` outside
    // a block.
    let mut open_block_line: Option<usize> = None;
    // Where the currently open block started, as an index into `dirs`. A
    // block whose `)` never arrives has no boundary between directive and
    // manifest prose, so at EOF the list is truncated back to this mark and
    // one warn is emitted, rather than probing `cwd.join("go 1.22")`-shaped
    // paths and inflating the module count.
    let mut block_start_mark = 0;

    for (raw, line_number) in content.lines().zip(1_usize..) {
        // Normalise the comment away *before* any structural test, matching
        // the order `go_mod.rs` uses, so a commented opener, terminator or
        // directive is recognised as its bare form — and a comment-only line
        // becomes empty and is skipped here.
        let line = strip_line_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if open_block_line.is_some() {
            if is_block_terminator(line) {
                open_block_line = None;
                continue;
            }
            // A nested `use(` / `use (` opener is not a directory entry.
            // Skip it with a warn so a malformed go.work does not surface a
            // directive whose name is `use(`.
            if is_block_opener(line, "use") {
                tracing::warn!(
                    path = ?path,
                    line_number,
                    line = ?line,
                    "go.work: nested `use(` opener inside an open block; skipping (cmd/go rejects this shape)"
                );
                continue;
            }
            let dir = unquote_token(line);
            if !dir.is_empty() {
                dirs.push(dir.into_owned());
            }
        } else if is_block_opener(line, "use") {
            open_block_line = Some(line_number);
            block_start_mark = dirs.len();
        } else if let Some(rest) = strip_verb(line, "use") {
            let dir = unquote_token(rest);
            if !dir.is_empty() && !dir.starts_with('(') {
                dirs.push(dir.into_owned());
            }
        }
    }

    // A `use` block still open at EOF means the file is truncated or
    // hand-mangled. Report it once — naming the manifest and the opener's
    // line — and drop the entries the block absorbed, so manifest prose
    // (`go 1.22`, `replace …`) neither becomes a ProjectUnit nor triggers a
    // go.mod probe against `cwd.join(<arbitrary manifest text>)`.
    if let Some(opener_line) = open_block_line {
        tracing::warn!(
            manifest = "go.work",
            path = ?path,
            line_number = opener_line,
            directive = "use",
            "go.work: unterminated `use (` block at end of file; dropping directives absorbed by the block"
        );
        dirs.truncate(block_start_mark);
    }

    if dirs.is_empty() { None } else { Some(dirs) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run the pure grammar over `content`, as if read from a root `go.work`.
    fn parse_str(content: &str) -> Option<Vec<String>> {
        parse_use_dirs_content(content, Path::new("go.work"))
    }

    #[test]
    fn strips_inline_comment_in_use_block() {
        let content = "go 1.21\n\nuse (\n\t./api // legacy\n\t./cmd\n)\n";
        let dirs = parse_str(content).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    /// A trailing line comment on a `use (` block opener still opens the
    /// block, so the workspace members are collected.
    #[test]
    fn block_opener_accepts_trailing_comment_on_use() {
        let content = "go 1.21\n\nuse ( // ws-members\n\t./api\n\t./cmd\n)\n";
        let dirs = parse_str(content).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    #[test]
    fn block_opener_accepts_no_space_before_paren() {
        let content = "go 1.21\n\nuse(\n\t./api\n\t./cmd\n)\n";
        let dirs = parse_str(content).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    #[test]
    fn strips_inline_comment_in_single_line_use() {
        let content = "go 1.21\nuse ./mymod // note\n";
        let dirs = parse_str(content).unwrap();
        assert_eq!(dirs, vec!["./mymod"]);
    }

    #[test]
    fn parses_multi_use_block() {
        let content = "go 1.21\n\nuse (\n\t./api\n\t./cmd\n\t./sdk\n)\n";
        let dirs = parse_str(content).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd", "./sdk"]);
    }

    #[test]
    fn parses_single_use_line() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\nuse ./mymod\n").unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./mymod"]);
    }

    #[test]
    fn missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(parse_use_dirs(dir.path()).is_none());
    }

    #[test]
    fn empty_use_block_yields_none() {
        let content = "go 1.21\n\nuse (\n)\n";
        assert!(parse_str(content).is_none());
    }

    #[test]
    fn comments_only_in_use_block_yields_none() {
        let content = "go 1.21\n\nuse (\n\t// a comment\n\t./real\n\t// another\n)\n";
        let dirs = parse_str(content).unwrap();
        assert_eq!(dirs, vec!["./real"]);
    }

    #[test]
    fn empty_file_yields_none() {
        let content = "";
        assert!(parse_str(content).is_none());
    }

    #[test]
    fn blank_lines_in_use_block() {
        let content = "go 1.21\n\nuse (\n\n\t./a\n\n\t./b\n\n)\n";
        let dirs = parse_str(content).unwrap();
        assert_eq!(dirs, vec!["./a", "./b"]);
    }

    /// `use(// note` — no whitespace before the inline comment — opens the
    /// block, so the members inside it reach the use list.
    #[test]
    fn use_block_with_inline_comment_no_whitespace_populates_list() {
        let content = "go 1.21\n\nuse(//ws-members\n\t./api\n\t./cmd\n)\n";
        let dirs = parse_str(content).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    /// A nested `use (` opener inside an outer block is rejected with a warn
    /// rather than absorbed as a directory entry named `use (`; the real
    /// entries around it still resolve.
    #[test]
    fn parse_use_dirs_warns_on_nested_block_opener() {
        let content = "go 1.21\n\nuse (\n\t./api\n\tuse (\n\t./cmd\n)\n";
        let (dirs, warn_count) = ops_core::test_utils::count_warnings(|| parse_str(content));
        // The nested opener is dropped and the entries on both sides of it
        // still resolve.
        assert_eq!(dirs, Some(vec!["./api".to_string(), "./cmd".to_string()]));
        assert_eq!(warn_count, 1);
    }

    /// The nested-opener diagnostic names the go.work path and the line the
    /// stray opener sits on.
    #[test]
    fn nested_block_opener_warn_names_path_and_line() {
        let content = "go 1.21\n\nuse (\n\t./api\n\tuse (\n\t./cmd\n)\n";
        let rendered = ops_core::test_utils::capture_warn(|| {
            parse_use_dirs_content(content, Path::new("ws/go.work"));
        });
        assert!(rendered.contains("ws/go.work"), "warn should name the path: {rendered}");
        assert!(rendered.contains("line_number=5"), "warn should name the line: {rendered}");
    }

    /// A `)` terminator carrying a trailing comment closes the block, so the
    /// following top-level lines are not absorbed as use directives.
    #[test]
    fn block_terminator_with_trailing_comment_closes_the_block() {
        let content = "go 1.21\n\nuse (\n\t./api\n\t./cmd\n) // workspace members\n\ngo 1.22\n\nreplace (\n\tex.com/a => ./api\n)\n";
        let dirs = parse_str(content).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    /// The same holds for a tab-indented terminator and for a
    /// no-whitespace inline comment.
    #[test]
    fn block_terminator_variants_close_the_block() {
        for terminator in [")//members", "\t)", "\t) // members", ")\t// members"] {
            let content = format!("go 1.21\n\nuse (\n\t./api\n{terminator}\n\ngo 1.22\n");
            let dirs = parse_str(&content).unwrap();
            assert_eq!(dirs, vec!["./api"], "terminator {terminator:?}");
        }
    }

    /// Quoted and tab-separated `use` directives are legal modfile syntax;
    /// the directory reaches the list unquoted so it can match a
    /// `tokei_files` path.
    #[test]
    fn parses_quoted_and_tab_separated_use_directives() {
        let content = "go 1.21\n\nuse\t./tabbed\n\nuse (\n\t\"./api\"\n\t\"./has space/sub\"\n)\n";
        let dirs = parse_str(content).unwrap();
        assert_eq!(dirs, vec!["./tabbed", "./api", "./has space/sub"]);
    }

    #[test]
    fn multiple_single_line_uses() {
        let content = "go 1.21\nuse ./first\nuse ./second\n";
        let dirs = parse_str(content).unwrap();
        assert_eq!(dirs, vec!["./first", "./second"]);
    }

    /// A `use` block whose `)` never arrives does not absorb the rest of the
    /// file as directives: exactly one warn fires and every absorbed line —
    /// real-looking entries and manifest prose alike — is dropped, so no
    /// `cwd.join("go 1.22")`-shaped go.mod probe is issued downstream.
    #[test]
    fn unterminated_use_block_warns_once_and_drops_absorbed_directives() {
        let content = "go 1.21\n\nuse (\n\t./api\ngo 1.22\nreplace ex.com/a => ../b\n";

        let (dirs, warn_count) = ops_core::test_utils::count_warnings(|| parse_str(content));

        // The whole unterminated block — including the legitimate-looking
        // `./api` — is dropped rather than half-trusted: with no terminator
        // there is no boundary between entry and prose.
        assert_eq!(dirs, None);
        assert_eq!(warn_count, 1);
    }

    /// The rendered diagnostic names the manifest and the unterminated
    /// directive.
    #[test]
    fn unterminated_use_block_warn_names_manifest_and_directive() {
        let content = "go 1.21\n\nuse (\n\t./api\n";
        let rendered = ops_core::test_utils::capture_warn(|| {
            parse_use_dirs_content(content, Path::new("ws/go.work"));
        });
        assert!(rendered.contains("ws/go.work"), "warn should name the path: {rendered}");
        assert!(rendered.contains("line_number=3"), "warn should name the opener line: {rendered}");
        assert!(rendered.contains("unterminated"), "warn should say what is wrong: {rendered}");
        assert!(rendered.contains("use"), "warn should name the directive: {rendered}");
    }

    /// A closed block followed by an unterminated one keeps the closed
    /// block's entries — only the malformed block's payload is dropped.
    #[test]
    fn unterminated_block_after_closed_block_keeps_earlier_entries() {
        let content = "go 1.21\n\nuse (\n\t./api\n)\n\nuse (\n\t./sdk\ngo 1.22\n";

        let (dirs, warn_count) = ops_core::test_utils::count_warnings(|| parse_str(content));

        assert_eq!(dirs, Some(vec!["./api".to_string()]));
        assert_eq!(warn_count, 1);
    }
}
