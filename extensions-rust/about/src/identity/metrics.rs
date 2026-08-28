//! DuckDB-backed metrics for the Rust identity provider.

use ops_core::project_identity::LanguageStat;
use ops_duckdb::sql::query_or_warn;
use ops_duckdb::DuckDb;
use ops_extension::Context;

/// Metrics queried from `DuckDB` (LOC, dependencies, coverage, languages).
pub(super) struct IdentityMetrics {
    pub loc: Option<i64>,
    pub file_count: Option<i64>,
    pub dependency_count: Option<usize>,
    pub coverage_percent: Option<f64>,
    pub languages: Vec<LanguageStat>,
}

/// TASK-0530: resolve `get_db` once and thread the borrowed handle to each
/// sub-query so we don't re-locate / re-lock the `DuckDB` handle three times
/// per `provide()`. Same anti-pattern that `about/units::enrich_from_db` got
/// fixed for. Falls back to all-`None` metrics when `DuckDB` is not available.
pub(super) fn query_identity_metrics(ctx: &Context) -> IdentityMetrics {
    let Some(db) = ops_duckdb::get_db(ctx) else {
        return IdentityMetrics {
            loc: None,
            file_count: None,
            dependency_count: None,
            coverage_percent: None,
            languages: Vec::new(),
        };
    };
    let (loc, file_count) = query_loc_from_db(db);
    let (coverage_percent, languages) = query_coverage_and_languages(db);
    IdentityMetrics {
        loc,
        file_count,
        dependency_count: query_dependency_count(db),
        coverage_percent,
        languages,
    }
}

// ERR-2 / TASK-0376: every DuckDB query lookup logs at warn before falling
// back. A schema mismatch or migration bug used to render as silent zeros
// because all four call sites used `.ok()` / `.unwrap_or_default()` without
// any signal.

fn query_dependency_count(db: &DuckDb) -> Option<usize> {
    query_or_warn(
        "query_dependency_count",
        "dependency_count will be None",
        None,
        || ops_duckdb::sql::query_dependency_count(db).map(Some),
    )
}

fn query_coverage_and_languages(db: &DuckDb) -> (Option<f64>, Vec<LanguageStat>) {
    // DUP-1 / TASK-1079: share the single `query_project_coverage` result
    // with `RustCoverageProvider` via the per-process cache rather than
    // dispatching our own query. The cache memoizes `Option<CrateCoverage>`
    // so a hard failure here is also already logged (or about to be) by
    // the sibling provider exactly once per `ops about` run.
    let coverage = crate::coverage_provider::cached_query_project_coverage(db).and_then(|c| {
        if c.lines_count > 0 {
            Some(c.lines_percent)
        } else {
            None
        }
    });

    let languages = query_or_warn(
        "query_project_languages",
        "languages will be empty",
        vec![],
        || ops_duckdb::sql::query_project_languages(db),
    );

    (coverage, languages)
}

fn query_loc_from_db(db: &DuckDb) -> (Option<i64>, Option<i64>) {
    let loc = query_or_warn("query_project_loc", "loc will be None", None, || {
        ops_duckdb::sql::query_project_loc(db).map(Some)
    });
    let files = query_or_warn(
        "query_project_file_count",
        "file_count will be None",
        None,
        || ops_duckdb::sql::query_project_file_count(db).map(Some),
    );
    (loc, files)
}

#[cfg(test)]
mod tests {
    use super::{query_coverage_and_languages, query_identity_metrics};
    use ops_duckdb::DuckDb;
    use ops_extension::Context;
    use std::sync::Arc;

    fn seed_coverage(lines_count: i64, lines_covered: i64, lines_percent: f64) -> DuckDb {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE coverage_files (\
                    filename VARCHAR, \
                    lines_count BIGINT, \
                    lines_covered BIGINT, \
                    lines_percent DOUBLE\
                 );",
            )
            .expect("create coverage_files");
            conn.execute_batch(&format!(
                "INSERT INTO coverage_files VALUES \
                 ('a.rs', {lines_count}, {lines_covered}, {lines_percent});"
            ))
            .expect("seed coverage row");
            drop(conn);
        }
        db
    }

    /// TEST-5 / TASK-1776 AC #3: with no `DuckDB` attached every metric falls
    /// back to `None` / empty rather than erroring.
    #[test]
    fn metrics_fall_back_to_none_without_duckdb() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ctx = Context::test_context(dir.path().to_path_buf());
        let metrics = query_identity_metrics(&ctx);
        assert!(metrics.loc.is_none());
        assert!(metrics.file_count.is_none());
        assert!(metrics.dependency_count.is_none());
        assert!(metrics.coverage_percent.is_none());
        assert!(metrics.languages.is_empty());
    }

    /// TEST-5 / TASK-1776 AC #4: the `lines_count > 0` guard decides whether a
    /// project with zero measurable lines reports `Some(0.0)` (a real 0%
    /// coverage figure) or `None` (no coverage data at all). Pin both sides.
    #[test]
    #[serial_test::serial(project_coverage_cache)]
    fn zero_line_project_reports_no_coverage_percent() {
        let db = seed_coverage(0, 0, 0.0);
        let (coverage, _languages) = query_coverage_and_languages(&db);
        assert_eq!(
            coverage, None,
            "a project with zero measured lines has no coverage percentage"
        );
    }

    #[test]
    #[serial_test::serial(project_coverage_cache)]
    fn non_zero_line_project_reports_its_coverage_percent() {
        let db = seed_coverage(10, 5, 50.0);
        let (coverage, _languages) = query_coverage_and_languages(&db);
        assert_eq!(coverage, Some(50.0));
    }

    /// The whole-metrics path with a live `DuckDB` still surfaces the coverage
    /// figure through `query_identity_metrics`.
    #[test]
    #[serial_test::serial(project_coverage_cache)]
    fn metrics_surface_coverage_from_an_attached_duckdb() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut ctx = Context::test_context(dir.path().to_path_buf());
        ctx.attach_db(Arc::new(seed_coverage(8, 2, 25.0)));
        let metrics = query_identity_metrics(&ctx);
        assert_eq!(metrics.coverage_percent, Some(25.0));
    }
}
