//! Node `project_units` data provider.
//!
//! Enumerates workspace members from `package.json` (`workspaces`) or
//! `pnpm-workspace.yaml` (pnpm). Glob entries like `packages/*` expand to
//! directories that contain a `package.json`.
//!
//! Precedence: when `package.json` declares any positive `workspaces`
//! includes, `pnpm-workspace.yaml` is ignored (npm/yarn shadow pnpm). An
//! `workspaces` array containing only `!`-prefixed exclusions is treated as
//! "no positive includes" and the pnpm fallback is consulted instead. Both
//! sources accept `!`-prefixed exclusion entries.

use std::path::Path;

use ops_about::cards::format_unit_name;
use ops_core::project_identity::ProjectUnit;
use ops_extension::{Context, DataProvider, DataProviderError};
use serde::Deserialize;

pub(crate) const PROVIDER_NAME: &str = "project_units";

pub(crate) struct NodeUnitsProvider;

impl DataProvider for NodeUnitsProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let units = collect_units(ctx.working_directory.as_path());
        serde_json::to_value(&units).map_err(DataProviderError::from)
    }
}

fn collect_units(cwd: &Path) -> Vec<ProjectUnit> {
    let (includes, excludes) = workspace_member_globs(cwd);
    let resolved =
        ops_about::workspace::resolve_member_globs(&includes, &excludes, cwd, "package.json");
    resolved
        .into_iter()
        .map(|(member, manifest)| {
            let manifest_path = cwd.join(&member).join("package.json");
            // DUP-3 / TASK-0987: call the shared `parse_package_metadata`
            // directly so the per-stack `PackageProbe` lives next to the
            // deserialiser, not behind a parallel shim function.
            let meta =
                ops_about::workspace::parse_package_metadata(&manifest_path, &manifest, |c| {
                    serde_json::from_str::<PackageProbe>(c).map(|p| {
                        ops_about::workspace::PackageMetadata {
                            name: p.name,
                            version: p.version,
                            description: p.description,
                        }
                    })
                });
            let mut unit = ProjectUnit::new(
                meta.name.unwrap_or_else(|| format_unit_name(&member)),
                member,
            );
            unit.version = meta.version;
            unit.description = meta.description;
            unit
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct RawRoot {
    workspaces: Option<WorkspacesField>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum WorkspacesField {
    List(Vec<String>),
    Object {
        #[serde(default)]
        packages: Vec<String>,
    },
}

/// Collect (includes, excludes) glob patterns from either
/// `package.json`.workspaces or `pnpm-workspace.yaml` (naive YAML parse —
/// packages-list only). `!`-prefixed entries are split into `excludes`. The
/// pnpm fallback fires only when no positive include is declared in
/// `package.json` (an exclude-only `workspaces` array still triggers it).
fn workspace_member_globs(root: &Path) -> (Vec<String>, Vec<String>) {
    let mut includes: Vec<String> = Vec::new();
    let mut excludes: Vec<String> = Vec::new();

    let pkg_path = root.join("package.json");
    // DUP-3 (TASK-0931): share the file read with the identity provider via
    // the per-process manifest cache. Each consumer still parses its own
    // typed projection (`RawRoot` here, `RawPackage` for identity) — only
    // the IO + UTF-8 validation is deduplicated, no Value tree clone.
    if let Some(content) = crate::manifest_cache::package_json_text(root) {
        match serde_json::from_str::<RawRoot>(&content) {
            Ok(raw) => {
                if let Some(ws) = raw.workspaces {
                    let items = match ws {
                        WorkspacesField::List(items) => items,
                        WorkspacesField::Object { packages } => packages,
                    };
                    split_include_exclude(items, &mut includes, &mut excludes);
                }
            }
            Err(e) => {
                // ERR-7 (TASK-0930): Debug-format the path so embedded
                // newlines/ANSI escapes in attacker-controlled checkout
                // paths cannot forge log lines. Mirrors the sister site in
                // `package_json.rs` (TASK-0818).
                tracing::warn!(
                    path = ?pkg_path.display(),
                    error = ?e,
                    "failed to parse package.json"
                );
            }
        }
    }

    if includes.is_empty() {
        let pnpm_path = root.join("pnpm-workspace.yaml");
        if let Some(content) =
            ops_about::manifest_io::read_optional_text(&pnpm_path, "pnpm-workspace.yaml")
        {
            let PnpmParse {
                items,
                saw_packages_key,
            } = parse_pnpm_workspace_yaml(&content);
            // ERR-4 / TASK-0684: distinguish "no packages: key" from
            // "packages: key matched but produced 0 entries" — the second
            // case is the symptom of a YAML shape we don't recognise (block
            // scalar, anchored list, nested mapping). Operators couldn't
            // tell them apart before this debug event landed.
            if items.is_empty() && saw_packages_key {
                tracing::debug!(
                    path = %pnpm_path.display(),
                    "pnpm-workspace.yaml: packages: key matched but no entries parsed"
                );
            }
            split_include_exclude(items, &mut includes, &mut excludes);
        }
    }

    (includes, excludes)
}

fn split_include_exclude(
    items: Vec<String>,
    includes: &mut Vec<String>,
    excludes: &mut Vec<String>,
) {
    for item in items {
        let trimmed = item.trim_start_matches("./");
        if let Some(rest) = trimmed.strip_prefix('!') {
            excludes.push(rest.trim_start_matches("./").to_string());
        } else {
            includes.push(trimmed.to_string());
        }
    }
}

/// Outcome of `parse_pnpm_workspace_yaml`. `saw_packages_key` distinguishes
/// "no packages: key in this file" from "key matched but the shape isn't
/// one we recognise" — the caller emits a debug log on the second case
/// (ERR-4 / TASK-0684).
struct PnpmParse {
    items: Vec<String>,
    saw_packages_key: bool,
}

/// Minimal parser for the `packages:` list in `pnpm-workspace.yaml`.
/// Handles the common shapes:
///   packages:
///     - 'apps/*'
///     - "libs/*"
///     - services/api
fn parse_pnpm_workspace_yaml(content: &str) -> PnpmParse {
    let mut out = Vec::new();
    let mut saw_packages_key = false;
    let mut in_packages = false;
    for raw_line in content.lines() {
        let line = raw_line.trim_end();
        if line.trim_start().starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let trimmed_start = line.trim_start();
        if let Some(rest) = trimmed_start.strip_prefix("packages:") {
            saw_packages_key = true;
            let rest = rest.trim();
            if let Some(inner) = rest.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                for item in inner.split(',') {
                    let item = item.trim();
                    if !item.is_empty() {
                        out.push(unquote(item).to_string());
                    }
                }
                in_packages = false;
                continue;
            }
            in_packages = true;
            continue;
        }
        if in_packages {
            let leading_ws = line.chars().take_while(|c| c.is_whitespace()).count();
            if leading_ws == 0 {
                // Next top-level key ends the block.
                in_packages = false;
                continue;
            }
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("- ") {
                let stripped = strip_trailing_yaml_comment(rest.trim());
                out.push(unquote(stripped.trim()).to_string());
            } else if let Some(rest) = trimmed.strip_prefix('-') {
                let rest = rest.trim();
                if !rest.is_empty() {
                    let stripped = strip_trailing_yaml_comment(rest);
                    out.push(unquote(stripped.trim()).to_string());
                }
            }
        }
    }
    PnpmParse {
        items: out,
        saw_packages_key,
    }
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    s.strip_prefix('\'')
        .and_then(|t| t.strip_suffix('\''))
        .or_else(|| s.strip_prefix('"').and_then(|t| t.strip_suffix('"')))
        .unwrap_or(s)
}

/// PATTERN-1 / TASK-1061: drop a trailing YAML `# comment` from a list-item
/// value. A `#` only starts a comment when it follows whitespace AND is not
/// inside a matching pair of single or double quotes — `'#literal'` and
/// `"#literal"` must survive intact. Walks the string left-to-right tracking
/// quote state; on the first whitespace-then-`#` outside quotes, truncates.
fn strip_trailing_yaml_comment(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut in_single = false;
    let mut in_double = false;
    let mut prev_ws = true; // a leading `#` (no preceding char) acts as a comment too
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'\'' if !in_double => in_single = !in_single,
            b'"' if !in_single => in_double = !in_double,
            b'#' if !in_single && !in_double && prev_ws => {
                return s[..i].trim_end();
            }
            _ => {}
        }
        prev_ws = (b as char).is_whitespace();
    }
    s
}

#[derive(Debug, Deserialize)]
struct PackageProbe {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    /// ERR-7 (TASK-0930): the `workspace_member_globs` warn event
    /// Debug-formats the path so a checkout containing newlines or ANSI
    /// escapes cannot forge log records. DUP-3 / TASK-0985: shared
    /// helper — see `ops_about::test_support`.
    #[test]
    fn workspace_member_globs_path_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc/package.json");
        ops_about::test_support::assert_debug_escapes_control_chars(p.display());
    }

    #[test]
    fn no_workspaces_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("package.json"), r#"{ "name": "solo" }"#);
        assert!(collect_units(dir.path()).is_empty());
    }

    #[test]
    fn npm_workspaces_array_form() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "root", "workspaces": ["packages/*"] }"#,
        );
        write(
            &dir.path().join("packages/alpha/package.json"),
            r#"{ "name": "@scope/alpha", "version": "1.0.0", "description": "A" }"#,
        );
        write(
            &dir.path().join("packages/beta/package.json"),
            r#"{ "name": "beta", "version": "2.0.0" }"#,
        );
        // No package.json → not a workspace.
        std::fs::create_dir_all(dir.path().join("packages/not-a-pkg")).unwrap();

        let units = collect_units(dir.path());
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].name, "@scope/alpha");
        assert_eq!(units[0].version.as_deref(), Some("1.0.0"));
        assert_eq!(units[0].description.as_deref(), Some("A"));
        assert_eq!(units[1].name, "beta");
    }

    #[test]
    fn yarn_workspaces_object_form() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "root", "workspaces": { "packages": ["apps/*"] } }"#,
        );
        write(
            &dir.path().join("apps/web/package.json"),
            r#"{ "name": "web", "version": "0.0.1" }"#,
        );
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].path, "apps/web");
        assert_eq!(units[0].name, "web");
    }

    #[test]
    fn parse_pnpm_block_scalar_shape_flags_empty_with_packages_key() {
        // ERR-4 / TASK-0684: a block-scalar `packages: |\n  apps/*` is not
        // a shape we recognise. The parser must record that `packages:`
        // matched even though no entries came out, so the caller can emit
        // a debug log distinguishing it from "no packages: key at all".
        let yaml = "packages: |\n  apps/*\n";
        let r = parse_pnpm_workspace_yaml(yaml);
        assert!(r.items.is_empty());
        assert!(r.saw_packages_key);

        let no_key = "name: foo\n";
        let r = parse_pnpm_workspace_yaml(no_key);
        assert!(r.items.is_empty());
        assert!(!r.saw_packages_key);
    }

    #[test]
    fn pnpm_workspace_yaml() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("package.json"), r#"{ "name": "root" }"#);
        write(
            &dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'libs/*'\n  - \"apps/web\"\n",
        );
        write(
            &dir.path().join("libs/foo/package.json"),
            r#"{ "name": "foo" }"#,
        );
        write(
            &dir.path().join("apps/web/package.json"),
            r#"{ "name": "web" }"#,
        );
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 2);
        let names: Vec<&str> = units.iter().map(|u| u.name.as_str()).collect();
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"web"));
    }

    /// TASK-0400: `!`-prefixed yarn/npm exclusion entries filter resolved
    /// members from the `packages/*` glob.
    #[test]
    fn exclusion_pattern_filters_member() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "root", "workspaces": ["packages/*", "!packages/ignored"] }"#,
        );
        write(
            &dir.path().join("packages/keep/package.json"),
            r#"{ "name": "keep" }"#,
        );
        write(
            &dir.path().join("packages/ignored/package.json"),
            r#"{ "name": "ignored" }"#,
        );
        let units = collect_units(dir.path());
        let names: Vec<&str> = units.iter().map(|u| u.name.as_str()).collect();
        assert!(names.contains(&"keep"));
        assert!(!names.contains(&"ignored"));
    }

    #[test]
    fn exclusion_glob_pattern_filters_members() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "root", "workspaces": ["packages/*", "!packages/internal-*"] }"#,
        );
        write(
            &dir.path().join("packages/web/package.json"),
            r#"{ "name": "web" }"#,
        );
        write(
            &dir.path().join("packages/internal-tools/package.json"),
            r#"{ "name": "internal-tools" }"#,
        );
        let units = collect_units(dir.path());
        let names: Vec<&str> = units.iter().map(|u| u.name.as_str()).collect();
        assert!(names.contains(&"web"));
        assert!(!names.contains(&"internal-tools"));
    }

    #[test]
    fn falls_back_to_dir_name_when_no_name() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "root", "workspaces": ["packages/*"] }"#,
        );
        write(
            &dir.path().join("packages/quiet/package.json"),
            r#"{ "version": "0.1.0" }"#,
        );
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].name, "Quiet");
    }

    /// TASK-0480: pnpm-workspace.yaml `!`-prefixed entries filter resolved
    /// members the same way npm/yarn `!`-prefixed entries do.
    #[test]
    fn pnpm_exclusion_pattern_filters_member() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("package.json"), r#"{ "name": "root" }"#);
        write(
            &dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n  - '!packages/internal-*'\n",
        );
        write(
            &dir.path().join("packages/keep/package.json"),
            r#"{ "name": "keep" }"#,
        );
        write(
            &dir.path().join("packages/internal-thing/package.json"),
            r#"{ "name": "internal-thing" }"#,
        );
        let units = collect_units(dir.path());
        let names: Vec<&str> = units.iter().map(|u| u.name.as_str()).collect();
        assert!(names.contains(&"keep"));
        assert!(!names.contains(&"internal-thing"));
    }

    /// TASK-0488: a `package.json` whose `workspaces` array contains only
    /// `!`-prefixed exclusions has no positive includes, so the
    /// pnpm-workspace.yaml fallback still applies.
    #[test]
    fn exclude_only_workspaces_falls_back_to_pnpm() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "root", "workspaces": ["!packages/legacy"] }"#,
        );
        write(
            &dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'libs/*'\n",
        );
        write(
            &dir.path().join("libs/foo/package.json"),
            r#"{ "name": "foo" }"#,
        );
        let units = collect_units(dir.path());
        let names: Vec<&str> = units.iter().map(|u| u.name.as_str()).collect();
        assert!(names.contains(&"foo"));
    }

    #[test]
    fn parses_pnpm_packages_list() {
        let yaml = "packages:\n  - 'apps/*'\n  - \"libs/core\"\n  - services/api\n\nother: key\n";
        let pats = parse_pnpm_workspace_yaml(yaml).items;
        assert_eq!(pats, vec!["apps/*", "libs/core", "services/api"]);
    }

    #[test]
    fn parses_pnpm_packages_inline_list() {
        let yaml = "packages: ['apps/*', \"libs/core\", services/api]\n";
        let pats = parse_pnpm_workspace_yaml(yaml).items;
        assert_eq!(pats, vec!["apps/*", "libs/core", "services/api"]);
    }

    /// PATTERN-1 / TASK-1061: a trailing `# comment` after a quoted list
    /// item must be stripped before `unquote` runs — otherwise the value
    /// retains the closing quote+comment and matches no directory.
    #[test]
    fn pnpm_trailing_comment_after_quoted_item_is_stripped() {
        let yaml = "packages:\n  - 'apps/*' # note\n";
        let pats = parse_pnpm_workspace_yaml(yaml).items;
        assert_eq!(pats, vec!["apps/*"]);
    }

    /// PATTERN-1 / TASK-1061: a trailing `# comment` after an unquoted
    /// list item is also stripped (whitespace-prefixed `#` is the YAML
    /// comment marker).
    #[test]
    fn pnpm_trailing_comment_after_unquoted_item_is_stripped() {
        let yaml = "packages:\n  - apps/* # note\n";
        let pats = parse_pnpm_workspace_yaml(yaml).items;
        assert_eq!(pats, vec!["apps/*"]);
    }

    /// PATTERN-1 / TASK-1061: a `#` inside matching quotes is a literal
    /// character, not a comment marker — `'#literal-pattern'` must pass
    /// through intact.
    #[test]
    fn pnpm_hash_inside_quotes_is_not_a_comment() {
        let yaml = "packages:\n  - '#literal-pattern'\n";
        let pats = parse_pnpm_workspace_yaml(yaml).items;
        assert_eq!(pats, vec!["#literal-pattern"]);
    }
}
