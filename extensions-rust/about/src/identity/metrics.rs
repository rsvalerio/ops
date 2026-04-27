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

// ERR-2 / TASK-0376: every DuckDB query lookup logs at warn before falling
// back. A schema mismatch or migration bug used to render as silent zeros
// because all four call sites used `.ok()` / `.unwrap_or_default()` without
// any signal.

fn query_dependency_count(ctx: &Context) -> Option<usize> {
    let db = ops_duckdb::get_db(ctx)?;
    match ops_duckdb::sql::query_dependency_count(db) {
        Ok(n) => Some(n),
        Err(e) => {
            tracing::warn!(
                query = "query_dependency_count",
                "duckdb query failed; dependency_count will be None: {e:#}"
            );
            None
        }
    }
}

fn query_coverage_and_languages(ctx: &Context) -> (Option<f64>, Vec<LanguageStat>) {
    let db = match ops_duckdb::get_db(ctx) {
        Some(db) => db,
        None => return (None, vec![]),
    };

    let coverage = match ops_duckdb::sql::query_project_coverage(db) {
        Ok(c) if c.lines_count > 0 => Some(c.lines_percent),
        Ok(_) => None,
        Err(e) => {
            tracing::warn!(
                query = "query_project_coverage",
                "duckdb query failed; coverage_percent will be None: {e:#}"
            );
            None
        }
    };

    let languages = match ops_duckdb::sql::query_project_languages(db) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                query = "query_project_languages",
                "duckdb query failed; languages will be empty: {e:#}"
            );
            vec![]
        }
    };

    (coverage, languages)
}

fn query_loc_from_db(ctx: &Context) -> (Option<i64>, Option<i64>) {
    let db = match ops_duckdb::get_db(ctx) {
        Some(db) => db,
        None => return (None, None),
    };

    let loc = match ops_duckdb::sql::query_project_loc(db) {
        Ok(n) => Some(n),
        Err(e) => {
            tracing::warn!(
                query = "query_project_loc",
                "duckdb query failed; loc will be None: {e:#}"
            );
            None
        }
    };
    let files = match ops_duckdb::sql::query_project_file_count(db) {
        Ok(n) => Some(n),
        Err(e) => {
            tracing::warn!(
                query = "query_project_file_count",
                "duckdb query failed; file_count will be None: {e:#}"
            );
            None
        }
    };
    (loc, files)
}
