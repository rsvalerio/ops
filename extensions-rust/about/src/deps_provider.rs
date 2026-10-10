//! Rust `project_dependencies` data provider.
//!
//! Queries `SQLite` for per-crate direct dependencies via cargo metadata.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use ops_about::deps::{
    DuplicateCrate, DuplicateReport, OlderVersion, PullingDependency, DUPLICATES_ALL_TARGETS,
    DUPLICATES_TARGET_ARG,
};
use ops_about::machine::{cfg_matches, parse_rustc_cfg, parse_rustc_host};
use ops_cargo_update::{parse_update_output, UpdateEntry, CARGO_UPDATE_TIMEOUT};
use ops_core::project_identity::{ProjectDependencies, UnitDeps};
use ops_core::subprocess::{run_cargo, run_with_timeout};
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

        // `query_or_warn` routes a SQLite schema/migration failure through
        // tracing::warn before falling back to an empty deps list.
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

/// Registry key of the dev-inclusive duplicates provider
/// (`--duplicates --include-dev`).
pub const DUPLICATES_WITH_DEV_PROVIDER_NAME: &str =
    ops_about::deps::PROJECT_DUPLICATES_WITH_DEV_PROVIDER;

/// Answers `ops about dependencies --duplicates` from the cached `cargo
/// metadata` document (warmed by the runner) plus one `cargo update
/// --dry-run -p <dep>@<version>` per pulling dependency. `--dry-run` never
/// writes `Cargo.lock`.
///
/// Registered twice: `include_dev: false` under [`DUPLICATES_PROVIDER_NAME`]
/// (dev-only duplicates excluded) and `include_dev: true` under
/// [`DUPLICATES_WITH_DEV_PROVIDER_NAME`].
pub struct RustDuplicatesProvider {
    /// Follow dev-dependency edges too.
    pub include_dev: bool,
}

impl DataProvider for RustDuplicatesProvider {
    fn name(&self) -> &'static str {
        if self.include_dev {
            DUPLICATES_WITH_DEV_PROVIDER_NAME
        } else {
            DUPLICATES_PROVIDER_NAME
        }
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let Some(metadata) = ctx.cached("metadata").cloned() else {
            return Ok(serde_json::to_value(DuplicateReport::default())?);
        };
        let working_dir = ctx.working_directory().to_path_buf();
        let platforms = platform_filter(ctx.arg(DUPLICATES_TARGET_ARG), &working_dir)?;
        let report = find_duplicates(
            &metadata,
            self.include_dev,
            &platforms,
            &|name: &str, version: &str| dry_run_update(&working_dir, name, version),
        );
        serde_json::to_value(&report).map_err(DataProviderError::from)
    }
}

/// Deadline for each `rustc -vV` / `rustc --print cfg` probe.
const RUSTC_PROBE_TIMEOUT: Duration = Duration::from_secs(30);

/// A target the platform filter admits dependency edges for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetPlatform {
    /// The triple, matched against edges gated on a bare triple.
    pub triple: String,
    /// `rustc --print cfg --target <triple>` atoms, for `cfg(..)` edges.
    pub cfg: Vec<String>,
}

impl TargetPlatform {
    /// Whether an edge gated on `gate` (a triple or a `cfg(..)`
    /// expression) is active on this platform.
    fn admits(&self, gate: &str) -> bool {
        if gate.trim_start().starts_with("cfg(") {
            cfg_matches(gate, &self.cfg)
        } else {
            gate.trim() == self.triple
        }
    }
}

/// Which platform a crate is compiled for. Build scripts' dependencies,
/// proc-macros and everything beneath either run on the host, whatever
/// `--target` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Side {
    Target,
    Host,
}

/// Which dependency edges count, by the platform they are gated on — the
/// `cargo tree --target` rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformFilter {
    /// Every edge, whatever platform it is gated on (`--target all`).
    All,
    /// Ungated edges, plus gated edges active on any of `targets` — or, for
    /// an edge compiled for the host (beneath a build-dependency or a
    /// proc-macro), active on `host`.
    Targets {
        targets: Vec<TargetPlatform>,
        host: TargetPlatform,
    },
}

impl PlatformFilter {
    /// Whether an edge gated on `target` (a `dep_kinds[].target` from
    /// `cargo metadata`: a triple, a `cfg(..)` expression, or absent)
    /// is active under this filter for a crate compiled on `side`.
    fn admits(&self, target: Option<&str>, side: Side) -> bool {
        match (self, target) {
            (Self::All, _) | (Self::Targets { .. }, None) => true,
            (Self::Targets { targets, host }, Some(gate)) => match side {
                Side::Target => targets.iter().any(|p| p.admits(gate)),
                Side::Host => host.admits(gate),
            },
        }
    }
}

/// Resolve the `--target` context argument: absent → the host triple
/// (`rustc -vV`), any `all` → [`PlatformFilter::All`], otherwise each
/// comma-separated triple with its `rustc --print cfg --target` atoms. The
/// host is always probed too: host-compiled edges are matched against it.
///
/// `rustc` is `$RUSTC` or `rustc` on PATH, run in `working_dir` so a
/// `rust-toolchain.toml` there picks the project's toolchain.
///
/// # Errors
///
/// [`DataProviderError::ComputationMessage`] when a rustc probe fails —
/// including an unknown triple — rather than silently widening the report
/// to every platform.
fn platform_filter(
    arg: Option<&str>,
    working_dir: &Path,
) -> Result<PlatformFilter, DataProviderError> {
    let requested: Vec<&str> = arg
        .into_iter()
        .flat_map(|a| a.split(','))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    if requested.contains(&DUPLICATES_ALL_TARGETS) {
        return Ok(PlatformFilter::All);
    }
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let probe = |triple: String| -> Result<TargetPlatform, DataProviderError> {
        let cfg = rustc_stdout(
            &rustc,
            &["--print", "cfg", "--target", &triple],
            working_dir,
        )?;
        Ok(TargetPlatform {
            cfg: parse_rustc_cfg(&cfg),
            triple,
        })
    };
    let host_triple =
        parse_rustc_host(&rustc_stdout(&rustc, &["-vV"], working_dir)?).ok_or_else(|| {
            DataProviderError::computation_failed("`rustc -vV` printed no host triple")
        })?;
    let host = probe(host_triple)?;
    let targets = if requested.is_empty() {
        vec![host.clone()]
    } else {
        requested
            .into_iter()
            .map(|triple| {
                if triple == host.triple {
                    Ok(host.clone())
                } else {
                    probe(triple.to_string())
                }
            })
            .collect::<Result<_, _>>()?
    };
    Ok(PlatformFilter::Targets { targets, host })
}

/// Stdout of `rustc <args>`, or an error naming the command and its stderr.
fn rustc_stdout(
    rustc: &OsStr,
    args: &[&str],
    working_dir: &Path,
) -> Result<String, DataProviderError> {
    let label = format!("rustc {}", args.join(" "));
    let mut cmd = Command::new(rustc);
    cmd.args(args).current_dir(working_dir);
    match run_with_timeout(&mut cmd, RUSTC_PROBE_TIMEOUT, &label) {
        Ok(out) if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        Ok(out) => Err(DataProviderError::computation_failed(format!(
            "`{label}` failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ))),
        Err(e) => Err(DataProviderError::computation_failed(format!(
            "`{label}` failed: {e}"
        ))),
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

/// Package ids with a `proc-macro` target: compiled for the host.
fn proc_macros(metadata: &serde_json::Value) -> HashSet<&str> {
    metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|p| {
            p.get("targets")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|t| t.get("kind")?.as_array())
                .flatten()
                .any(|k| k.as_str() == Some("proc-macro"))
        })
        .filter_map(|p| p.get("id")?.as_str())
        .collect()
}

/// A package in the resolve graph, on the side it is compiled for.
type Unit<'a> = (&'a str, Side);

/// Where one `resolve.nodes[].deps[]` entry leads from a crate compiled on
/// `side`: nowhere when none of its `dep_kinds` is both of a counted kind
/// (every kind with `include_dev`, otherwise normal (`null`) or `build`)
/// and gated on a platform `platforms` admits for `side`; absent kinds
/// count as one ungated normal edge. A `build` edge, a proc-macro and
/// anything already on the host lead to the host side.
fn edge_sides(
    dep: &serde_json::Value,
    side: Side,
    include_dev: bool,
    platforms: &PlatformFilter,
    host_compiled: bool,
) -> BTreeSet<Side> {
    let ungated = [serde_json::Value::Null];
    let kinds = dep
        .get("dep_kinds")
        .and_then(serde_json::Value::as_array)
        .filter(|kinds| !kinds.is_empty())
        .map_or(&ungated[..], Vec::as_slice);
    kinds
        .iter()
        .filter_map(|k| {
            let kind = k.get("kind").and_then(serde_json::Value::as_str);
            let is_build = kind == Some("build");
            let kind_counts = include_dev || kind.is_none() || is_build;
            let admitted =
                platforms.admits(k.get("target").and_then(serde_json::Value::as_str), side);
            (kind_counts && admitted).then_some(
                if side == Side::Host || is_build || host_compiled {
                    Side::Host
                } else {
                    Side::Target
                },
            )
        })
        .collect()
}

/// Edges of the resolve graph that count under [`edge_sides`], from each
/// package on each side.
fn resolve_edges<'a>(
    metadata: &'a serde_json::Value,
    include_dev: bool,
    platforms: &PlatformFilter,
) -> HashMap<Unit<'a>, Vec<Unit<'a>>> {
    let host_compiled = proc_macros(metadata);
    let nodes = metadata
        .pointer("/resolve/nodes")
        .and_then(serde_json::Value::as_array);
    let mut edges = HashMap::new();
    for node in nodes.into_iter().flatten() {
        let Some(id) = node.get("id").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let deps = node.get("deps").and_then(serde_json::Value::as_array);
        for side in [Side::Target, Side::Host] {
            let next: Vec<Unit<'a>> = deps
                .into_iter()
                .flatten()
                .filter_map(|d| Some((d, d.get("pkg")?.as_str()?)))
                .flat_map(|(d, pkg)| {
                    edge_sides(d, side, include_dev, platforms, host_compiled.contains(pkg))
                        .into_iter()
                        .map(move |next| (pkg, next))
                })
                .collect();
            edges.insert((id, side), next);
        }
    }
    edges
}

/// Package ids reachable from `start`, on whichever side.
fn closure<'a>(start: &[Unit<'a>], edges: &HashMap<Unit<'a>, Vec<Unit<'a>>>) -> BTreeSet<&'a str> {
    let mut seen: BTreeSet<Unit<'a>> = BTreeSet::new();
    let mut stack: Vec<Unit<'a>> = start.to_vec();
    while let Some(unit) = stack.pop() {
        if seen.insert(unit) {
            stack.extend(edges.get(&unit).into_iter().flatten().copied());
        }
    }
    seen.into_iter().map(|(id, _)| id).collect()
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
/// non-dev edges (every edge when `include_dev`) active under `platforms`
/// from the workspace members, name the direct dependencies pulling each
/// older version, and ask `updater` whether updating each puller drops it.
///
/// Reachability and pullers are computed over the same edge set, so every
/// listed older version has at least one puller — including one reached
/// only through a target-gated edge. Edges beneath a build-dependency or a
/// proc-macro are gated against the host, as `cargo tree --target` does.
///
/// `updater(name, version)` returns the dry-run's lockfile changes, or
/// `None` when the check could not run; it is called once per distinct
/// puller.
pub fn find_duplicates(
    metadata: &serde_json::Value,
    include_dev: bool,
    platforms: &PlatformFilter,
    updater: &dyn Fn(&str, &str) -> Option<Vec<UpdateEntry>>,
) -> DuplicateReport {
    let packages = package_index(metadata);
    let edges = resolve_edges(metadata, include_dev, platforms);
    let members = workspace_member_ids(metadata);
    let member_units: Vec<Unit<'_>> = members.iter().map(|m| (*m, Side::Target)).collect();
    let reachable = closure(&member_units, &edges);

    let mut pullers = Pullers {
        direct_closures: direct_closures(&member_units, &members, &edges, &packages),
        packages: &packages,
        updater,
        dry_runs: HashMap::new(),
    };
    let crates = versions_by_name(&reachable, &members, &packages)
        .into_iter()
        .filter(|(_, versions)| versions.len() >= 2)
        .map(|(name, versions)| pullers.duplicate_crate(name, versions))
        .collect();
    DuplicateReport { crates }
}

/// `(name, version)` of a package.
type Package<'a> = (&'a str, &'a str);

/// Package ids of the workspace members.
fn workspace_member_ids(metadata: &serde_json::Value) -> BTreeSet<&str> {
    metadata
        .get("workspace_members")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect()
}

/// The distinct versions of each non-member crate among `reachable`.
fn versions_by_name<'a>(
    reachable: &BTreeSet<&'a str>,
    members: &BTreeSet<&'a str>,
    packages: &HashMap<&'a str, Package<'a>>,
) -> BTreeMap<&'a str, BTreeSet<&'a str>> {
    let mut versions: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for id in reachable.iter().filter(|id| !members.contains(*id)) {
        if let Some((name, version)) = packages.get(id) {
            versions.entry(name).or_default().insert(version);
        }
    }
    versions
}

/// Each direct (non-member) dependency of the workspace members, by package
/// id, with every package reachable from it.
fn direct_closures<'a>(
    member_units: &[Unit<'a>],
    members: &BTreeSet<&'a str>,
    edges: &HashMap<Unit<'a>, Vec<Unit<'a>>>,
    packages: &HashMap<&'a str, Package<'a>>,
) -> Vec<(&'a str, BTreeSet<Package<'a>>)> {
    let mut direct: BTreeMap<&str, Vec<Unit<'_>>> = BTreeMap::new();
    for unit in member_units
        .iter()
        .flat_map(|m| edges.get(m).into_iter().flatten().copied())
        .filter(|(id, _)| !members.contains(id))
    {
        direct.entry(unit.0).or_default().push(unit);
    }
    direct
        .iter()
        .map(|(id, units)| {
            let reached = closure(units, edges)
                .into_iter()
                .filter_map(|id| packages.get(id).copied())
                .collect();
            (*id, reached)
        })
        .collect()
}

/// Names the direct dependencies pulling a duplicated crate's older versions,
/// running `updater` at most once per distinct puller.
struct Pullers<'a, 'u> {
    direct_closures: Vec<(&'a str, BTreeSet<Package<'a>>)>,
    packages: &'u HashMap<&'a str, Package<'a>>,
    updater: &'u dyn Fn(&str, &str) -> Option<Vec<UpdateEntry>>,
    /// Dry-run result per puller package id; `None` when the check could not
    /// run.
    dry_runs: HashMap<&'a str, Option<Vec<UpdateEntry>>>,
}

impl<'a> Pullers<'a, '_> {
    /// The report entry for `name`, locked at two or more `versions`: every
    /// version but the newest is listed with its pullers.
    fn duplicate_crate(&mut self, name: &'a str, versions: BTreeSet<&'a str>) -> DuplicateCrate {
        let mut sorted: Vec<&str> = versions.into_iter().collect();
        sorted.sort_by_key(|v| version_key(v));
        let older_versions = sorted.split_last().map_or(&[][..], |(_, rest)| rest);
        let older = older_versions
            .iter()
            .map(|&version| {
                let others: Vec<&str> = sorted.iter().copied().filter(|v| *v != version).collect();
                OlderVersion {
                    version: version.to_string(),
                    pulled_by: self.pullers_of(name, version, &others),
                }
            })
            .collect();
        DuplicateCrate {
            name: name.to_string(),
            versions: sorted.iter().map(ToString::to_string).collect(),
            older,
        }
    }

    /// The direct dependencies whose closure reaches `name@version`, each with
    /// whether updating it drops that version in favour of one of `others`.
    fn pullers_of(&mut self, name: &str, version: &str, others: &[&str]) -> Vec<PullingDependency> {
        let mut pulled_by = Vec::new();
        for (puller_id, reached) in &self.direct_closures {
            if !reached.contains(&(name, version)) {
                continue;
            }
            let Some(&(puller_name, puller_version)) = self.packages.get(puller_id) else {
                continue;
            };
            let entries = self
                .dry_runs
                .entry(puller_id)
                .or_insert_with(|| (self.updater)(puller_name, puller_version));
            let update_removes_duplicate = entries
                .as_ref()
                .map(|e| update_drops_version(e, name, version, others));
            pulled_by.push(PullingDependency {
                name: puller_name.to_string(),
                version: puller_version.to_string(),
                update_removes_duplicate,
            });
        }
        pulled_by
    }
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
        let report = super::find_duplicates(
            &sample_metadata(),
            false,
            &super::PlatformFilter::All,
            &updater,
        );
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
        let report = super::find_duplicates(
            &sample_metadata(),
            false,
            &super::PlatformFilter::All,
            &no_change,
        );
        assert_eq!(
            report.crates[0].older[0].pulled_by[0].update_removes_duplicate,
            Some(false)
        );
        let failed = |_: &str, _: &str| None;
        let report = super::find_duplicates(
            &sample_metadata(),
            false,
            &super::PlatformFilter::All,
            &failed,
        );
        assert_eq!(
            report.crates[0].older[0].pulled_by[0].update_removes_duplicate,
            None
        );
    }

    /// TASK-2299 AC #2: with `include_dev` the dev-only `old` duplicate
    /// (pulled by the dev-dependency `mock`) is listed next to `term`.
    #[test]
    fn find_duplicates_include_dev_lists_dev_only_duplicates() {
        let no_change = |_: &str, _: &str| Some(Vec::new());
        let report = super::find_duplicates(
            &sample_metadata(),
            true,
            &super::PlatformFilter::All,
            &no_change,
        );
        let names: Vec<&str> = report.crates.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["old", "term"]);
        let old = &report.crates[0];
        assert_eq!(old.older[0].version, "1.0.0");
        assert_eq!(old.older[0].pulled_by[0].name, "mock");
    }

    /// A workspace `app` with `term` duplicated on every platform, plus a
    /// `sys` crate duplicated only on Windows: `sys 1.0.0` through a
    /// `cfg(windows)`-gated `gate`, `sys 2.0.0` through an edge gated on the
    /// bare `x86_64-pc-windows-msvc` triple.
    fn gated_metadata() -> serde_json::Value {
        let mut metadata = sample_metadata();
        let packages = metadata["packages"].as_array_mut().expect("packages");
        packages.extend([
            serde_json::json!({"id": "gate", "name": "gate", "version": "0.3.0"}),
            serde_json::json!({"id": "sys1", "name": "sys", "version": "1.0.0"}),
            serde_json::json!({"id": "sys2", "name": "sys", "version": "2.0.0"}),
        ]);
        let nodes = metadata["resolve"]["nodes"].as_array_mut().expect("nodes");
        nodes[0]["deps"].as_array_mut().expect("app deps").extend([
            serde_json::json!({"pkg": "gate", "dep_kinds": [{"kind": null, "target": "cfg(windows)"}]}),
            serde_json::json!({"pkg": "sys2", "dep_kinds": [{"kind": null, "target": "x86_64-pc-windows-msvc"}]}),
        ]);
        nodes.extend([
            serde_json::json!({"id": "gate", "deps": [{"pkg": "sys1", "dep_kinds": [{"kind": null}]}]}),
            serde_json::json!({"id": "sys1", "deps": []}),
            serde_json::json!({"id": "sys2", "deps": []}),
        ]);
        metadata
    }

    fn platform(triple: &str, cfg: &[&str]) -> super::TargetPlatform {
        super::TargetPlatform {
            triple: triple.to_string(),
            cfg: cfg.iter().map(ToString::to_string).collect(),
        }
    }

    /// A filter for one target that is also the host.
    fn only(p: super::TargetPlatform) -> super::PlatformFilter {
        super::PlatformFilter::Targets {
            targets: vec![p.clone()],
            host: p,
        }
    }

    fn duplicate_names(platforms: &super::PlatformFilter) -> Vec<String> {
        names_in(&gated_metadata(), platforms)
    }

    fn names_in(metadata: &serde_json::Value, platforms: &super::PlatformFilter) -> Vec<String> {
        let no_change = |_: &str, _: &str| Some(Vec::new());
        super::find_duplicates(metadata, false, platforms, &no_change)
            .crates
            .into_iter()
            .map(|c| c.name)
            .collect()
    }

    /// `sample_metadata` plus host-compiled paths to a `unix`-only crate:
    /// `app` build-depends on `cc`, and depends on the proc-macro `derive`;
    /// each reaches `jobs 1.0.0` through a `cfg(unix)` edge, next to an
    /// ungated `jobs 2.0.0`. Beneath `cc` a `cfg(windows)` edge reaches
    /// `winjobs 1.0.0`, next to an ungated `winjobs 2.0.0`.
    fn host_metadata(via: &str) -> serde_json::Value {
        let mut metadata = sample_metadata();
        let packages = metadata["packages"].as_array_mut().expect("packages");
        packages.extend([
            serde_json::json!({"id": "cc", "name": "cc", "version": "1.2.0"}),
            serde_json::json!({"id": "derive", "name": "derive", "version": "1.0.0",
                "targets": [{"kind": ["proc-macro"]}]}),
            serde_json::json!({"id": "jobs1", "name": "jobs", "version": "1.0.0"}),
            serde_json::json!({"id": "jobs2", "name": "jobs", "version": "2.0.0"}),
            serde_json::json!({"id": "winjobs1", "name": "winjobs", "version": "1.0.0"}),
            serde_json::json!({"id": "winjobs2", "name": "winjobs", "version": "2.0.0"}),
        ]);
        let nodes = metadata["resolve"]["nodes"].as_array_mut().expect("nodes");
        let app = nodes[0]["deps"].as_array_mut().expect("app deps");
        app.extend([
            serde_json::json!({"pkg": "jobs2", "dep_kinds": [{"kind": null}]}),
            serde_json::json!({"pkg": "winjobs2", "dep_kinds": [{"kind": null}]}),
        ]);
        app.push(match via {
            "build" => serde_json::json!({"pkg": "cc", "dep_kinds": [{"kind": "build"}]}),
            _ => serde_json::json!({"pkg": "derive", "dep_kinds": [{"kind": null}]}),
        });
        nodes.extend([
            serde_json::json!({"id": "cc", "deps": [
                {"pkg": "jobs1", "dep_kinds": [{"kind": null, "target": "cfg(unix)"}]},
                {"pkg": "winjobs1", "dep_kinds": [{"kind": null, "target": "cfg(windows)"}]}
            ]}),
            serde_json::json!({"id": "derive", "deps": [
                {"pkg": "jobs1", "dep_kinds": [{"kind": null, "target": "cfg(unix)"}]}
            ]}),
            serde_json::json!({"id": "jobs1", "deps": []}),
            serde_json::json!({"id": "jobs2", "deps": []}),
            serde_json::json!({"id": "winjobs1", "deps": []}),
            serde_json::json!({"id": "winjobs2", "deps": []}),
        ]);
        metadata
    }

    /// `--target` Windows on a Linux host.
    fn windows_on_linux() -> super::PlatformFilter {
        super::PlatformFilter::Targets {
            targets: vec![platform(
                "x86_64-pc-windows-msvc",
                &["windows", "target_os=\"windows\""],
            )],
            host: platform("x86_64-unknown-linux-gnu", &["unix", "target_os=\"linux\""]),
        }
    }

    /// TASK-2319 AC #1: beneath a build-dependency, a non-host `--target`
    /// gates edges against the host — the `unix`-only `jobs 1.0.0` is a
    /// duplicate and the `windows`-only `winjobs 1.0.0` is not.
    #[test]
    fn edges_below_a_build_dependency_are_gated_on_the_host() {
        let metadata = host_metadata("build");
        let no_change = |_: &str, _: &str| Some(Vec::new());
        let report = super::find_duplicates(&metadata, false, &windows_on_linux(), &no_change);
        let names: Vec<&str> = report.crates.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["jobs", "term"]);
        assert_eq!(report.crates[0].older[0].pulled_by[0].name, "cc");
    }

    /// TASK-2319 AC #1: a proc-macro is compiled for the host, so its own
    /// dependencies are gated against the host too.
    #[test]
    fn edges_below_a_proc_macro_are_gated_on_the_host() {
        let metadata = host_metadata("proc-macro");
        assert_eq!(names_in(&metadata, &windows_on_linux()), ["jobs", "term"]);
    }

    /// With the host as the only target the host-side walk changes nothing.
    #[test]
    fn host_target_filter_matches_the_host_side() {
        let linux = only(platform(
            "x86_64-unknown-linux-gnu",
            &["unix", "target_os=\"linux\""],
        ));
        assert_eq!(names_in(&host_metadata("build"), &linux), ["jobs", "term"]);
    }

    /// TASK-2310 AC #1: a Linux target leaves out crates reachable only
    /// through other targets' `cfg(..)` / triple edges.
    #[test]
    fn find_duplicates_drops_crates_gated_on_other_targets() {
        let linux = only(platform(
            "x86_64-unknown-linux-gnu",
            &["unix", "target_os=\"linux\""],
        ));
        assert_eq!(duplicate_names(&linux), ["term"]);
    }

    /// TASK-2310 AC #2/#3: a Windows target and `all` both keep the
    /// target-gated duplicate, and its older version names the gated puller.
    #[test]
    fn find_duplicates_keeps_gated_crates_for_their_target_and_for_all() {
        let windows = only(platform(
            "x86_64-pc-windows-msvc",
            &["windows", "target_os=\"windows\""],
        ));
        assert_eq!(duplicate_names(&windows), ["sys", "term"]);
        assert_eq!(
            duplicate_names(&super::PlatformFilter::All),
            ["sys", "term"]
        );

        let no_change = |_: &str, _: &str| Some(Vec::new());
        for platforms in [windows, super::PlatformFilter::All] {
            let report = super::find_duplicates(&gated_metadata(), false, &platforms, &no_change);
            for dup in &report.crates {
                for older in &dup.older {
                    assert!(
                        !older.pulled_by.is_empty(),
                        "{} {} has no puller under {platforms:?}",
                        dup.name,
                        older.version
                    );
                }
            }
            let sys = report.crates.iter().find(|c| c.name == "sys").expect("sys");
            assert_eq!(sys.older[0].pulled_by[0].name, "gate");
        }
    }

    /// TASK-2310: an edge counts when any listed target admits it, and a
    /// cfg the target does not set never matches.
    #[test]
    fn platform_filter_admits_ungated_matching_cfg_and_matching_triple() {
        let linux = platform("x86_64-unknown-linux-gnu", &["unix"]);
        let filter = super::PlatformFilter::Targets {
            targets: vec![
                linux.clone(),
                platform("wasm32-wasip1", &["target_os=\"wasi\""]),
            ],
            host: linux,
        };
        let admits = |gate| filter.admits(gate, super::Side::Target);
        assert!(admits(None));
        assert!(admits(Some("cfg(unix)")));
        assert!(admits(Some("cfg(target_os = \"wasi\")")));
        assert!(admits(Some("wasm32-wasip1")));
        assert!(!admits(Some("cfg(windows)")));
        assert!(!admits(Some("x86_64-pc-windows-msvc")));
        assert!(!admits(Some("cfg(windows_raw_dylib)")));
        // Host-compiled edges see only the host.
        assert!(filter.admits(Some("cfg(unix)"), super::Side::Host));
        assert!(!filter.admits(Some("wasm32-wasip1"), super::Side::Host));
    }

    /// TASK-2310 AC #2: `all` (alone or among triples) disables the filter
    /// without probing rustc.
    #[test]
    fn platform_filter_all_needs_no_rustc() {
        let dir = std::path::Path::new("/nonexistent-ops-dir");
        assert_eq!(
            super::platform_filter(Some("all"), dir).expect("all"),
            super::PlatformFilter::All
        );
        assert_eq!(
            super::platform_filter(Some("x86_64-pc-windows-msvc,all"), dir).expect("all"),
            super::PlatformFilter::All
        );
    }

    #[test]
    fn duplicates_providers_register_under_distinct_names() {
        use super::{
            RustDuplicatesProvider, DUPLICATES_PROVIDER_NAME, DUPLICATES_WITH_DEV_PROVIDER_NAME,
        };
        assert_eq!(
            RustDuplicatesProvider { include_dev: false }.name(),
            DUPLICATES_PROVIDER_NAME
        );
        assert_eq!(
            RustDuplicatesProvider { include_dev: true }.name(),
            DUPLICATES_WITH_DEV_PROVIDER_NAME
        );
        assert_ne!(DUPLICATES_PROVIDER_NAME, DUPLICATES_WITH_DEV_PROVIDER_NAME);
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
        let value = RustDuplicatesProvider { include_dev: false }
            .provide(&mut ctx)
            .expect("provide");
        assert_eq!(value, serde_json::json!({"crates": []}));
    }
}
