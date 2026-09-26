//! Rust `project_dependencies` data provider.
//!
//! Queries `SQLite` for per-crate direct dependencies via cargo metadata.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use ops_about::deps::{DuplicateCrate, DuplicateReport, OlderVersion, PullingDependency};
use ops_cargo_update::{parse_update_output, UpdateEntry, CARGO_UPDATE_TIMEOUT};
use ops_core::project_identity::{ProjectDependencies, UnitDeps};
use ops_core::subprocess::run_cargo;
use ops_extension::{Context, DataProvider, DataProviderError};
use ops_sqlite::sql::{query_crate_deps, query_or_warn};

pub const PROVIDER_NAME: &str = "project_dependencies";

pub struct RustDepsProvider;

impl DataProvider for RustDepsProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let Some(db) = ops_sqlite::get_db(ctx) else {
            return Ok(serde_json::to_value(ProjectDependencies::default())?);
        };

        // ERR-2 / TASK-0376: a SQLite schema/migration error here used to
        // surface as an empty deps list with no signal. `query_or_warn`
        // routes the failure through tracing::warn before falling back.
        let per_crate = query_or_warn(
            "query_crate_deps",
            "project_dependencies will be empty",
            std::collections::HashMap::<String, Vec<(String, String)>>::new(),
            || query_crate_deps(db),
        );
        let units: Vec<UnitDeps> = per_crate
            .into_iter()
            .map(|(unit_name, deps)| UnitDeps::new(unit_name, deps))
            .collect();

        let result = ProjectDependencies::new(units);
        serde_json::to_value(&result).map_err(DataProviderError::from)
    }
}

/// Registry key of the duplicates provider (re-exported from `ops_about`
/// so the renderer and this provider cannot drift).
pub const DUPLICATES_PROVIDER_NAME: &str = ops_about::deps::PROJECT_DUPLICATES_PROVIDER;

/// Answers `ops about dependencies --duplicates` from the cached `cargo
/// metadata` document (warmed by the runner) plus one `cargo update
/// --dry-run -p <dep>@<version>` per pulling dependency. `--dry-run` never
/// writes `Cargo.lock`.
pub struct RustDuplicatesProvider;

impl DataProvider for RustDuplicatesProvider {
    fn name(&self) -> &'static str {
        DUPLICATES_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let Some(metadata) = ctx.cached("metadata").cloned() else {
            return Ok(serde_json::to_value(DuplicateReport::default())?);
        };
        let working_dir = ctx.working_directory().to_path_buf();
        let report = find_duplicates(&metadata, &|name: &str, version: &str| {
            dry_run_update(&working_dir, name, version)
        });
        serde_json::to_value(&report).map_err(DataProviderError::from)
    }
}

/// `cargo update --dry-run -p <name>@<version>`, parsed; `None` (with a
/// warning) when it cannot run or exits non-zero.
fn dry_run_update(working_dir: &Path, name: &str, version: &str) -> Option<Vec<UpdateEntry>> {
    let spec = format!("{name}@{version}");
    let output = run_cargo(
        &["update", "--dry-run", "-p", &spec],
        working_dir,
        CARGO_UPDATE_TIMEOUT,
        "cargo update --dry-run -p",
    );
    match output {
        Ok(out) if out.status.success() => Some(parse_update_output(&out.stderr).entries),
        Ok(out) => {
            tracing::warn!(spec, status = %out.status, "about/duplicates: cargo update --dry-run failed");
            None
        }
        Err(e) => {
            tracing::warn!(spec, error = %e, "about/duplicates: cargo update --dry-run failed");
            None
        }
    }
}

/// `(name, version)` of each package id in `cargo metadata`'s `packages`.
fn package_index(metadata: &serde_json::Value) -> HashMap<&str, (&str, &str)> {
    metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| {
            Some((
                p.get("id")?.as_str()?,
                (p.get("name")?.as_str()?, p.get("version")?.as_str()?),
            ))
        })
        .collect()
}

/// Non-dev edges of the resolve graph: an edge counts when any of its
/// `dep_kinds` is normal (`null`) or `build`, or when kinds are absent.
fn non_dev_edges(metadata: &serde_json::Value) -> HashMap<&str, Vec<&str>> {
    let nodes = metadata
        .pointer("/resolve/nodes")
        .and_then(serde_json::Value::as_array);
    nodes
        .into_iter()
        .flatten()
        .filter_map(|node| {
            let id = node.get("id")?.as_str()?;
            let deps = node
                .get("deps")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter(|d| {
                    d.get("dep_kinds")
                        .and_then(serde_json::Value::as_array)
                        .is_none_or(|kinds| {
                            kinds.is_empty()
                                || kinds.iter().any(|k| {
                                    k.get("kind").is_none_or(|kind| {
                                        kind.is_null() || kind.as_str() == Some("build")
                                    })
                                })
                        })
                })
                .filter_map(|d| d.get("pkg")?.as_str())
                .collect();
            Some((id, deps))
        })
        .collect()
}

fn closure<'a>(start: &[&'a str], edges: &HashMap<&'a str, Vec<&'a str>>) -> BTreeSet<&'a str> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut stack: Vec<&str> = start.to_vec();
    while let Some(id) = stack.pop() {
        if seen.insert(id) {
            stack.extend(edges.get(id).into_iter().flatten().copied());
        }
    }
    seen
}

/// Semver-ish sort key: numeric `major.minor.patch`, then release before
/// pre-release, then the pre-release text.
fn version_key(version: &str) -> (Vec<u64>, bool, String) {
    let core = version.split('+').next().unwrap_or(version);
    let (numbers, pre) = core.split_once('-').unwrap_or((core, ""));
    let nums = numbers.split('.').map(|n| n.parse().unwrap_or(0)).collect();
    (nums, pre.is_empty(), pre.to_string())
}

/// Find crates reachable at two or more distinct versions through
/// non-dev edges from the workspace members, name the direct dependencies
/// pulling each older version, and ask `updater` whether updating each
/// puller drops it.
///
/// `updater(name, version)` returns the dry-run's lockfile changes, or
/// `None` when the check could not run; it is called once per distinct
/// puller.
pub fn find_duplicates(
    metadata: &serde_json::Value,
    updater: &dyn Fn(&str, &str) -> Option<Vec<UpdateEntry>>,
) -> DuplicateReport {
    let packages = package_index(metadata);
    let edges = non_dev_edges(metadata);
    let members: BTreeSet<&str> = metadata
        .get("workspace_members")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect();
    let member_list: Vec<&str> = members.iter().copied().collect();
    let reachable = closure(&member_list, &edges);

    let mut versions_by_name: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for id in reachable.iter().filter(|id| !members.contains(*id)) {
        if let Some((name, version)) = packages.get(id) {
            versions_by_name.entry(name).or_default().insert(version);
        }
    }

    let direct: BTreeSet<&str> = member_list
        .iter()
        .flat_map(|m| edges.get(m).into_iter().flatten().copied())
        .filter(|id| !members.contains(id))
        .collect();
    let direct_closures: Vec<(&str, BTreeSet<(&str, &str)>)> = direct
        .iter()
        .map(|d| {
            let reached = closure(&[d], &edges)
                .into_iter()
                .filter_map(|id| packages.get(id).copied())
                .collect();
            (*d, reached)
        })
        .collect();

    let mut dry_runs: HashMap<&str, Option<Vec<UpdateEntry>>> = HashMap::new();
    let mut crates = Vec::new();
    for (name, versions) in versions_by_name.into_iter().filter(|(_, v)| v.len() >= 2) {
        let mut sorted: Vec<&str> = versions.into_iter().collect();
        sorted.sort_by_key(|v| version_key(v));
        let newest_others =
            |older: &str| -> Vec<&str> { sorted.iter().copied().filter(|v| *v != older).collect() };
        let older_versions: Vec<&str> = sorted
            .split_last()
            .map(|(_, rest)| rest.to_vec())
            .unwrap_or_default();
        let mut older = Vec::new();
        for version in older_versions {
            let others = newest_others(version);
            let mut pulled_by = Vec::new();
            for (puller_id, reached) in &direct_closures {
                if !reached.contains(&(name, version)) {
                    continue;
                }
                let Some(&(puller_name, puller_version)) = packages.get(puller_id) else {
                    continue;
                };
                let entries = dry_runs
                    .entry(puller_id)
                    .or_insert_with(|| updater(puller_name, puller_version));
                let update_removes_duplicate = entries
                    .as_ref()
                    .map(|e| update_drops_version(e, name, version, &others));
                pulled_by.push(PullingDependency {
                    name: puller_name.to_string(),
                    version: puller_version.to_string(),
                    update_removes_duplicate,
                });
            }
            older.push(OlderVersion {
                version: version.to_string(),
                pulled_by,
            });
        }
        crates.push(DuplicateCrate {
            name: name.to_string(),
            versions: sorted.iter().map(ToString::to_string).collect(),
            older,
        });
    }
    DuplicateReport { crates }
}

/// Whether the dry-run `entries` remove `name@version` outright, or move it
/// onto one of the crate's other locked versions.
fn update_drops_version(
    entries: &[UpdateEntry],
    name: &str,
    version: &str,
    others: &[&str],
) -> bool {
    entries.iter().any(|e| {
        e.name() == name
            && e.from() == Some(version)
            && e.to().is_none_or(|to| others.contains(&to))
    })
}

#[cfg(test)]
mod tests {
    use super::{RustDepsProvider, PROVIDER_NAME};
    use ops_about::test_support::capture_tracing;
    use ops_extension::{Context, DataProvider};
    use ops_sqlite::Sqlite;
    use std::sync::Arc;

    /// TEST-5 / TASK-1776 AC #2.
    #[test]
    fn provider_name_matches_the_registered_constant() {
        assert_eq!(RustDepsProvider.name(), PROVIDER_NAME);
        assert_eq!(PROVIDER_NAME, "project_dependencies");
    }

    fn provide_with(db: Option<Sqlite>) -> (serde_json::Value, String) {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut ctx = Context::test_context(dir.path().to_path_buf());
        if let Some(db) = db {
            ctx.attach_db(Arc::new(db));
        }
        let (logs, value) = capture_tracing(tracing::Level::WARN, || {
            RustDepsProvider.provide(&mut ctx).expect("provide")
        });
        (value, logs)
    }

    /// TEST-5 / TASK-1776 AC #1: no `SQLite` in the context serialises a
    /// default (empty but well-formed) `ProjectDependencies`, not an error.
    #[test]
    fn provide_without_sqlite_yields_empty_dependencies() {
        let (value, logs) = provide_with(None);
        assert_eq!(
            value.get("units").and_then(|u| u.as_array()).map(Vec::len),
            Some(0),
            "expected an empty units list, got: {value}"
        );
        assert!(
            logs.is_empty(),
            "an absent SQLite is not a degraded mode; no warn expected, got: {logs}"
        );
    }

    /// TEST-5 / TASK-1776 AC #1: the ERR-2 / TASK-0376 contract — a `SQLite`
    /// schema/migration error must warn before falling back, not surface as a
    /// silently empty deps list. The seeded `crate_dependencies` table is
    /// missing the `dependency_name`, `version_req` and `dependency_kind`
    /// columns the query selects, so the prepare fails while `table_exists`
    /// still passes.
    #[test]
    fn provide_warns_and_falls_back_when_the_query_fails() {
        let db = Sqlite::open_in_memory().expect("open in-memory db");
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE crate_dependencies (crate_name VARCHAR); \
                 INSERT INTO crate_dependencies VALUES ('a');",
            )
            .expect("seed broken-schema crate_dependencies");
        }
        let (value, logs) = provide_with(Some(db));
        assert_eq!(
            value.get("units").and_then(|u| u.as_array()).map(Vec::len),
            Some(0),
            "the fallback must be a valid empty ProjectDependencies, got: {value}"
        );
        assert!(
            logs.contains("query=\"query_crate_deps\""),
            "the failure must warn before falling back, got: {logs}"
        );
    }

    /// TEST-5 / TASK-1776 AC #1: a successful multi-crate result is mapped
    /// into one `UnitDeps` per crate, carrying `(name, version_req)` pairs.
    /// Only `dependency_kind = 'normal'` rows are included.
    #[test]
    fn provide_maps_multi_crate_rows_into_unit_deps() {
        let db = Sqlite::open_in_memory().expect("open in-memory db");
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE crate_dependencies (\
                    crate_name VARCHAR, \
                    dependency_name VARCHAR, \
                    version_req VARCHAR, \
                    dependency_kind VARCHAR\
                 ); \
                 INSERT INTO crate_dependencies VALUES \
                    ('alpha', 'serde', '1.0', 'normal'), \
                    ('alpha', 'anyhow', '1.0', 'normal'), \
                    ('beta', 'tracing', '0.1', 'normal'), \
                    ('beta', 'tempfile', '3', 'dev');",
            )
            .expect("seed crate_dependencies");
        }
        let (value, logs) = provide_with(Some(db));
        assert!(logs.is_empty(), "a healthy query must not warn: {logs}");

        let units = value
            .get("units")
            .and_then(|u| u.as_array())
            .expect("units");
        assert_eq!(units.len(), 2, "one UnitDeps per crate, got: {value}");
        let by_name: std::collections::BTreeMap<&str, Vec<&str>> = units
            .iter()
            .map(|u| {
                let name = u
                    .get("unit_name")
                    .and_then(|n| n.as_str())
                    .expect("unit name");
                let deps = u
                    .get("deps")
                    .and_then(|d| d.as_array())
                    .expect("deps array")
                    .iter()
                    .filter_map(|d| d.get(0).and_then(|n| n.as_str()))
                    .collect();
                (name, deps)
            })
            .collect();
        assert_eq!(by_name.get("alpha"), Some(&vec!["anyhow", "serde"]));
        assert_eq!(
            by_name.get("beta"),
            Some(&vec!["tracing"]),
            "dev-dependencies must be excluded"
        );
    }

    /// A workspace `app` depending on `table` (which pulls `term 0.28`) and
    /// directly on `term 0.29`, plus a dev-only `mock` pulling `old 1.0`
    /// next to `old 2.0` — only `term` is a real (non-dev) duplicate.
    fn sample_metadata() -> serde_json::Value {
        serde_json::json!({
            "packages": [
                {"id": "app", "name": "app", "version": "0.1.0"},
                {"id": "table", "name": "table", "version": "7.1.4"},
                {"id": "term28", "name": "term", "version": "0.28.1"},
                {"id": "term29", "name": "term", "version": "0.29.0"},
                {"id": "mock", "name": "mock", "version": "1.0.0"},
                {"id": "old1", "name": "old", "version": "1.0.0"},
                {"id": "old2", "name": "old", "version": "2.0.0"}
            ],
            "workspace_members": ["app"],
            "resolve": {"nodes": [
                {"id": "app", "deps": [
                    {"pkg": "table", "dep_kinds": [{"kind": null}]},
                    {"pkg": "term29", "dep_kinds": [{"kind": null}]},
                    {"pkg": "old2", "dep_kinds": [{"kind": null}]},
                    {"pkg": "mock", "dep_kinds": [{"kind": "dev"}]}
                ]},
                {"id": "table", "deps": [{"pkg": "term28", "dep_kinds": [{"kind": null}]}]},
                {"id": "mock", "deps": [{"pkg": "old1", "dep_kinds": [{"kind": null}]}]},
                {"id": "term28", "deps": []},
                {"id": "term29", "deps": []},
                {"id": "old1", "deps": []},
                {"id": "old2", "deps": []}
            ]}
        })
    }

    /// TASK-2288 AC #1/#2: only distinct-version, non-dev duplicates are
    /// listed; the older version names its pulling direct dependency and
    /// the dry-run verdict.
    #[test]
    fn find_duplicates_lists_real_duplicates_with_puller_and_verdict() {
        use ops_cargo_update::parse_update_output;
        let calls = std::cell::RefCell::new(Vec::new());
        let updater = |name: &str, version: &str| {
            calls.borrow_mut().push(format!("{name}@{version}"));
            Some(
                parse_update_output(
                    b"    Updating table v7.1.4 -> v7.2.2\n    Removing term v0.28.1\n",
                )
                .entries,
            )
        };
        let report = super::find_duplicates(&sample_metadata(), &updater);
        let value = serde_json::to_value(&report).expect("serialize");
        assert_eq!(
            value,
            serde_json::json!({"crates": [{
                "name": "term",
                "versions": ["0.28.1", "0.29.0"],
                "older": [{"version": "0.28.1", "pulledBy": [
                    {"name": "table", "version": "7.1.4", "updateRemovesDuplicate": true}
                ]}]
            }]})
        );
        assert_eq!(*calls.borrow(), ["table@7.1.4"], "one dry run per puller");
    }

    #[test]
    fn find_duplicates_reports_unfixable_and_failed_checks() {
        let no_change = |_: &str, _: &str| Some(Vec::new());
        let report = super::find_duplicates(&sample_metadata(), &no_change);
        assert_eq!(
            report.crates[0].older[0].pulled_by[0].update_removes_duplicate,
            Some(false)
        );
        let failed = |_: &str, _: &str| None;
        let report = super::find_duplicates(&sample_metadata(), &failed);
        assert_eq!(
            report.crates[0].older[0].pulled_by[0].update_removes_duplicate,
            None
        );
    }

    #[test]
    fn version_key_orders_numerically_and_pre_release_first() {
        let mut v = vec!["0.10.0", "0.9.1", "1.0.0", "1.0.0-rc.1"];
        v.sort_by_key(|s| super::version_key(s));
        assert_eq!(v, ["0.9.1", "0.10.0", "1.0.0-rc.1", "1.0.0"]);
    }

    #[test]
    fn duplicates_provider_without_metadata_is_empty() {
        use super::RustDuplicatesProvider;
        let dir = tempfile::tempdir().expect("tempdir");
        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let value = RustDuplicatesProvider.provide(&mut ctx).expect("provide");
        assert_eq!(value, serde_json::json!({"crates": []}));
    }
}
