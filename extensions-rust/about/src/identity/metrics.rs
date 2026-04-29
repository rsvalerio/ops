//! DuckDB-backed metrics for the Rust identity provider.

use ops_core::project_identity::LanguageStat;
use ops_duckdb::sql::query_or_warn;
use ops_duckdb::DuckDb;
use ops_extension::Context;

/// Metrics queried from DuckDB (LOC, dependencies, coverage, languages).
pub(super) struct IdentityMetrics {
    pub loc: Option<i64>,
    pub file_count: Option<i64>,
    pub dependency_count: Option<usize>,
    pub coverage_percent: Option<f64>,
    pub languages: Vec<LanguageStat>,
}

/// TASK-0530: resolve `get_db` once and thread the borrowed handle to each
/// sub-query so we don't re-locate / re-lock the DuckDB handle three times
/// per `provide()`. Same anti-pattern that about/units::enrich_from_db got
/// fixed for. Falls back to all-`None` metrics when DuckDB is not available.
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
    let coverage = query_or_warn(
        "query_project_coverage",
        "coverage_percent will be None",
        None,
        || {
            ops_duckdb::sql::query_project_coverage(db).map(|c| {
                if c.lines_count > 0 {
                    Some(c.lines_percent)
                } else {
                    None
                }
            })
        },
    );

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
