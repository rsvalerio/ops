//! Go `project_units` data provider.
//!
//! Reads `go.work` / `go.mod` to build a list of [`ProjectUnit`] entries
//! describing each module. LOC/file counts are enriched by the generic
//! `ops_about::run_about_units` runner.
//!
//! `go.work` takes precedence: when the root carries one, its `use`
//! directives are the unit list and the root `go.mod` is not consulted for
//! units. Otherwise the root `go.mod` yields a single root-module unit whose
//! path is the `"."` sentinel.

use std::path::{Component, Path};

use ops_about::cards::format_unit_name;
use ops_core::project_identity::ProjectUnit;
use ops_extension::{Context, DataProvider, DataProviderError};

pub const PROVIDER_NAME: &str = "project_units";

pub struct GoUnitsProvider;

impl DataProvider for GoUnitsProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let units = collect_units(ctx.working_directory());
        serde_json::to_value(&units).map_err(DataProviderError::from)
    }
}

fn collect_units(cwd: &Path) -> Vec<ProjectUnit> {
    if let Some(dirs) = crate::go_work::parse_use_dirs(cwd) {
        return dirs
            .into_iter()
            .map(|dir| unit_from_use_dir(cwd, &dir))
            .collect();
    }
    let (module, go_version) = read_mod_info(cwd);
    // The `Some` arm builds the unit across several statements; a
    // `map_or_else` closure would put the empty-vec default first and read
    // backwards.
    #[allow(clippy::option_if_let_else)]
    match module {
        Some(m) => {
            let mut unit = ProjectUnit::new(
                last_segment(Some(&m)).unwrap_or_else(|| m.clone()),
                // Root-module sentinel. A non-workspace `go.mod` lives at the
                // project root, so its unit subpath is `"."` — the same thing
                // `normalize_module_path` produces for a `use .` workspace
                // directive. Enrichment in `extensions/about/src/units.rs`
                // accepts either `""` or `"."` as that sentinel and takes LOC
                // and file counts from the project-wide totals instead of the
                // per-module `starts_with(file, path || '/')` join, so the
                // single-module card is project-wide by design: non-Go
                // content in cwd (vendored JS, generated artefacts) counts
                // toward it, as it does for single-package `package.json` and
                // `pyproject.toml` projects.
                ".".to_string(),
            );
            unit.version = go_version;
            unit.description = Some(m);
            vec![unit]
        }
        None => vec![],
    }
}

/// Build the [`ProjectUnit`] for a single `go.work` `use` directive: path
/// normalisation, the out-of-tree classification and its diagnostic, the
/// per-module `go.mod` lookup, and the description-shaping that marks
/// `(outside project root)` members.
///
/// A directive is out-of-tree when it is absolute or root-prefixed, when its
/// first path component is `..`, or when it carries a `..` past a real
/// segment. All three still produce a unit — the workspace member is real and
/// belongs on the card — but the first and third skip the `go.mod` read,
/// because the path they name lies outside the project root.
fn unit_from_use_dir(cwd: &Path, dir: &str) -> ProjectUnit {
    let normalized = normalize_module_path(dir);
    // Absolute and root-prefixed directives (`use /etc/secrets`,
    // `use \\?\C:\...`) point outside the project root as surely as a `..`
    // prefix does, so RootDir and Prefix count alongside ParentDir. This is
    // the `resolve_member_globs` threat model (`extensions/about/src/
    // workspace.rs`), one step stricter.
    let first_component = Path::new(&normalized).components().next();
    let out_of_tree_via_components = matches!(
        first_component,
        Some(Component::RootDir | Component::Prefix(_) | Component::ParentDir)
    );
    // Compare the *first path component* against `..`, not the raw string: a
    // `starts_with("..")` test would also flag legal directories such as
    // `..staging/api` or `..backup-2025`, whose first component merely begins
    // with two dots. Splitting on `/` and `\\` covers go.work entries
    // authored on Windows.
    let out_of_tree_via_string = normalized
        .split(['/', '\\'])
        .next()
        .is_some_and(|first| first == "..");
    // The two checks above inspect only the first component, so
    // `use ./api/../../../etc` normalises to `api/../../../etc` and reads as
    // in-tree. `Path::join` does not normalise `..` and the OS resolves it
    // lexically on open, so a `..` past a real segment escapes the root too.
    // The predicate is shared with the `replace`-target path in `go_mod`, so
    // both directives enforce one traversal policy.
    let has_embedded_traversal = crate::go_syntax::has_embedded_parent_dir_segment(&normalized);
    let out_of_tree =
        out_of_tree_via_components || out_of_tree_via_string || has_embedded_traversal;
    // Debug-format the directive so embedded newlines or ANSI escapes cannot
    // forge log lines, matching the project-wide path-log policy. Exactly one
    // warn per rejected directive.
    if has_embedded_traversal {
        tracing::warn!(
            directive = ?dir,
            "go.work `use` directive contains an embedded `..` traversal segment past the leading prefix; skipping the go.mod read",
        );
    } else if out_of_tree {
        // Out-of-tree workspace members (e.g. `use ../shared`) match no
        // `tokei_files.file` entry under cwd, so the unit would render with
        // zero LOC and no diagnostic. Surface it instead.
        tracing::warn!(
            directive = ?dir,
            "go.work `use` directive points outside the project root; LOC stats will be empty",
        );
    }
    // For an absolute or root-prefixed directive `cwd.join(&normalized)`
    // returns the absolute target verbatim, and an embedded-`..` directive
    // escapes the root the same way, so neither reads a `go.mod`: the unit is
    // emitted with no module or version, the same shape as a missing go.mod.
    let is_absolute_directive = matches!(
        first_component,
        Some(Component::RootDir | Component::Prefix(_))
    );
    let (module, go_version) = if is_absolute_directive || has_embedded_traversal {
        (None, None)
    } else {
        let mod_path = cwd.join(&normalized);
        read_mod_info(&mod_path)
    };
    let name = last_segment(module.as_deref()).unwrap_or_else(|| format_unit_name(&normalized));
    let description = if out_of_tree {
        Some(module.map_or_else(
            || "(outside project root)".to_string(),
            |m| format!("{m} (outside project root)"),
        ))
    } else {
        module
    };
    let mut unit = ProjectUnit::new(name, normalized);
    unit.version = go_version;
    unit.description = description;
    unit
}

/// Normalize a `go.work` use-directive entry so it matches `tokei_files.file`
/// paths, which are recorded relative to cwd with no `./` prefix and no
/// trailing separator.
///
/// A `use .` directive maps to the empty string, the root-module sentinel
/// enrichment accepts alongside `"."`: such a unit is enriched from
/// project-wide stats rather than the per-module path join.
fn normalize_module_path(dir: &str) -> String {
    let trimmed = dir
        .trim_start_matches("./")
        .trim_start_matches(".\\")
        .trim_end_matches(['/', '\\']);
    if trimmed == "." {
        String::new()
    } else {
        trimmed.to_string()
    }
}

/// Extract the module's display name from its path.
///
/// Returns the last `/`-separated segment, except when that segment is a Go
/// major-version suffix (`v\d+` for `v >= 2`). Per Go module semantics, paths
/// like `github.com/foo/bar/v2` carry the `/vN` suffix as a versioning
/// convention; the human-meaningful name is the *preceding* segment (`bar`).
///
/// - `last_segment("github.com/foo/bar/v2")` → `"bar"`
/// - `last_segment("github.com/openbao/openbao/api/v2")` → `"api"`
/// - `last_segment("module v2")` → `"module v2"` (no `/`, returned unchanged)
/// - `last_segment("github.com/foo/bar")` → `"bar"`
/// - `last_segment("foo/v")` → `"v"` (no digits, not a version suffix)
pub fn last_segment(module: Option<&str>) -> Option<String> {
    let m = module?;
    let mut segments: Vec<&str> = m.split('/').collect();
    if segments.len() >= 2 {
        if let Some(last) = segments.last() {
            if is_go_major_version_suffix(last) {
                segments.pop();
            }
        }
    }
    segments.last().map(|s| (*s).to_string())
}

/// True when `s` matches `^v\d+$` (Go major-version suffix shape).
fn is_go_major_version_suffix(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('v') else {
        return false;
    };
    !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
}

fn read_mod_info(dir: &Path) -> (Option<String>, Option<String>) {
    match crate::go_mod::parse(dir) {
        Some(m) => (m.module, m.go_version),
        None => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_use_dirs_multi() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api\n\t./cmd\n)\n",
        )
        .unwrap();
        let dirs = crate::go_work::parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    #[test]
    fn read_mod_info_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/user/myapp\n\ngo 1.22\n",
        )
        .unwrap();
        let (module, go_version) = read_mod_info(dir.path());
        assert_eq!(module.as_deref(), Some("github.com/user/myapp"));
        assert_eq!(go_version.as_deref(), Some("1.22"));
    }

    #[test]
    fn collect_units_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api\n\t./cmd\n)\n",
        )
        .unwrap();
        let api = dir.path().join("api");
        std::fs::create_dir(&api).unwrap();
        std::fs::write(api.join("go.mod"), "module example.com/api\n\ngo 1.21\n").unwrap();
        let cmd = dir.path().join("cmd");
        std::fs::create_dir(&cmd).unwrap();
        std::fs::write(cmd.join("go.mod"), "module example.com/cmd\n\ngo 1.22\n").unwrap();

        let units = collect_units(dir.path());
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].name, "api");
        assert_eq!(units[0].description.as_deref(), Some("example.com/api"));
        assert_eq!(units[1].version.as_deref(), Some("1.22"));
    }

    #[test]
    fn collect_units_single_mod() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/user/app\n\ngo 1.23\n",
        )
        .unwrap();
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        // `.` is the root-module sentinel: enrichment
        // (`extensions/about/src/units.rs::enrich_from_db`) accepts `""` or
        // `"."` and uses project-wide stats for such a unit.
        assert_eq!(units[0].path, ".");
        assert_eq!(units[0].name, "app");
    }

    /// A non-workspace `go.mod` at cwd produces one unit whose `path` is the
    /// root-module sentinel, so enrichment
    /// (`extensions/about/src/units.rs`) uses project-wide LOC and file
    /// totals — the same single-package behaviour Node (`package.json`) and
    /// Python (`pyproject.toml`) get, with non-Go content at the root
    /// counting toward the card.
    #[test]
    fn collect_units_single_mod_with_non_go_files_uses_root_sentinel() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/user/app\n\ngo 1.23\n",
        )
        .unwrap();
        // Sprinkle non-Go and Go files at the module root to mimic a real
        // project: vendored JS, generated artefacts, and the actual Go
        // source. The path filter on the resulting unit decides which of
        // these contribute to the single-mod LOC card.
        std::fs::write(dir.path().join("main.go"), "package main\nfunc main(){}\n").unwrap();
        std::fs::create_dir_all(dir.path().join("vendor/js")).unwrap();
        std::fs::write(
            dir.path().join("vendor/js/bundle.js"),
            "console.log('vendored');\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("README.md"), "# app\n").unwrap();

        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        // Enrichment recognises `""` *or* `"."` as the root-module sentinel.
        // This provider emits `"."`: it is self-documenting (the relative
        // directory holding `go.mod`) and is what a `use .` workspace
        // directive resolves to through enrichment.
        assert!(
            units[0].path == "." || units[0].path.is_empty(),
            "expected root-module sentinel, got {:?}",
            units[0].path
        );
        assert_eq!(units[0].name, "app");
        assert_eq!(units[0].description.as_deref(), Some("github.com/user/app"));
    }

    #[test]
    fn normalize_strips_dot_slash_prefix() {
        assert_eq!(
            normalize_module_path("./staging/src/k8s.io/api"),
            "staging/src/k8s.io/api"
        );
        assert_eq!(normalize_module_path("./api/"), "api");
        assert_eq!(normalize_module_path("."), "");
        assert_eq!(normalize_module_path("pkg/foo"), "pkg/foo");
    }

    #[test]
    fn collect_units_out_of_tree_use_directive_does_not_panic() {
        // `use ../shared` is accepted by cmd/go but lives outside cwd, so
        // the unit's `..` path matches no tokei_files row. The unit is still
        // emitted (with zero LOC) alongside a diagnostic.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t../shared\n)\n",
        )
        .unwrap();
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].path, "../shared");
        assert_eq!(
            units[0].description.as_deref(),
            Some("(outside project root)")
        );
    }

    /// The `use` directive reaches `tracing::warn!` through the `?`
    /// formatter, so embedded newlines or ANSI escapes cannot forge
    /// multi-line log records.
    #[test]
    fn directive_debug_escapes_control_characters() {
        let dir = "../shared\nINJECTED line\u{1b}[31m";
        ops_about::test_support::assert_debug_escapes_control_chars(dir);
    }

    /// A `use ..staging/api` directive names a legal directory whose first
    /// component merely *begins* with `..`, so it is in-tree: its `go.mod` is
    /// read, the description carries no `(outside project root)` suffix, and
    /// no warn is emitted.
    #[test]
    fn collect_units_dotdot_prefixed_dir_is_in_tree() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./..staging/api\n)\n",
        )
        .unwrap();
        let staging_api = dir.path().join("..staging").join("api");
        std::fs::create_dir_all(&staging_api).unwrap();
        std::fs::write(
            staging_api.join("go.mod"),
            "module example.com/staging/api\n\ngo 1.21\n",
        )
        .unwrap();

        let (units, warn_count) =
            ops_about::test_support::count_warnings(|| collect_units(dir.path()));

        assert_eq!(units.len(), 1);
        assert_eq!(units[0].path, "..staging/api");
        // In-tree: description is the bare module name, no suffix.
        assert_eq!(
            units[0].description.as_deref(),
            Some("example.com/staging/api")
        );
        assert!(!units[0]
            .description
            .as_deref()
            .unwrap_or("")
            .contains("(outside project root)"));
        assert_eq!(warn_count, 0);
    }

    /// A `/vN` major-version suffix is stripped, so the rendered name is the
    /// preceding segment rather than the literal `v2`.
    #[test]
    fn last_segment_strips_go_major_version_suffix() {
        assert_eq!(
            last_segment(Some("github.com/foo/bar/v2")).as_deref(),
            Some("bar")
        );
        assert_eq!(
            last_segment(Some("github.com/openbao/openbao/api/v2")).as_deref(),
            Some("api")
        );
        // /v10 is also a valid Go major-version suffix.
        assert_eq!(
            last_segment(Some("example.com/x/v10")).as_deref(),
            Some("x")
        );
    }

    #[test]
    fn last_segment_no_suffix_returns_last_path_component() {
        assert_eq!(
            last_segment(Some("github.com/foo/bar")).as_deref(),
            Some("bar")
        );
    }

    #[test]
    fn last_segment_single_segment_unchanged() {
        // No `/` — a bare module string is returned whole, even when it
        // contains a `vN`-shaped token.
        assert_eq!(last_segment(Some("module")).as_deref(), Some("module"));
        // A single segment that happens to be `v2` is not a suffix on
        // anything; return it verbatim.
        assert_eq!(last_segment(Some("v2")).as_deref(), Some("v2"));
    }

    #[test]
    fn last_segment_trailing_v_without_digits_is_not_a_version() {
        // `v` alone is not a Go major-version suffix; keep it as the name.
        assert_eq!(last_segment(Some("foo/v")).as_deref(), Some("v"));
    }

    /// An absolute `use` directive marks the unit out-of-tree and does not
    /// invoke `read_mod_info` on the absolute target — the same guard
    /// `resolve_member_globs` applies in `workspace.rs`.
    #[cfg(unix)]
    #[test]
    fn collect_units_absolute_use_directive_is_marked_out_of_tree() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        // Place a real go.mod at the absolute target to prove we're not
        // reading it. If the parser ran against this file the module name
        // would propagate into the unit description and the assertion would
        // fail.
        std::fs::write(
            other.path().join("go.mod"),
            "module example.com/should-not-be-read\n\ngo 1.21\n",
        )
        .unwrap();
        let absolute = other.path().to_string_lossy().to_string();
        std::fs::write(
            dir.path().join("go.work"),
            format!("go 1.21\n\nuse (\n\t{absolute}\n)\n"),
        )
        .unwrap();

        let (units, warn_count) =
            ops_about::test_support::count_warnings(|| collect_units(dir.path()));

        assert_eq!(units.len(), 1);
        // Out-of-tree marker present on the description.
        assert_eq!(
            units[0].description.as_deref(),
            Some("(outside project root)"),
            "absolute directive should not have populated module via read_mod_info"
        );
        // The directive triggered exactly one warn.
        assert_eq!(warn_count, 1);
    }

    /// A `use` directive whose normalized path carries a `..` past a real
    /// segment escapes the project root at the OS layer (`Path::join` does
    /// not normalise `..`), so it is treated as out-of-tree: no go.mod read
    /// at the traversal target, no module name in the description, and
    /// exactly one warn.
    #[cfg(unix)]
    #[test]
    fn collect_units_embedded_parent_dir_use_directive_is_marked_out_of_tree() {
        let dir = tempfile::tempdir().unwrap();
        // Place a real go.mod at the traversal target. `<cwd>/api/../../<leaf>`
        // resolves to `<cwd>/../<leaf>`, i.e. a sibling of the project root.
        let leaf = dir
            .path()
            .file_name()
            .expect("tempdir has a final component")
            .to_string_lossy()
            .to_string();
        let target = dir.path().parent().expect("tempdir has a parent").join(
            // Keep the escape inside the tempdir's parent by targeting the
            // tempdir itself via the traversal, then a nested marker dir.
            format!("{leaf}-sec14"),
        );
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(
            target.join("go.mod"),
            "module example.com/should-not-be-read\n\ngo 1.21\n",
        )
        .unwrap();

        std::fs::write(
            dir.path().join("go.work"),
            format!("go 1.21\n\nuse (\n\t./api/../../{leaf}-sec14\n)\n"),
        )
        .unwrap();

        let (units, warn_count) =
            ops_about::test_support::count_warnings(|| collect_units(dir.path()));

        std::fs::remove_dir_all(&target).ok();

        assert_eq!(units.len(), 1);
        assert!(
            !units.iter().any(|u| u
                .description
                .as_deref()
                .unwrap_or("")
                .contains("should-not-be-read")),
            "traversal target's go.mod must not be read: {:?}",
            units[0].description
        );
        assert_eq!(
            units[0].description.as_deref(),
            Some("(outside project root)")
        );
        assert!(units[0].version.is_none());
        assert_eq!(warn_count, 1);
    }

    /// A *leading* run of `..` is accepted: its `go.mod` is read and the unit
    /// is merely marked `(outside project root)`. Only `..` past a real
    /// segment skips the read outright.
    #[test]
    fn collect_units_leading_parent_dir_use_directive_still_reads_go_mod() {
        let root = tempfile::tempdir().unwrap();
        let cwd = root.path().join("project");
        let shared = root.path().join("shared");
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::write(
            shared.join("go.mod"),
            "module example.com/shared\n\ngo 1.21\n",
        )
        .unwrap();
        std::fs::write(cwd.join("go.work"), "go 1.21\n\nuse (\n\t../shared\n)\n").unwrap();

        let units = collect_units(&cwd);
        assert_eq!(units.len(), 1);
        assert_eq!(
            units[0].description.as_deref(),
            Some("example.com/shared (outside project root)")
        );
    }

    #[test]
    fn collect_units_empty() {
        let dir = tempfile::tempdir().unwrap();
        let units = collect_units(dir.path());
        assert!(units.is_empty());
    }

    /// `PROVIDER_NAME` is the key the registry indexes this provider under
    /// (`lib.rs`'s `register_data_providers`), and the literal on the right
    /// is the cross-stack registry contract shared with the Node and Rust
    /// stacks — a typo in either silently unregisters the Go units card.
    #[test]
    fn units_provider_name() {
        assert_eq!(GoUnitsProvider.name(), PROVIDER_NAME);
        assert_eq!(PROVIDER_NAME, "project_units");
    }

    /// `GoUnitsProvider::provide` driven through a real `Context` over a
    /// `go.work` fixture: the returned `Value` deserialises back into
    /// `Vec<ProjectUnit>`, pinning the `serde_json::to_value` step and the
    /// JSON shape consumers read, not just the private `collect_units`
    /// helper. The Rust stack holds the same provider-level coverage in
    /// `extensions-rust/about/src/coverage_provider.rs`.
    #[test]
    fn units_provider_serialises_go_work_modules() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api\n\t./cmd\n)\n",
        )
        .unwrap();
        for (name, module, version) in [
            ("api", "example.com/api", "1.21"),
            ("cmd", "example.com/cmd", "1.22"),
        ] {
            std::fs::create_dir_all(dir.path().join(name)).unwrap();
            std::fs::write(
                dir.path().join(name).join("go.mod"),
                format!("module {module}\n\ngo {version}\n"),
            )
            .unwrap();
        }

        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = GoUnitsProvider.provide(&mut ctx).unwrap();
        let units: Vec<ProjectUnit> = serde_json::from_value(value).unwrap();

        assert_eq!(units.len(), 2, "one unit per go.work use dir: {units:?}");
        assert_eq!(units[0].name, "api");
        assert_eq!(units[0].path, "api");
        assert_eq!(units[0].version.as_deref(), Some("1.21"));
        assert_eq!(units[0].description.as_deref(), Some("example.com/api"));
        assert_eq!(units[1].name, "cmd");
        assert_eq!(units[1].path, "cmd");
        assert_eq!(units[1].version.as_deref(), Some("1.22"));
    }

    /// A directory with no `go.work` / `go.mod` serialises to an empty JSON
    /// array — not `null`, and not an error.
    #[test]
    fn units_provider_empty_project_is_empty_array() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = GoUnitsProvider.provide(&mut ctx).unwrap();
        assert_eq!(value, serde_json::json!([]));
        let units: Vec<ProjectUnit> = serde_json::from_value(value).unwrap();
        assert!(units.is_empty());
    }
}
