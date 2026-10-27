//! Discovery of the cargo workspaces nested inside a project.
//!
//! Cargo has no command that lists every workspace in a repository —
//! `cargo metadata` only sees the one the current directory belongs to — but
//! the manifests say enough. A `Cargo.toml` below the root is a workspace of
//! its own when it declares a `[workspace]` table (cargo-fuzz's
//! `fuzz/Cargo.toml` carries `[workspace] members = ["."]` for exactly this
//! reason), or when it is a package the root's `[workspace].exclude` names:
//! cargo then treats it as a standalone root. Any other nested manifest is a
//! member of the root workspace or a fixture, not a workspace.
//!
//! The walk honours `.gitignore` and skips hidden directories, `target/`
//! and `vendor/`, never follows symlinks, and stops at [`MAX_DEPTH`]. It runs
//! from `ops init` and `ops init --rust --check` only, never on ordinary
//! command dispatch: the resolved plan follows `[cargo] workspaces` in
//! `.ops.toml`, not whatever directories exist today.

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use ops_core::text::read_capped_to_string;

/// Deepest directory level searched for a nested `Cargo.toml`.
pub const MAX_DEPTH: usize = 8;

/// Directory names never descended into: build output and vendored crates.
const SKIP_DIRS: &[&str] = &["target", "vendor"];

/// A cargo workspace below the project root.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NestedWorkspace {
    /// Directory relative to the project root, `/`-separated (`fuzz`).
    pub dir: String,
    /// Its manifest declares a `[workspace]` table.
    pub own_workspace: bool,
    /// The root manifest's `[workspace].exclude` names it.
    pub excluded_by_root: bool,
}

impl NestedWorkspace {
    /// Why it counts as a workspace, for `ops init` comments and drift
    /// messages.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        match (self.own_workspace, self.excluded_by_root) {
            (true, true) => "own [workspace], excluded from the root workspace",
            (true, false) => "own [workspace]",
            _ => "excluded from the root workspace",
        }
    }
}

/// Normalize a manifest-relative directory for comparison: no leading `./`
/// and no trailing `/`.
#[must_use]
pub fn normalize_dir(dir: &str) -> &str {
    let mut dir = dir;
    while let Some(rest) = dir.strip_prefix("./") {
        dir = rest;
    }
    dir.trim_end_matches('/')
}

/// Whether the root's `exclude` entries name `dir` or one of its ancestors
/// (cargo excludes a path together with everything under it). Entries are
/// matched literally; a glob entry matches nothing here.
fn excluded(exclude: &[String], dir: &str) -> bool {
    exclude.iter().map(|e| normalize_dir(e)).any(|e| {
        !e.is_empty()
            && dir
                .strip_prefix(e)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    })
}

fn table_of(path: &Path) -> Option<toml::Table> {
    let text = match read_capped_to_string(path) {
        Ok(text) => text,
        Err(e) => {
            tracing::debug!(path = ?path.display(), error = %e, "skipping unreadable nested Cargo.toml");
            return None;
        }
    };
    match toml::from_str(&text) {
        Ok(table) => Some(table),
        Err(e) => {
            tracing::debug!(path = ?path.display(), error = %e, "skipping unparseable nested Cargo.toml");
            None
        }
    }
}

/// The cargo workspaces below `root`, sorted by directory.
///
/// A root without a parseable `Cargo.toml` has no `exclude` list, so only
/// manifests declaring `[workspace]` themselves are found. Unreadable or
/// unparseable nested manifests are skipped.
#[must_use]
pub fn nested_workspaces(root: &Path) -> Vec<NestedWorkspace> {
    let exclude: Vec<String> = table_of(&root.join("Cargo.toml"))
        .and_then(|t| t.get("workspace")?.get("exclude")?.as_array().cloned())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|e| e.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let walker = WalkBuilder::new(root)
        .require_git(false)
        .max_depth(Some(MAX_DEPTH))
        .filter_entry(|entry| {
            entry.depth() == 0
                || !entry.file_type().is_some_and(|t| t.is_dir())
                || !SKIP_DIRS
                    .iter()
                    .any(|d| *d == entry.file_name().to_string_lossy())
        })
        .build();

    let mut found = Vec::new();
    for entry in walker.flatten() {
        if entry.depth() < 2
            || entry.file_name() != "Cargo.toml"
            || !entry.file_type().is_some_and(|t| t.is_file())
        {
            continue;
        }
        let Some(dir) = entry.path().parent().and_then(|p| relative_dir(root, p)) else {
            continue;
        };
        let Some(table) = table_of(entry.path()) else {
            continue;
        };
        let own_workspace = table.get("workspace").is_some_and(toml::Value::is_table);
        let excluded_by_root = table.contains_key("package") && excluded(&exclude, &dir);
        if own_workspace || excluded_by_root {
            found.push(NestedWorkspace {
                dir,
                own_workspace,
                excluded_by_root,
            });
        }
    }
    found.sort_by(|a, b| a.dir.cmp(&b.dir));
    found
}

/// `dir` relative to `root`, `/`-separated; `None` for a non-UTF-8 path.
fn relative_dir(root: &Path, dir: &Path) -> Option<String> {
    let rel: PathBuf = dir.strip_prefix(root).ok()?.to_path_buf();
    let parts: Option<Vec<&str>> = rel.components().map(|c| c.as_os_str().to_str()).collect();
    Some(parts?.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    fn dirs(found: &[NestedWorkspace]) -> Vec<(&str, &'static str)> {
        found.iter().map(|w| (w.dir.as_str(), w.reason())).collect()
    }

    #[test]
    fn finds_own_and_excluded_workspaces() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"./fuzz\", \"tools/\"]\n",
        );
        write(root, "crates/a/Cargo.toml", "[package]\nname = \"a\"\n");
        write(
            root,
            "fuzz/Cargo.toml",
            "[package]\nname = \"f\"\n[workspace]\nmembers = [\".\"]\n",
        );
        write(root, "tools/gen/Cargo.toml", "[package]\nname = \"gen\"\n");
        write(root, "bench/Cargo.toml", "[workspace]\nmembers = []\n");

        assert_eq!(
            dirs(&nested_workspaces(root)),
            [
                ("bench", "own [workspace]"),
                ("fuzz", "own [workspace], excluded from the root workspace"),
                ("tools/gen", "excluded from the root workspace"),
            ]
        );
    }

    #[test]
    fn skips_build_output_vendored_ignored_and_broken_manifests() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let ws = "[workspace]\nmembers = [\".\"]\n";
        write(root, "Cargo.toml", "[package]\nname = \"r\"\n");
        write(root, ".gitignore", "/scratch/\n");
        for skipped in [
            "target/x",
            "fuzz/target/y",
            "vendor/z",
            ".hidden/w",
            "scratch/v",
        ] {
            write(root, &format!("{skipped}/Cargo.toml"), ws);
        }
        write(root, "broken/Cargo.toml", "[workspace\n");
        write(
            root,
            "fixtures/member/Cargo.toml",
            "[package]\nname = \"m\"\n",
        );
        assert!(nested_workspaces(root).is_empty());
    }

    #[test]
    fn root_without_a_manifest_still_finds_declared_workspaces() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "fuzz/Cargo.toml", "[workspace]\n");
        assert_eq!(
            dirs(&nested_workspaces(dir.path())),
            [("fuzz", "own [workspace]")]
        );
    }

    #[test]
    fn normalize_dir_strips_leading_dot_and_trailing_slash() {
        assert_eq!(normalize_dir("./fuzz/"), "fuzz");
        assert_eq!(normalize_dir("tools/fuzz"), "tools/fuzz");
        assert_eq!(normalize_dir("."), ".");
    }
}
