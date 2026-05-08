//! Rust `project_coverage` data provider.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use ops_core::project_identity::{CoverageStats, ProjectCoverage, UnitCoverage};
use ops_duckdb::sql::{query_crate_coverage, query_or_warn, query_project_coverage, CrateCoverage};
use ops_duckdb::DuckDb;
use ops_extension::{Context, DataProvider, DataProviderError};

use crate::query::{load_workspace_manifest, log_manifest_load_failure};
use crate::units::resolve_crate_display_name;

pub(crate) const PROVIDER_NAME: &str = "project_coverage";

/// DUP-1 (TASK-1079): per-process memoization for `query_project_coverage`.
///
/// `RustCoverageProvider::provide` and `identity::metrics::query_identity_metrics`
/// both run during a single `ops about` invocation and historically each
/// dispatched their own `query_project_coverage` call against the same
/// DuckDB. That doubled the scan and — more visibly — fired any
/// `query_or_warn` schema-drift log line twice.
///
/// We dedup with a tiny global cache keyed by the `DuckDb` pointer address.
/// The pointer is a stable identity for the lifetime of the in-process Arc
/// the providers share via [`Context`]; since the `ops` binary exits between
/// invocations, the cache is naturally process-scoped and never has to
/// invalidate. `Option<CrateCoverage>` mirrors the `query_or_warn` fallback
/// (None on query failure) so a hard failure is also memoized — the warn
/// fires exactly once per run regardless of how many providers consume the
/// value.
fn project_coverage_cache() -> &'static Mutex<HashMap<usize, Option<CrateCoverage>>> {
    static CACHE: OnceLock<Mutex<HashMap<usize, Option<CrateCoverage>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Run `query_project_coverage` at most once per `DuckDb` per process.
///
/// Both the identity metrics provider and the coverage provider call this
/// in turn during `ops about`; the second caller gets the cached value
/// (including the cached `None` when the query failed and `query_or_warn`
/// already logged the warn).
pub(crate) fn cached_query_project_coverage(db: &DuckDb) -> Option<CrateCoverage> {
    let key = std::ptr::from_ref(db) as usize;

    if let Ok(guard) = project_coverage_cache().lock() {
        if let Some(cached) = guard.get(&key) {
            return cached.clone();
        }
    }

    let fresh = query_or_warn(
        "query_project_coverage",
        "reporting empty coverage",
        None,
        || query_project_coverage(db).map(Some),
    );

    if let Ok(mut guard) = project_coverage_cache().lock() {
        guard.insert(key, fresh.clone());
    }
    fresh
}

#[cfg(test)]
pub(crate) fn clear_project_coverage_cache_for_test() {
    if let Ok(mut guard) = project_coverage_cache().lock() {
        guard.clear();
    }
}

pub(crate) struct RustCoverageProvider;

impl DataProvider for RustCoverageProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let cwd = ctx.working_directory.clone();
        let manifest = match load_workspace_manifest(ctx) {
            Ok(m) => Some(m),
            Err(e) => {
                log_manifest_load_failure(&e);
                None
            }
        };

        let Some(db) = ops_duckdb::get_db(ctx) else {
            return Ok(serde_json::to_value(ProjectCoverage::default())?);
        };

        // ERR-2 / TASK-0376 / PATTERN-1 (TASK-0608): route through
        // `query_or_warn` so this site matches the convention used by every
        // sister DuckDB call in the crate (units, identity::metrics,
        // deps_provider). Wrapping the return in `Option` preserves the
        // early-return-on-failure semantics — if the project_coverage query
        // fails we return a fully-default `ProjectCoverage` rather than
        // partial data, matching the prior behaviour.
        // DUP-1 / TASK-1079: dispatched via `cached_query_project_coverage`
        // so the parallel call from `identity::metrics` reuses this result
        // (and any warn it already logged) instead of re-querying DuckDB
        // and double-warning per `ops about`.
        let project_total = cached_query_project_coverage(db);
        let Some(p) = project_total else {
            return Ok(serde_json::to_value(ProjectCoverage::default())?);
        };
        let total = CoverageStats::new(p.lines_percent, p.lines_covered, p.lines_count);

        let units = if let Some(manifest) = manifest {
            // ERR-1 / TASK-1076: read the resolved-members sibling on
            // `LoadedManifest`. The cached `manifest.workspace.members` now
            // preserves the original glob spec verbatim.
            let members: &[String] = manifest.resolved_members();
            if members.is_empty() {
                Vec::new()
            } else {
                // READ-5 / TASK-0986: short-circuit when the workspace cwd is
                // not valid UTF-8 instead of piping a U+FFFD-replaced string
                // into the SQL key. The lossy collapse would silently match
                // an unrelated workspace's coverage rows. Sister policy to
                // TASK-0946 (workspace member relpaths in query.rs).
                let Some(cwd_str) = cwd.to_str() else {
                    tracing::warn!(
                        cwd = ?cwd.display(),
                        "non-UTF-8 cwd; skipping per-crate coverage to avoid lossy SQL key collapse"
                    );
                    return Ok(serde_json::to_value(ProjectCoverage::new(
                        total,
                        Vec::new(),
                    ))?);
                };
                let member_strs: Vec<&str> = members.iter().map(String::as_str).collect();
                let per_crate = query_or_warn(
                    "query_crate_coverage",
                    "per-crate coverage will be blank",
                    std::collections::HashMap::<String, ops_duckdb::sql::CrateCoverage>::new(),
                    || query_crate_coverage(db, &member_strs, cwd_str),
                );
                // PERF-1 (TASK-0798): resolve display names up front in one
                // pass over members with coverage rows, so each member's
                // Cargo.toml is read at most once per provide() call.
                let mut display_names: std::collections::HashMap<&str, String> =
                    std::collections::HashMap::with_capacity(per_crate.len());
                for member in members {
                    if per_crate.contains_key(member.as_str()) {
                        display_names
                            .insert(member.as_str(), resolve_crate_display_name(member, &cwd));
                    }
                }
                members
                    .iter()
                    .filter_map(|member| {
                        let cov = per_crate.get(member)?;
                        let unit_name = display_names.remove(member.as_str())?;
                        Some(UnitCoverage::new(
                            unit_name,
                            member.clone(),
                            CoverageStats::new(
                                cov.lines_percent,
                                cov.lines_covered,
                                cov.lines_count,
                            ),
                        ))
                    })
                    .collect()
            }
        } else {
            Vec::new()
        };

        let coverage = ProjectCoverage::new(total, units);
        serde_json::to_value(&coverage).map_err(DataProviderError::from)
    }
}

#[cfg(test)]
mod cache_tests {
    use super::{cached_query_project_coverage, clear_project_coverage_cache_for_test};
    use ops_duckdb::DuckDb;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct BufWriter(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for BufWriter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for BufWriter {
        type Writer = BufWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    /// DUP-1 / TASK-1079: the identity-metrics and coverage providers used
    /// to dispatch their own `query_project_coverage` against the same
    /// DuckDB during a single `ops about`, so any `query_or_warn`
    /// schema-drift line fired twice. Pin that the per-process cache fires
    /// the underlying query (and its warn) exactly once across both call
    /// sites for a forced query failure.
    #[test]
    #[serial_test::serial(project_coverage_cache)]
    fn project_coverage_warn_fires_once_across_both_call_sites() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");

        // Force a hard failure inside `query_project_coverage`:
        // create `coverage_files` with the column `lines_count` typed as
        // VARCHAR, which makes the CASE/SUM aggregation in
        // `coverage_col_select` blow up with a type error. This is the
        // schema-drift scenario the DUP report cites.
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE coverage_files (\
                    filename VARCHAR, \
                    lines_count VARCHAR, \
                    lines_covered VARCHAR, \
                    lines_percent VARCHAR\
                 ); \
                 INSERT INTO coverage_files VALUES ('a.rs', 'x', 'y', 'z');",
            )
            .expect("seed broken-schema coverage_files");
        }

        clear_project_coverage_cache_for_test();

        let buf = BufWriter::default();
        let captured = buf.0.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf)
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();

        // Two call-site simulation: both providers run during a single
        // `ops about`, so we invoke the cache helper twice. The first call
        // dispatches the query and logs once; the second must hit the
        // cache and stay silent.
        let (first, second) = tracing::subscriber::with_default(subscriber, || {
            let a = cached_query_project_coverage(&db);
            let b = cached_query_project_coverage(&db);
            (a, b)
        });

        // Both call sites observe the same fallback value (None) — failure
        // memoization is part of the contract.
        assert!(first.is_none(), "first call must hit fallback");
        assert!(second.is_none(), "second call must hit cached fallback");

        let logs = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        let warn_count = logs.matches("query_project_coverage").count();
        assert_eq!(
            warn_count, 1,
            "warn must fire exactly once across both call sites; got {warn_count} in:\n{logs}"
        );

        clear_project_coverage_cache_for_test();
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    /// READ-5 / TASK-0986: a non-UTF-8 cwd must NOT collapse to a
    /// U+FFFD-replaced SQL key. The provider's short-circuit relies on
    /// `Path::to_str()` returning `None` for non-UTF-8 input — pin that
    /// invariant so a future refactor that swaps in `to_string_lossy`
    /// can't silently re-introduce the lossy-collapse.
    #[test]
    fn non_utf8_cwd_path_to_str_returns_none() {
        // Construct a non-UTF-8 path: 0x80 is a continuation byte with no
        // leading byte, so it's invalid UTF-8.
        let bytes = b"/tmp/non\xC3\x28-utf8";
        let p = Path::new(OsStr::from_bytes(bytes));
        assert!(
            p.to_str().is_none(),
            "non-UTF-8 path must not pass `to_str()`; got: {:?}",
            p.to_str()
        );
        // Confirm that `to_string_lossy` would have produced a U+FFFD
        // replacement key — the very behaviour the short-circuit avoids.
        let lossy = p.to_string_lossy();
        assert!(
            lossy.contains('\u{FFFD}'),
            "expected lossy conversion to produce U+FFFD: {lossy}"
        );
    }
}
