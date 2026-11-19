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

/// Registry key this provider's units are indexed under.
pub const PROVIDER_NAME: &str = "project_units";

/// Go `project_units` provider: one [`ProjectUnit`] per `go.work`/`go.mod` module.
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
        return dirs.into_iter().map(|dir| unit_from_use_dir(cwd, &dir)).collect();
    }
    let (module, go_version) = read_mod_info(cwd);
    let Some(module) = module else {
        return vec![];
    };
    let mut unit = ProjectUnit::new(
        // A module path with no non-empty segment (`/`) has no better
        // display name than itself.
        last_segment(&module).unwrap_or(&module).to_string(),
        // Root-module sentinel. A non-workspace `go.mod` lives at the
        // project root, so its unit subpath is `"."` — the same thing
        // `normalize_module_path` produces for a `use .` workspace
        // directive. Enrichment in `extensions/about/src/units.rs` accepts
        // either `""` or `"."` as that sentinel and takes LOC and file
        // counts from the project-wide totals instead of the per-module
        // `starts_with(file, path || '/')` join, so the single-module card
        // is project-wide by design: non-Go content in cwd (vendored JS,
        // generated artefacts) counts toward it, as it does for
        // single-package `package.json` and `pyproject.toml` projects.
        ".".to_string(),
    );
    unit.version = go_version;
    unit.description = Some(module);
    vec![unit]
}

/// Where a normalized `go.work` `use` directive points, relative to the
/// project root. This is the crate's one traversal policy for
/// filesystem-valued directives — the same one `resolve_member_globs`
/// applies in `extensions/about/src/workspace.rs`, one step stricter — and
/// each variant fixes the warn, whether the member's `go.mod` is read, and
/// whether the description is marked `(outside project root)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DirectivePlacement {
    /// Inside the project root: read `go.mod`, no warn, no marker.
    InTree,
    /// A leading run of `..` (`use ../shared`): a legal cmd/go sibling
    /// member, so its `go.mod` is read, but it matches no `tokei_files` row
    /// under cwd and renders with zero LOC.
    OutOfTreeReadable,
    /// Absolute or root-prefixed (`use /etc/secrets`, `use \\?\C:\...`):
    /// `cwd.join` would return the target verbatim, so no `go.mod` is read.
    OutOfTreeAbsolute,
    /// A `..` past a real segment (`use ./api/../../etc`): `Path::join` does
    /// not normalise `..` and the OS resolves it lexically on open, so it
    /// escapes the root and no `go.mod` is read.
    EmbeddedTraversal,
}

impl DirectivePlacement {
    /// Classify a directive already passed through [`normalize_module_path`].
    fn classify(normalized: &str) -> Self {
        // Checked first: an embedded traversal skips the read whatever the
        // first component is.
        if crate::go_syntax::has_embedded_parent_dir_segment(normalized) {
            return Self::EmbeddedTraversal;
        }
        match Path::new(normalized).components().next() {
            Some(Component::RootDir | Component::Prefix(_)) => Self::OutOfTreeAbsolute,
            Some(Component::ParentDir) => Self::OutOfTreeReadable,
            // Compare the first *segment* against `..`, not the raw string:
            // `starts_with("..")` would also flag legal directories such as
            // `..staging/api`. Splitting on `\\` too catches go.work entries
            // authored on Windows, which `Path` on Unix reads as one
            // component.
            _ if normalized.split(['/', '\\']).next() == Some("..") => Self::OutOfTreeReadable,
            _ => Self::InTree,
        }
    }

    const fn reads_go_mod(self) -> bool {
        matches!(self, Self::InTree | Self::OutOfTreeReadable)
    }

    /// Emit the one diagnostic this placement carries, if any. The raw
    /// directive is Debug-formatted so embedded newlines or ANSI escapes
    /// cannot forge log lines, matching the project-wide path-log policy.
    fn warn(self, directive: &str) {
        match self {
            Self::InTree => {}
            Self::EmbeddedTraversal => tracing::warn!(
                directive = ?directive,
                "go.work `use` directive contains an embedded `..` traversal segment past the leading prefix; skipping the go.mod read",
            ),
            Self::OutOfTreeReadable | Self::OutOfTreeAbsolute => tracing::warn!(
                directive = ?directive,
                "go.work `use` directive points outside the project root; LOC stats will be empty",
            ),
        }
    }

    /// The unit description: the module path, suffixed with
    /// `(outside project root)` for every out-of-tree placement.
    fn describe(self, module: Option<String>) -> Option<String> {
        if self == Self::InTree {
            return module;
        }
        Some(module.map_or_else(
            || "(outside project root)".to_string(),
            |m| format!("{m} (outside project root)"),
        ))
    }
}

/// Build the [`ProjectUnit`] for a single `go.work` `use` directive.
///
/// Every directive produces a unit — an out-of-tree workspace member is real
/// and belongs on the card — and its [`DirectivePlacement`] decides the
/// warn, the `go.mod` read and the description marker.
fn unit_from_use_dir(cwd: &Path, dir: &str) -> ProjectUnit {
    let normalized = normalize_module_path(dir);
    let placement = DirectivePlacement::classify(&normalized);
    placement.warn(dir);
    let (module, go_version) =
        if placement.reads_go_mod() { read_mod_info(&cwd.join(&normalized)) } else { (None, None) };
    let name = module
        .as_deref()
        .and_then(last_segment)
        .map_or_else(|| format_unit_name(&normalized), str::to_owned);
    let mut unit = ProjectUnit::new(name, normalized);
    unit.version = go_version;
    unit.description = placement.describe(module);
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
    let trimmed =
        dir.trim_start_matches("./").trim_start_matches(".\\").trim_end_matches(['/', '\\']);
    if trimmed == "." { String::new() } else { trimmed.to_string() }
}

/// Extract the module's display name from its path.
///
/// Returns the last `/`-separated segment, except when that segment is a Go
/// major-version suffix (`v\d+` for `v >= 2`). Per Go module semantics, paths
/// like `github.com/foo/bar/v2` carry the `/vN` suffix as a versioning
/// convention; the human-meaningful name is the *preceding* segment (`bar`).
///
/// Empty segments are ignored, so a trailing `/` (`example.com/m/`) does not
/// yield an empty name; `None` only when the path has no non-empty segment.
///
/// - `last_segment("github.com/foo/bar/v2")` → `"bar"`
/// - `last_segment("github.com/openbao/openbao/api/v2")` → `"api"`
/// - `last_segment("module v2")` → `"module v2"` (no `/`, returned unchanged)
/// - `last_segment("github.com/foo/bar")` → `"bar"`
/// - `last_segment("foo/v")` → `"v"` (no digits, not a version suffix)
/// - `last_segment("example.com/m/")` → `"m"`
pub fn last_segment(module: &str) -> Option<&str> {
    let mut segments = module.rsplit('/').filter(|s| !s.is_empty());
    let last = segments.next()?;
    match segments.next() {
        Some(previous) if is_go_major_version_suffix(last) => Some(previous),
        _ => Some(last),
    }
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
    fn read_mod_info_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module github.com/user/myapp\n\ngo 1.22\n")
            .unwrap();
        let (module, go_version) = read_mod_info(dir.path());
        assert_eq!(module.as_deref(), Some("github.com/user/myapp"));
        assert_eq!(go_version.as_deref(), Some("1.22"));
    }

    #[test]
    fn collect_units_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\n\nuse (\n\t./api\n\t./cmd\n)\n")
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
        std::fs::write(dir.path().join("go.mod"), "module github.com/user/app\n\ngo 1.23\n")
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
        std::fs::write(dir.path().join("go.mod"), "module github.com/user/app\n\ngo 1.23\n")
            .unwrap();
        // Sprinkle non-Go and Go files at the module root to mimic a real
        // project: vendored JS, generated artefacts, and the actual Go
        // source. The path filter on the resulting unit decides which of
        // these contribute to the single-mod LOC card.
        std::fs::write(dir.path().join("main.go"), "package main\nfunc main(){}\n").unwrap();
        std::fs::create_dir_all(dir.path().join("vendor/js")).unwrap();
        std::fs::write(dir.path().join("vendor/js/bundle.js"), "console.log('vendored');\n")
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
        assert_eq!(normalize_module_path("./staging/src/k8s.io/api"), "staging/src/k8s.io/api");
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
        std::fs::write(dir.path().join("go.work"), "go 1.21\n\nuse (\n\t../shared\n)\n").unwrap();
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].path, "../shared");
        assert_eq!(units[0].description.as_deref(), Some("(outside project root)"));
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
        std::fs::write(dir.path().join("go.work"), "go 1.21\n\nuse (\n\t./..staging/api\n)\n")
            .unwrap();
        let staging_api = dir.path().join("..staging").join("api");
        std::fs::create_dir_all(&staging_api).unwrap();
        std::fs::write(staging_api.join("go.mod"), "module example.com/staging/api\n\ngo 1.21\n")
            .unwrap();

        let (units, warn_count) =
            ops_core::test_utils::count_warnings(|| collect_units(dir.path()));

        assert_eq!(units.len(), 1);
        assert_eq!(units[0].path, "..staging/api");
        // In-tree: description is the bare module name, no suffix.
        assert_eq!(units[0].description.as_deref(), Some("example.com/staging/api"));
        assert!(!units[0].description.as_deref().unwrap_or("").contains("(outside project root)"));
        assert_eq!(warn_count, 0);
    }

    /// A `/vN` major-version suffix is stripped, so the rendered name is the
    /// preceding segment rather than the literal `v2`.
    #[test]
    fn last_segment_strips_go_major_version_suffix() {
        assert_eq!(last_segment("github.com/foo/bar/v2"), Some("bar"));
        assert_eq!(last_segment("github.com/openbao/openbao/api/v2"), Some("api"));
        // /v10 is also a valid Go major-version suffix.
        assert_eq!(last_segment("example.com/x/v10"), Some("x"));
    }

    #[test]
    fn last_segment_no_suffix_returns_last_path_component() {
        assert_eq!(last_segment("github.com/foo/bar"), Some("bar"));
    }

    #[test]
    fn last_segment_single_segment_unchanged() {
        // No `/` — a bare module string is returned whole, even when it
        // contains a `vN`-shaped token.
        assert_eq!(last_segment("module"), Some("module"));
        // A single segment that happens to be `v2` is not a suffix on
        // anything; return it verbatim.
        assert_eq!(last_segment("v2"), Some("v2"));
    }

    /// Empty segments are ignored: a trailing `/` does not produce an empty
    /// name, and a path with no non-empty segment has none.
    #[test]
    fn last_segment_ignores_empty_trailing_segments() {
        assert_eq!(last_segment("example.com/m/"), Some("m"));
        assert_eq!(last_segment("example.com/m/v2/"), Some("m"));
        assert_eq!(last_segment("/"), None);
        assert_eq!(last_segment(""), None);
    }

    #[test]
    fn last_segment_trailing_v_without_digits_is_not_a_version() {
        // `v` alone is not a Go major-version suffix; keep it as the name.
        assert_eq!(last_segment("foo/v"), Some("v"));
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
        std::fs::write(dir.path().join("go.work"), format!("go 1.21\n\nuse (\n\t{absolute}\n)\n"))
            .unwrap();

        let (units, warn_count) =
            ops_core::test_utils::count_warnings(|| collect_units(dir.path()));

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
        // One TempDir owns every path: cwd is `root/project`, and the
        // traversal `<cwd>/api/../../escaped` resolves to `root/escaped`,
        // where a real go.mod waits to prove it is never read.
        let root = tempfile::tempdir().unwrap();
        let cwd = root.path().join("project");
        let target = root.path().join("escaped");
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("go.mod"), "module example.com/should-not-be-read\n\ngo 1.21\n")
            .unwrap();
        std::fs::write(cwd.join("go.work"), "go 1.21\n\nuse (\n\t./api/../../escaped\n)\n")
            .unwrap();

        let (units, warn_count) = ops_core::test_utils::count_warnings(|| collect_units(&cwd));

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
        assert_eq!(units[0].description.as_deref(), Some("(outside project root)"));
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
        std::fs::write(shared.join("go.mod"), "module example.com/shared\n\ngo 1.21\n").unwrap();
        std::fs::write(cwd.join("go.work"), "go 1.21\n\nuse (\n\t../shared\n)\n").unwrap();

        let units = collect_units(&cwd);
        assert_eq!(units.len(), 1);
        assert_eq!(
            units[0].description.as_deref(),
            Some("example.com/shared (outside project root)")
        );
    }

    /// A malformed go.mod belonging to one workspace member is reported
    /// under that member's own path, so the operator can tell which of the
    /// workspace's go.mod files to fix.
    #[test]
    fn nested_use_dir_go_mod_warn_names_its_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\n\nuse (\n\t./api\n)\n").unwrap();
        let api = dir.path().join("api");
        std::fs::create_dir(&api).unwrap();
        std::fs::write(
            api.join("go.mod"),
            "module example.com/api\n\nreplace (\n\tex.com/a => ./a\n",
        )
        .unwrap();

        let rendered = ops_core::test_utils::capture_warn(|| {
            collect_units(dir.path());
        });

        let expected = format!("{:?}", api.join("go.mod"));
        assert!(rendered.contains(&expected), "warn should name {expected}: {rendered}");
    }

    #[test]
    fn collect_units_empty() {
        let dir = tempfile::tempdir().unwrap();
        let units = collect_units(dir.path());
        assert!(units.is_empty());
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
        std::fs::write(dir.path().join("go.work"), "go 1.21\n\nuse (\n\t./api\n\t./cmd\n)\n")
            .unwrap();
        for (name, module, version) in
            [("api", "example.com/api", "1.21"), ("cmd", "example.com/cmd", "1.22")]
        {
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
