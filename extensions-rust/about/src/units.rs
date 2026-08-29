//! Rust-specific `project_units` data provider.
//!
//! Reads `[workspace].members` from Cargo.toml and per-crate Cargo manifests
//! for display metadata. LOC/file counts are enriched by the generic
//! `run_about_units` runner when `DuckDB` is available.

use std::collections::HashMap;
use std::path::Path;

use ops_about::cards::format_unit_name;
use ops_cargo_toml::CargoToml;
use ops_core::project_identity::ProjectUnit;
use ops_extension::{Context, DataProvider, DataProviderError};

use crate::manifest::{load_workspace_manifest, log_manifest_load_failure};
use crate::members::member_path_is_workspace_safe;

/// Subset of crate manifest metadata used by the `project_units` provider.
///
/// FN-4 (TASK-0805): named struct so adding a field cannot silently shift
/// positions in tuple destructures at call sites.
#[derive(Debug, Default, Clone)]
pub struct CrateMetadata {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

pub const PROVIDER_NAME: &str = "project_units";

pub struct RustUnitsProvider;

impl DataProvider for RustUnitsProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    /// FN-1 / TASK-1784: orchestration only — load the manifest, fetch the dep
    /// counts, map each safe member to a [`ProjectUnit`]. The per-member work
    /// (canonical-path lookup, dep-count resolution, diagnostics) lives in
    /// named helpers below that are unit-testable without a live `DuckDB`.
    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let manifest = match load_workspace_manifest(ctx) {
            Ok(m) => m,
            Err(e) => {
                log_manifest_load_failure(&e);
                return Ok(serde_json::to_value(Vec::<ProjectUnit>::new())?);
            }
        };
        let dep_counts = crate_dep_counts(ctx);

        // PERF-3 / TASK-1569: the canonical-manifest-path map is built once per
        // workspace (`LoadedManifest` is itself cached per workspace root), so
        // sibling providers and later `provide()` calls reuse it.
        let canonical_manifests = manifest.canonical_member_manifests();
        // PERF-3 / TASK-1251: `resolved_members` already returns a sorted +
        // deduplicated list (TASK-0794 + TASK-1042); consumers must not re-sort.
        let units: Vec<ProjectUnit> = manifest
            .resolved_members()
            .iter()
            .map(String::as_str)
            .filter(|member| member_is_unit_safe(member))
            .map(|member| {
                build_unit(
                    member,
                    manifest.workspace_root(),
                    canonical_manifests,
                    &dep_counts,
                )
            })
            .collect();

        serde_json::to_value(&units).map_err(DataProviderError::from)
    }
}

/// Per-crate dep counts from `DuckDB`, keyed by `crate_manifest_path`
/// (ERR-2 / TASK-1253 — the previous bare-name key collided for renamed
/// (`package = "alt-name"`) or duplicate-named workspace members).
///
/// ERR-2 / TASK-0376: query failures route through `query_or_warn` so they
/// don't manifest as a silent "no deps" on a misconfigured DB.
fn crate_dep_counts(ctx: &Context) -> HashMap<String, i64> {
    ops_duckdb::get_db(ctx).map_or_else(HashMap::new, |db| {
        ops_duckdb::sql::query_or_warn(
            "query_crate_dep_counts",
            "per-crate dep_counts will be empty",
            HashMap::<String, i64>::new(),
            || ops_duckdb::sql::query_crate_dep_counts(db),
        )
    })
}

/// SEC-14 / TASK-1246 AC #1: defence-in-depth — even though
/// `resolved_workspace_members` already filters absolute / `..`-segment
/// entries, re-validate here so a future caller bypassing that helper (custom
/// enrichment, test harness) cannot drive `root.join(member)` at arbitrary
/// filesystem locations. The warn matches the helper's breadcrumb shape so an
/// attacker-controlled member surfaces exactly once per provider invocation.
fn member_is_unit_safe(member: &str) -> bool {
    if member_path_is_workspace_safe(member) {
        return true;
    }
    tracing::warn!(
        member = %member,
        "SEC-14 / TASK-1246: rejecting absolute or `..` workspace member in units provider"
    );
    false
}

/// Assemble one [`ProjectUnit`] from a workspace member.
///
/// CL-3 / TASK-1762: `workspace_root` is the *resolved* root, not the process
/// cwd — running `ops about` from a member crate must still find each
/// member's manifest.
fn build_unit(
    member: &str,
    workspace_root: &Path,
    canonical_manifests: &HashMap<String, std::path::PathBuf>,
    dep_counts: &HashMap<String, i64>,
) -> ProjectUnit {
    let crate_toml = workspace_root.join(member).join("Cargo.toml");
    let CrateMetadata {
        name: pkg_name,
        version,
        description,
    } = read_crate_metadata(&crate_toml);
    // PERF-3 / TASK-1569: canonical `Cargo.toml` paths are cached on
    // `LoadedManifest` so the N-syscall fan-out happens at most once per
    // workspace, not once per provide() invocation. ERR-2 / TASK-1253: cargo
    // metadata stores `manifest_path` as a canonicalised absolute path, so the
    // lookup keys must match.
    let canonical_manifest_path = canonical_manifests.get(member).cloned().unwrap_or_else(|| {
        std::fs::canonicalize(&crate_toml).unwrap_or_else(|_| crate_toml.clone())
    });

    let mut unit = ProjectUnit::new(format_unit_name(member), member.to_string());
    unit.version = version;
    unit.description = description;
    unit.dep_count = resolve_dep_count(
        member,
        pkg_name.as_deref(),
        &canonical_manifest_path,
        dep_counts,
    );
    unit
}

/// Resolve a member's dependency count from the `crate_manifest_path`-keyed
/// map, emitting the diagnostic breadcrumb for each way the lookup can come up
/// empty.
///
/// FN-1 / TASK-1784: extracted from `provide`'s map closure so the three
/// diagnostic branches are reachable from a unit test without a live `DuckDB`,
/// and so the `clippy::option_if_let_else` suppression the nesting used to
/// require is no longer needed.
fn resolve_dep_count(
    member: &str,
    package_name: Option<&str>,
    canonical_manifest_path: &Path,
    dep_counts: &HashMap<String, i64>,
) -> Option<i64> {
    if package_name.is_none() {
        tracing::debug!(
            member,
            "no package name resolved for member; dep_count unavailable"
        );
        return None;
    }
    // PERF-3 / TASK-1570: borrow the canonical path as `&str` so the HashMap
    // lookup costs no allocation (`HashMap<String, _>` borrows via
    // `Borrow<str>`). A non-UTF-8 canonical path skips the lookup with a debug
    // breadcrumb rather than collapsing through `to_string_lossy` and silently
    // keying on a U+FFFD corrupted name (sister-policy to TASK-0946).
    let Some(key) = canonical_manifest_path.to_str() else {
        tracing::debug!(
            member,
            manifest_path = ?canonical_manifest_path,
            "PERF-3 / TASK-1570: canonical manifest_path is not valid UTF-8; \
             skipping dep_count lookup rather than collapsing through to_string_lossy"
        );
        return None;
    };
    let lookup = dep_counts.get(key).copied();
    if lookup.is_none() {
        tracing::debug!(
            member,
            manifest_path = %key,
            "ERR-2 / TASK-1253: no dep_count row for canonical manifest_path"
        );
    }
    lookup
}

/// Read package name, version, and description from a crate's Cargo.toml.
///
/// Returns an all-`None` [`CrateMetadata`] on read or parse failure. `NotFound`
/// reads are silent (an absent member manifest is expected during workspace
/// globbing); other read errors are logged at `debug` and parse errors at
/// `warn` so a malformed Cargo.toml shows up in logs instead of silently
/// producing an empty unit (TASK-0377).
///
/// DUP-3 (TASK-0806): delegates to `ops_cargo_toml::CargoToml::parse` so this
/// extension does not maintain a second TOML parser for the same manifest
/// shape.
pub fn read_crate_metadata(crate_toml_path: &Path) -> CrateMetadata {
    // SEC-33 (TASK-0926): cap the per-crate manifest read; this fans out across
    // every workspace member declared by the root Cargo.toml.
    let content = match ops_core::text::read_capped_to_string(crate_toml_path) {
        Ok(c) => c,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                // ERR-7 (TASK-0977): Debug-format path/error so an
                // attacker-controlled workspace member name with embedded
                // newlines / ANSI escapes cannot forge log records.
                tracing::debug!(
                    path = ?crate_toml_path.display(),
                    error = ?e,
                    "failed to read crate manifest"
                );
            }
            return CrateMetadata::default();
        }
    };

    let parsed = match CargoToml::parse(&content) {
        Ok(p) => p,
        Err(e) => {
            // ERR-7 (TASK-0977): Debug-format path/error so an
            // attacker-controlled workspace member name with embedded
            // newlines / ANSI escapes cannot forge log records.
            tracing::warn!(
                path = ?crate_toml_path.display(),
                error = ?e,
                "failed to parse crate manifest as TOML"
            );
            return CrateMetadata::default();
        }
    };

    let name = parsed.package_name().map(str::to_string);
    let version = parsed.package_version().map(str::to_string);
    let description = parsed
        .package
        .as_ref()
        .and_then(|p| p.description.as_str())
        .map(str::to_string);

    CrateMetadata {
        name,
        version,
        description,
    }
}

/// Resolve display name for a member by reading its Cargo.toml, falling back
/// to the capitalized last path segment.
///
/// SEC-14 / TASK-1246: rejects absolute and `..`-traversal member entries
/// before any join and falls back to the formatted member name. `Path::join`
/// discards `workspace_root` when `member` is absolute and walks parents on
/// `..`, which would otherwise drive `read_capped_to_string` and tracing
/// breadcrumbs at any filesystem location.
pub fn resolve_crate_display_name(member: &str, workspace_root: &Path) -> String {
    if !member_path_is_workspace_safe(member) {
        tracing::warn!(
            member = %member,
            "SEC-14 / TASK-1246: rejecting absolute or `..` workspace member in display-name resolver"
        );
        return format_unit_name(member);
    }
    let toml_path = workspace_root.join(member).join("Cargo.toml");
    read_crate_metadata(&toml_path)
        .name
        .unwrap_or_else(|| format_unit_name(member))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_about::test_support::capture_tracing;

    /// ERR-7 (TASK-0977) / TEST-25 (TASK-1773): `read_crate_metadata`'s
    /// breadcrumbs must Debug-format the manifest path so an
    /// attacker-controlled workspace member name with embedded newlines /
    /// ANSI escapes cannot forge log records.
    ///
    /// This drives the crate's own function and asserts on the *captured* log
    /// lines — swapping either `path = ?…` for `path = %…` makes it fail. The
    /// previous shape asserted only that `std`'s `Debug for Path::Display`
    /// escapes control characters, which stayed green through exactly that
    /// regression. Both breadcrumbs are covered: the debug line on a
    /// non-`NotFound` read error and the warn line on a TOML parse error.
    #[cfg(unix)]
    #[test]
    fn crate_metadata_breadcrumbs_debug_escape_control_characters() {
        let dir = tempfile::tempdir().expect("tempdir");
        let hostile = dir.path().join("a\nb\u{1b}[31mc");
        std::fs::create_dir_all(&hostile).unwrap();

        // (a) read failure that is not `NotFound`: the manifest path is a
        // directory, so `read_capped_to_string` errors and the debug
        // breadcrumb fires.
        let as_dir = hostile.join("Cargo.toml");
        std::fs::create_dir_all(&as_dir).unwrap();
        // (b) parse failure: a sibling member with malformed TOML.
        let malformed_dir = dir.path().join("m\nalformed\u{1b}[31m");
        std::fs::create_dir_all(&malformed_dir).unwrap();
        let malformed = malformed_dir.join("Cargo.toml");
        std::fs::write(&malformed, "[package\nname = \"unterminated\n").unwrap();

        let (logs, ()) = capture_tracing(tracing::Level::DEBUG, || {
            assert!(read_crate_metadata(&as_dir).name.is_none());
            assert!(read_crate_metadata(&malformed).name.is_none());
        });

        assert!(
            logs.contains("failed to read crate manifest"),
            "expected the read-failure debug breadcrumb, got: {logs}"
        );
        assert!(
            logs.contains("failed to parse crate manifest as TOML"),
            "expected the parse-failure warn breadcrumb, got: {logs}"
        );
        assert!(
            !logs.contains('\u{1b}'),
            "raw ESC must not reach the log lines: {logs:?}"
        );
        // Only the path fields carry embedded newlines; a `%` formatter would
        // emit them raw and split the log record.
        assert_eq!(
            logs.lines().count(),
            2,
            "each breadcrumb must stay on one line, got: {logs:?}"
        );
        assert!(
            logs.contains("\\u{1b}"),
            "paths must be Debug-escaped in the log lines, got: {logs}"
        );
    }

    #[test]
    fn read_crate_metadata_basic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(
            &path,
            "[package]\nname = \"foo\"\nversion = \"1.0.0\"\ndescription = \"a foo\"\n",
        )
        .unwrap();
        let meta = read_crate_metadata(&path);
        assert_eq!(meta.name.as_deref(), Some("foo"));
        assert_eq!(meta.version.as_deref(), Some("1.0.0"));
        assert_eq!(meta.description.as_deref(), Some("a foo"));
    }

    #[test]
    fn read_crate_metadata_missing() {
        let meta = read_crate_metadata(Path::new("/nonexistent/Cargo.toml"));
        assert!(meta.name.is_none());
        assert!(meta.version.is_none());
        assert!(meta.description.is_none());
    }

    /// TASK-0377 AC#2: a malformed Cargo.toml returns an empty `CrateMetadata`
    /// and should not crash.
    #[test]
    fn read_crate_metadata_malformed_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(&path, "[package\nname = \"unterminated\n").unwrap();
        let meta = read_crate_metadata(&path);
        assert!(meta.name.is_none());
        assert!(meta.version.is_none());
        assert!(meta.description.is_none());
    }

    #[test]
    fn resolve_crate_display_name_with_toml() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("crates/my-lib")).unwrap();
        std::fs::write(
            root.join("crates/my-lib/Cargo.toml"),
            "[package]\nname = \"ops-my-lib\"\n",
        )
        .unwrap();
        assert_eq!(
            resolve_crate_display_name("crates/my-lib", root),
            "ops-my-lib"
        );
    }

    /// FN-1 / TASK-1784 AC #2: the dep-count resolution is reachable without a
    /// live `DuckDB`. All three branches — hit, missing row, and missing
    /// package name — are pinned here.
    #[test]
    fn resolve_dep_count_hits_missing_and_nameless() {
        let mut dep_counts = HashMap::new();
        dep_counts.insert("/ws/crates/foo/Cargo.toml".to_string(), 7);

        assert_eq!(
            resolve_dep_count(
                "crates/foo",
                Some("foo"),
                Path::new("/ws/crates/foo/Cargo.toml"),
                &dep_counts
            ),
            Some(7),
            "a canonical manifest_path present in the map resolves"
        );
        assert_eq!(
            resolve_dep_count(
                "crates/bar",
                Some("bar"),
                Path::new("/ws/crates/bar/Cargo.toml"),
                &dep_counts
            ),
            None,
            "a member with no dep_count row yields None"
        );
        assert_eq!(
            resolve_dep_count(
                "crates/foo",
                None,
                Path::new("/ws/crates/foo/Cargo.toml"),
                &dep_counts
            ),
            None,
            "a member with no package name never reaches the lookup"
        );
    }

    /// PERF-3 / TASK-1570: a non-UTF-8 canonical manifest path skips the
    /// lookup rather than collapsing through `to_string_lossy`.
    #[cfg(unix)]
    #[test]
    fn resolve_dep_count_skips_non_utf8_manifest_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let mut dep_counts = HashMap::new();
        // The lossy rendering of the path below; a `to_string_lossy` regression
        // would key on exactly this.
        let lossy = Path::new(OsStr::from_bytes(b"/ws/n\xC3\x28/Cargo.toml"))
            .to_string_lossy()
            .into_owned();
        dep_counts.insert(lossy, 3);

        let path = Path::new(OsStr::from_bytes(b"/ws/n\xC3\x28/Cargo.toml"));
        assert!(path.to_str().is_none(), "test premise: path is not UTF-8");
        assert_eq!(
            resolve_dep_count("crates/n", Some("n"), path, &dep_counts),
            None,
            "a non-UTF-8 manifest path must not resolve via a lossy key"
        );
    }

    /// SEC-14 / TASK-1246 AC #3: a workspace whose `[workspace].members`
    /// list contains absolute (`/abs`) or `..`-traversal (`../escape`)
    /// entries must produce zero `ProjectUnit`s and not drive any
    /// per-crate manifest read at the hostile location. We pin the
    /// behaviour at the units provider boundary — the AC #2 scrub in
    /// `resolved_workspace_members` plus AC #1 re-validation in
    /// `provide` together guarantee no `root.join(member)` reaches the
    /// adversarial path.
    ///
    /// TEST-19 / TASK-1770: the workspace and the hostile `escape` sibling both
    /// live inside this test's own tempdir (`tmp/ws` and `tmp/escape`), so
    /// `../escape` from the workspace root still resolves to the planted
    /// manifest while nothing is created — or `remove_dir_all`'d — outside the
    /// tempdir's random path. The previous shape anchored `escape` to
    /// `tempdir.parent()` (i.e. `/tmp/escape`), a fixed absolute path shared by
    /// every concurrent run and every developer on the box: two runs raced on
    /// creating and deleting it, and a loser could observe an empty unit list
    /// for the wrong reason — a silently false-green SEC-14 regression test.
    #[test]
    #[serial_test::serial(typed_manifest_cache)]
    fn provide_drops_absolute_and_traversal_members() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("ws");
        std::fs::create_dir_all(&root).unwrap();
        // Plant a hostile manifest at the path `../escape` resolves to from the
        // workspace root, so a regression that lets the entry through surfaces
        // as a non-empty unit list carrying the planted manifest's name.
        let hostile = dir.path().join("escape");
        std::fs::create_dir_all(&hostile).unwrap();
        std::fs::write(
            hostile.join("Cargo.toml"),
            "[package]\nname = \"hostile\"\nversion = \"0.0.0\"\n",
        )
        .unwrap();

        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"../escape\", \"/abs\"]\n",
        )
        .unwrap();

        let mut ctx = Context::test_context(root);
        let v = RustUnitsProvider.provide(&mut ctx).expect("provide");
        let arr = v.as_array().expect("array");
        assert!(
            arr.is_empty(),
            "absolute and `..` members must produce zero units, got: {arr:?}"
        );
    }

    /// PERF-3 / TASK-1251: with the redundant `Vec<&str>` collect +
    /// `sort_unstable` removed, traversal order on subsequent `provide()`
    /// calls must match the ordering invariant declared on
    /// `LoadedManifest::resolved_members`. Pin the sorted-stable order so a
    /// future regression that re-introduces a sort (or a refactor that
    /// changes `resolved_workspace_members` order) is caught here.
    #[test]
    #[serial_test::serial(typed_manifest_cache)]
    fn rust_units_provider_traversal_order_is_stable_across_provides() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for name in ["zeta", "alpha", "mu", "beta"] {
            let crate_dir = root.join(format!("crates/{name}"));
            std::fs::create_dir_all(&crate_dir).unwrap();
            std::fs::write(
                crate_dir.join("Cargo.toml"),
                format!("[package]\nname=\"{name}\"\nversion=\"0.1.0\"\n"),
            )
            .unwrap();
        }
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();

        let mut ctx = Context::test_context(root.to_path_buf());
        let v1 = RustUnitsProvider.provide(&mut ctx).expect("provide1");
        let v2 = RustUnitsProvider.provide(&mut ctx).expect("provide2");

        let order1 = unit_paths(&v1);
        let order2 = unit_paths(&v2);
        assert_eq!(order1, order2, "traversal order must be stable");
        // Sorted order matches the documented invariant.
        let mut sorted = order1.clone();
        sorted.sort();
        assert_eq!(order1, sorted, "must traverse in sorted order");
    }

    fn unit_paths(v: &serde_json::Value) -> Vec<String> {
        v.as_array()
            .expect("array")
            .iter()
            .filter_map(|u| u.get("path").and_then(|p| p.as_str()).map(String::from))
            .collect()
    }

    /// ERR-2 / TASK-1253: two workspace members both named `lib` (legal in
    /// cargo when the parent paths differ) must each resolve to a
    /// `ProjectUnit` rather than colliding on the bare `package.name` key
    /// the previous code used. The provider doesn't fail loudly on a
    /// missing `dep_count` map (`DuckDB` is optional in this provider's
    /// contract), so the assertion here is structural — both units appear
    /// with their correct path metadata even when the names duplicate.
    #[test]
    #[serial_test::serial(typed_manifest_cache)]
    fn rust_units_provider_handles_duplicate_named_members() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for parent in ["a", "b"] {
            let crate_dir = root.join(parent).join("lib");
            std::fs::create_dir_all(&crate_dir).unwrap();
            std::fs::write(
                crate_dir.join("Cargo.toml"),
                "[package]\nname = \"lib\"\nversion = \"0.1.0\"\n",
            )
            .unwrap();
        }
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a/lib\", \"b/lib\"]\n",
        )
        .unwrap();

        let mut ctx = Context::test_context(root.to_path_buf());
        let v = RustUnitsProvider.provide(&mut ctx).expect("provide");
        let arr = v.as_array().expect("array");
        assert_eq!(
            arr.len(),
            2,
            "duplicate-named members must each surface as a distinct unit, got: {arr:?}"
        );
        let paths: std::collections::BTreeSet<&str> = arr
            .iter()
            .filter_map(|u| u.get("path").and_then(|p| p.as_str()))
            .collect();
        assert!(paths.contains("a/lib"));
        assert!(paths.contains("b/lib"));
    }

    /// CL-3 / TASK-1762 AC #4: running the providers from a *subdirectory* of a
    /// glob workspace must produce the same view as running them from the root.
    /// Before the fix the ancestor walk found the root, then every member join
    /// used `ctx.working_directory`, so `crates/*` expanded to nothing and the
    /// units list, `module_count` and per-crate coverage all silently emptied.
    ///
    /// The cache is cleared between the two runs so the equality is produced by
    /// the root-relative resolution rather than by the subdirectory run hitting
    /// the root run's cache entry.
    #[test]
    #[serial_test::serial(typed_manifest_cache)]
    fn providers_agree_between_root_and_subdirectory_cwd() {
        use crate::coverage_provider::RustCoverageProvider;
        use crate::identity::RustIdentityProvider;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for name in ["alpha", "beta", "gamma"] {
            let crate_dir = root.join(format!("crates/{name}/src"));
            std::fs::create_dir_all(&crate_dir).unwrap();
            std::fs::write(
                root.join(format!("crates/{name}/Cargo.toml")),
                format!("[package]\nname=\"{name}\"\nversion=\"0.1.0\"\n"),
            )
            .unwrap();
        }
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();

        let canonical_root = std::fs::canonicalize(root).expect("canonicalize");
        let run = |cwd: std::path::PathBuf| {
            crate::manifest_cache::evict(&canonical_root);
            let mut ctx = Context::test_context(cwd);
            let units = RustUnitsProvider.provide(&mut ctx).expect("units");
            let identity = RustIdentityProvider.provide(&mut ctx).expect("identity");
            let coverage = RustCoverageProvider.provide(&mut ctx).expect("coverage");
            (units, identity, coverage)
        };

        let (root_units, root_identity, root_coverage) = run(root.to_path_buf());
        let (sub_units, sub_identity, sub_coverage) = run(root.join("crates/alpha/src"));

        assert_eq!(
            unit_paths(&root_units),
            vec!["crates/alpha", "crates/beta", "crates/gamma"],
            "premise: the root run sees all three glob members"
        );
        assert_eq!(
            unit_paths(&sub_units),
            unit_paths(&root_units),
            "a subdirectory cwd must resolve the same workspace members"
        );
        assert_eq!(
            sub_identity.get("module_count"),
            root_identity.get("module_count"),
            "module_count must not depend on the cwd"
        );
        assert_eq!(
            root_identity
                .get("module_count")
                .and_then(serde_json::Value::as_u64),
            Some(3),
            "premise: module_count counts the expanded members"
        );
        assert_eq!(
            sub_coverage, root_coverage,
            "coverage must not depend on the cwd"
        );

        crate::manifest_cache::evict(&canonical_root);
    }

    #[test]
    fn resolve_crate_display_name_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            resolve_crate_display_name("crates/nothing", dir.path()),
            "Nothing"
        );
    }
}
