//! DuckDB-backed metrics for the Rust identity provider.

use ops_core::project_identity::LanguageStat;
use ops_extension::Context;

/// Metrics queried from DuckDB (LOC, dependencies, coverage, languages).
pub(super) struct IdentityMetrics {
    pub loc: Option<i64>,
    pub file_count: Option<i64>,
    pub dependency_count: Option<usize>,
    pub coverage_percent: Option<f64>,
    pub languages: Vec<LanguageStat>,
}

pub(super) fn query_identity_metrics(ctx: &Context) -> IdentityMetrics {
    let (loc, file_count) = query_loc_from_db(ctx);
    let (coverage_percent, languages) = query_coverage_and_languages(ctx);
    IdentityMetrics {
        loc,
        file_count,
        dependency_count: query_dependency_count(ctx),
        coverage_percent,
        languages,
    }
}

fn query_dependency_count(ctx: &Context) -> Option<usize> {
    let db = ops_duckdb::get_db(ctx)?;
    ops_duckdb::sql::query_dependency_count(db).ok()
}

fn query_coverage_and_languages(ctx: &Context) -> (Option<f64>, Vec<LanguageStat>) {
    let db = match ops_duckdb::get_db(ctx) {
        Some(db) => db,
        None => return (None, vec![]),
    };

    let coverage = ops_duckdb::sql::query_project_coverage(db)
        .ok()
        .filter(|c| c.lines_count > 0)
        .map(|c| c.lines_percent);

    let languages = ops_duckdb::sql::query_project_languages(db).unwrap_or_default();

    (coverage, languages)
}

fn query_loc_from_db(ctx: &Context) -> (Option<i64>, Option<i64>) {
    let db = match ops_duckdb::get_db(ctx) {
        Some(db) => db,
        None => return (None, None),
    };

    let loc = ops_duckdb::sql::query_project_loc(db).ok();
    let files = ops_duckdb::sql::query_project_file_count(db).ok();
    (loc, files)
}
