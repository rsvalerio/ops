//! Shared `go.work` parser used by identity and modules providers.
//!
//! Returns the list of `use` directives (block or single-line form) verbatim
//! from the file. Comment-only and empty lines are skipped. Returns `None` if
//! the file is missing or no `use` entries are found.
//!
//! PATTERN-1 (TASK-1216): nested `use(` openers inside an already-open block
//! are not legal go.work syntax — cmd/go itself rejects them. The parser
//! mirrors that intent: when a `use` block is open and a fresh `use(` /
//! `use (` line appears, it is logged at `tracing::warn!` and dropped from
//! the directive list rather than being absorbed as a directory whose name
//! happens to be `use(`.

use std::path::Path;

use crate::go_syntax::{
    is_block_opener, is_block_terminator, strip_line_comment, strip_verb, unquote_token,
};

pub fn parse_use_dirs(root: &Path) -> Option<Vec<String>> {
    let path = root.join("go.work");
    let content = ops_about::manifest_io::read_optional_text(&path, "go.work")?;
    let mut dirs = Vec::new();
    let mut in_use_block = false;

    for raw in content.lines() {
        // PATTERN-1 (TASK-1724): normalise the comment away *before* any
        // structural test, matching the order go_mod.rs has always used. With
        // the raw line, a legal `) // workspace members` terminator matched
        // neither `")"` nor the comment arm, was pushed as a use directive
        // named `)`, and left the block open so every following top-level line
        // was absorbed too.
        let line = strip_line_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if is_block_opener(line, "use") && !in_use_block {
            in_use_block = true;
            continue;
        }
        if in_use_block {
            if is_block_terminator(line) {
                in_use_block = false;
                continue;
            }
            if line.starts_with("//") {
                continue;
            }
            // PATTERN-1 (TASK-1216): a nested `use(` / `use (` opener is not
            // a directory entry. Skip it with a warn so a malformed go.work
            // does not silently surface a directive whose name is `use(`.
            if is_block_opener(line, "use") {
                tracing::warn!(
                    line = ?line,
                    "go.work: nested `use(` opener inside an open block; skipping (cmd/go rejects this shape)"
                );
                continue;
            }
            let dir = unquote_token(line);
            if !dir.is_empty() {
                dirs.push(dir.into_owned());
            }
        } else if let Some(rest) = strip_verb(line, "use") {
            let dir = unquote_token(rest);
            if !dir.is_empty() && !dir.starts_with('(') {
                dirs.push(dir.into_owned());
            }
        }
    }

    if dirs.is_empty() {
        None
    } else {
        Some(dirs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_inline_comment_in_use_block() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api // legacy\n\t./cmd\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    /// TASK-0994: cmd/go accepts a trailing line comment on a `use (`
    /// block opener; the parser must too — otherwise the entire workspace
    /// reports as a single-mod project.
    #[test]
    fn block_opener_accepts_trailing_comment_on_use() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse ( // ws-members\n\t./api\n\t./cmd\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    #[test]
    fn block_opener_accepts_no_space_before_paren() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse(\n\t./api\n\t./cmd\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    #[test]
    fn strips_inline_comment_in_single_line_use() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\nuse ./mymod // note\n").unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./mymod"]);
    }

    #[test]
    fn parses_multi_use_block() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api\n\t./cmd\n\t./sdk\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
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
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\n\nuse (\n)\n").unwrap();
        assert!(parse_use_dirs(dir.path()).is_none());
    }

    #[test]
    fn comments_only_in_use_block_yields_none() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t// a comment\n\t./real\n\t// another\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./real"]);
    }

    #[test]
    fn empty_file_yields_none() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "").unwrap();
        assert!(parse_use_dirs(dir.path()).is_none());
    }

    #[test]
    fn blank_lines_in_use_block() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\n\t./a\n\n\t./b\n\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./a", "./b"]);
    }

    /// PATTERN-1 (TASK-1255): `use(// note` (no whitespace before the
    /// inline comment) is legal go.work syntax cmd/go accepts. The parser
    /// must populate the use list rather than silently dropping every
    /// member because the opener didn't match.
    #[test]
    fn use_block_with_inline_comment_no_whitespace_populates_list() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse(//ws-members\n\t./api\n\t./cmd\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    /// PATTERN-1 (TASK-1216): a nested `use (` opener inside an outer block
    /// must be rejected with a warn, not absorbed as a directory entry whose
    /// name is `use (`.
    #[test]
    fn parse_use_dirs_warns_on_nested_block_opener() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api\n\tuse (\n\t./cmd\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert!(
            !dirs.iter().any(|d| d.contains("use (") || d == "use("),
            "nested `use (` should not appear as a directive: {dirs:?}"
        );
        // The legitimate entries before and after the nested opener still
        // resolve.
        assert!(dirs.contains(&"./api".to_string()));
    }

    /// PATTERN-1 (TASK-1724): cmd/go accepts a trailing line comment after the
    /// block terminator. The parser used to push `)` as a use directive and
    /// leave the block open, absorbing every following top-level line into the
    /// directive list — an inflated module count plus a `cwd.join(")")` probe.
    #[test]
    fn block_terminator_with_trailing_comment_closes_the_block() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api\n\t./cmd\n) // workspace members\n\ngo 1.22\n\nreplace (\n\tex.com/a => ./api\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    /// PATTERN-1 (TASK-1724) AC #3: the same holds for a tab-indented
    /// terminator and for a no-whitespace inline comment.
    #[test]
    fn block_terminator_variants_close_the_block() {
        for terminator in [")//members", "\t)", "\t) // members", ")\t// members"] {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(
                dir.path().join("go.work"),
                format!("go 1.21\n\nuse (\n\t./api\n{terminator}\n\ngo 1.22\n"),
            )
            .unwrap();
            let dirs = parse_use_dirs(dir.path()).unwrap();
            assert_eq!(dirs, vec!["./api"], "terminator {terminator:?}");
        }
    }

    /// PATTERN-1 (TASK-1727): quoted and tab-separated `use` directives are
    /// legal modfile syntax. Left quoted, `ProjectUnit::path` kept its quotes
    /// and matched no `tokei_files` row, so the module reported zero LOC.
    #[test]
    fn parses_quoted_and_tab_separated_use_directives() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse\t./tabbed\n\nuse (\n\t\"./api\"\n\t\"./has space/sub\"\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./tabbed", "./api", "./has space/sub"]);
    }

    #[test]
    fn multiple_single_line_uses() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\nuse ./first\nuse ./second\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./first", "./second"]);
    }
}
