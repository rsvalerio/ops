//! Discovery of the cargo workspaces nested inside a project.
//!
//! Cargo has no command that lists every workspace in a repository —
//! `cargo metadata` only sees the one the current directory belongs to — but
//! the manifests say enough. A `Cargo.toml` below the root is a workspace of
//! its own when it declares a `[workspace]` table (cargo-fuzz's
//! `fuzz/Cargo.toml` carries `[workspace] members = ["."]` for exactly this
//! reason), or when it is a package that the nearest enclosing workspace —
//! a nested one, else the root — names in its `[workspace].exclude`: cargo
//! then treats it as a standalone root. Any other nested manifest is a member
//! of an enclosing workspace or a fixture, not a workspace.
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
    /// The enclosing workspace whose `[workspace].exclude` names it: `"."`
    /// for the root, else the nested workspace's directory.
    pub excluded_from: Option<String>,
}

impl NestedWorkspace {
    /// Why it counts as a workspace, for `ops init` comments and drift
    /// messages.
    #[must_use]
    pub fn reason(&self) -> String {
        let excluded = self.excluded_from.as_deref().map(|from| match from {
            "." => "excluded from the root workspace".to_string(),
            dir => format!("excluded from the {dir} workspace"),
        });
        match (self.own_workspace, excluded) {
            (true, Some(excluded)) => format!("own [workspace], {excluded}"),
            (true, None) => "own [workspace]".to_string(),
            (false, Some(excluded)) => excluded,
            (false, None) => String::new(),
        }
    }
}

/// One nested manifest as the walk read it.
struct Manifest {
    dir: String,
    own_workspace: bool,
    package: bool,
    exclude: Vec<String>,
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

/// Whether `exclude` entries name `dir` or one of its ancestors
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
    let root_exclude = table_of(&root.join("Cargo.toml"))
        .map(|t| exclude_of(&t))
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

    let mut manifests = Vec::new();
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
        manifests.push(Manifest {
            dir,
            own_workspace: table.get("workspace").is_some_and(toml::Value::is_table),
            package: table.contains_key("package"),
            exclude: exclude_of(&table),
        });
    }

    let mut found: Vec<NestedWorkspace> = manifests
        .iter()
        .filter_map(|m| {
            // Cargo resolves a manifest to its nearest enclosing workspace
            // root: the deepest nested `[workspace]` above it, else the root.
            let enclosing = manifests
                .iter()
                .filter(|w| w.own_workspace && is_under(&m.dir, &w.dir))
                .max_by_key(|w| w.dir.len());
            let (from, exclude, rel) =
                enclosing.map_or((".", &root_exclude, m.dir.as_str()), |w| {
                    (
                        w.dir.as_str(),
                        &w.exclude,
                        m.dir
                            .get(w.dir.len().saturating_add(1)..)
                            .unwrap_or_default(),
                    )
                });
            let excluded_from = excluded(exclude, rel).then(|| from.to_string());
            // An excluded package is a standalone root; an excluded manifest
            // with neither table is nothing cargo builds.
            (m.own_workspace || (m.package && excluded_from.is_some())).then(|| NestedWorkspace {
                dir: m.dir.clone(),
                own_workspace: m.own_workspace,
                excluded_from,
            })
        })
        .collect();
    found.sort_by(|a, b| a.dir.cmp(&b.dir));
    found
}

/// A manifest's `[workspace].exclude` entries.
fn exclude_of(table: &toml::Table) -> Vec<String> {
    table
        .get("workspace")
        .and_then(|w| w.get("exclude"))
        .and_then(toml::Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|e| e.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Whether `dir` lies strictly below `ancestor` (both `/`-separated).
fn is_under(dir: &str, ancestor: &str) -> bool {
    dir.strip_prefix(ancestor)
        .is_some_and(|rest| rest.starts_with('/'))
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

    fn dirs(found: &[NestedWorkspace]) -> Vec<String> {
        found
            .iter()
            .map(|w| format!("{}: {}", w.dir, w.reason()))
            .collect()
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
                "bench: own [workspace]",
                "fuzz: own [workspace], excluded from the root workspace",
                "tools/gen: excluded from the root workspace",
            ]
        );
    }

    /// A member of an excluded nested workspace belongs to that workspace,
    /// not to a workspace of its own.
    #[test]
    fn members_of_a_nested_workspace_are_not_workspaces() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "Cargo.toml", "[workspace]\nexclude = [\"tools\"]\n");
        write(root, "tools/Cargo.toml", "[workspace]\nmembers = [\"a\"]\n");
        write(root, "tools/a/Cargo.toml", "[package]\nname = \"a\"\n");
        write(
            root,
            "tools/b/Cargo.toml",
            "[package]\nname = \"b\"\n[workspace]\n",
        );
        assert_eq!(
            dirs(&nested_workspaces(root)),
            [
                "tools: own [workspace], excluded from the root workspace",
                "tools/b: own [workspace]",
            ]
        );
    }

    /// A package the nested workspace itself excludes is a standalone root.
    #[test]
    fn a_package_a_nested_workspace_excludes_is_a_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "Cargo.toml", "[workspace]\nexclude = [\"tools\"]\n");
        write(
            root,
            "tools/Cargo.toml",
            "[workspace]\nmembers = [\"a\"]\nexclude = [\"b\"]\n",
        );
        write(root, "tools/a/Cargo.toml", "[package]\nname = \"a\"\n");
        write(root, "tools/b/Cargo.toml", "[package]\nname = \"b\"\n");
        assert_eq!(
            dirs(&nested_workspaces(root)),
            [
                "tools: own [workspace], excluded from the root workspace",
                "tools/b: excluded from the tools workspace",
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
            ["fuzz: own [workspace]"]
        );
    }

    #[test]
    fn normalize_dir_strips_leading_dot_and_trailing_slash() {
        assert_eq!(normalize_dir("./fuzz/"), "fuzz");
        assert_eq!(normalize_dir("tools/fuzz"), "tools/fuzz");
        assert_eq!(normalize_dir("."), ".");
    }
}
