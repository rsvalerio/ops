//! LOC, file count, and per-language queries over `tokei_files`, plus the
//! Rust production / test / example breakdown over `rust_loc_summary`.

use crate::DuckDb;
use std::collections::HashMap;

use ops_core::project_identity::LanguageStat;

use super::super::ingest::table_exists;
use super::helpers::{
    query_per_crate_i64, query_project_scalar, ColumnAlias, ColumnName, PerCrateI64Query,
    QueryTableName,
};

/// Query total file count across the whole project from `tokei_files`.
///
/// # Errors
///
/// If the database lock is poisoned, or the query fails. A missing table
/// is not an error — it yields `0`.
pub fn query_project_file_count(db: &DuckDb) -> anyhow::Result<i64> {
    query_project_scalar(
        db,
        "tokei_files",
        "SELECT COUNT(*) FROM tokei_files",
        "query_project_file_count",
    )
}

/// Query per-crate file counts from `tokei_files`.
///
/// Returns a map of member path -> file count. Members with no matching
/// files get 0.
///
/// # Errors
///
/// If the database lock is poisoned, or the query fails. A missing table
/// is not an error — every member maps to `0`.
pub fn query_crate_file_count(
    db: &DuckDb,
    member_paths: &[&str],
) -> anyhow::Result<HashMap<String, i64>> {
    query_per_crate_i64(&PerCrateI64Query {
        db,
        table: QueryTableName::new("tokei_files")?,
        member_paths,
        select_expr: "COUNT(f.file)",
        join_alias: ColumnAlias::new("f")?,
        join_column: ColumnName::new("file")?,
        label: "query_crate_file_count",
    })
}

/// Query total lines of code across the whole project from `tokei_files`.
///
/// # Errors
///
/// If the database lock is poisoned, or the query fails. A missing table
/// is not an error — it yields `0`.
pub fn query_project_loc(db: &DuckDb) -> anyhow::Result<i64> {
    query_project_scalar(
        db,
        "tokei_files",
        "SELECT COALESCE(SUM(code), 0) FROM tokei_files",
        "query_project_loc",
    )
}

/// Query per-language breakdown from `tokei_files`: LOC, file count, and
/// percentages of both. Ordered by LOC descending.
///
/// READ-5 / TASK-0362: languages whose `loc_pct` rounds below 0.1% are
/// omitted, *including* the case where every language is sub-threshold.
/// Previously this function fell back to the top entry when the filtered
/// set would otherwise be empty, which contradicted the documented
/// "omit < 0.1%" contract and made it impossible for callers to
/// distinguish "no tokei data" from "every language tiny". The empty
/// return is now the only signal, matching the doc.
///
/// # Errors
///
/// If the database lock is poisoned, or the query or row decode fails. A
/// missing `tokei_files` table is not an error — it yields an empty vec.
pub fn query_project_languages(db: &DuckDb) -> anyhow::Result<Vec<LanguageStat>> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_project_languages")?;

    if !table_exists(&conn, "tokei_files")? {
        return Ok(vec![]);
    }

    let mut stmt = conn
        .prepare(
            "WITH totals AS (SELECT COALESCE(SUM(code), 0) AS total_loc, COUNT(*) AS total_files FROM tokei_files) \
             SELECT language, \
                    SUM(code) AS loc, \
                    COUNT(*) AS files, \
                    COALESCE(ROUND(SUM(code) * 100.0 / NULLIF(totals.total_loc, 0), 1), 0) AS loc_pct, \
                    COALESCE(ROUND(COUNT(*) * 100.0 / NULLIF(totals.total_files, 0), 1), 0) AS files_pct \
             FROM tokei_files, totals \
             GROUP BY language, totals.total_loc, totals.total_files \
             ORDER BY SUM(code) DESC",
        )
        .context("preparing query_project_languages")?;

    let rows = stmt
        .query_map([], |row: &duckdb::Row<'_>| {
            Ok(LanguageStat::new(
                row.get::<_, String>(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .context("querying project languages")?;

    let mut filtered = Vec::new();
    for row in rows {
        let stat = row.context("reading language row")?;
        if stat.loc_pct >= 0.1 {
            filtered.push(stat);
        }
    }
    Ok(filtered)
}

/// One row of the `rust_loc_summary` view: the totals for a single region
/// (`main`, `test`, or `example`) of the Rust sources.
///
/// `region` stays a `String` rather than an enum because the canonical
/// region list lives in the `ops-rust-loc` crate, which depends on this
/// one — modelling it here would invert that edge. Renderers map the raw
/// value to a label and treat an unrecognised region as displayable data
/// rather than an error, so a future region added upstream shows up
/// instead of disappearing.
///
/// Constructed by struct literal rather than a `new` — six same-typed
/// `i64` counts behind a positional constructor read as an unlabelled
/// number soup at every callsite (and trip `too_many_arguments`), while
/// field names make a transposed `docs`/`comments` pair obvious. That
/// rules out `#[non_exhaustive]`, which would block literal construction
/// from the sibling crates that render and test this row; nothing here is
/// published, so the compatibility guarantee it buys has no consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustLocStat {
    pub region: String,
    pub files: i64,
    pub code: i64,
    pub docs: i64,
    pub comments: i64,
    pub blanks: i64,
    pub lines: i64,
}

/// Query the Rust production / test / example line breakdown from the
/// `rust_loc_summary` view, which the `rust-loc` data provider creates
/// alongside the `rust_loc_files` table it ingests.
///
/// Returns an empty `Vec` when the view is absent. That is the normal
/// state for every non-Rust workspace — `rust-loc` only registers on the
/// Rust stack — so it is reported as "no data" rather than an error, the
/// same contract [`query_project_languages`] uses for a missing
/// `tokei_files`.
///
/// Rows come back in the view's own order (code descending); callers that
/// need a fixed region order sort at the point of display.
///
/// # Errors
///
/// If the database lock is poisoned, or the query or row decode fails. A
/// missing `rust_loc_summary` table is not an error — it yields an empty vec.
pub fn query_rust_loc_summary(db: &DuckDb) -> anyhow::Result<Vec<RustLocStat>> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_rust_loc_summary")?;

    if !table_exists(&conn, "rust_loc_summary")? {
        return Ok(vec![]);
    }

    let mut stmt = conn
        .prepare(
            "SELECT region, files, code, docs, comments, blanks, lines \
             FROM rust_loc_summary",
        )
        .context("preparing query_rust_loc_summary")?;

    let rows = stmt
        .query_map([], |row: &duckdb::Row<'_>| {
            Ok(RustLocStat {
                region: row.get(0)?,
                files: row.get(1)?,
                code: row.get(2)?,
                docs: row.get(3)?,
                comments: row.get(4)?,
                blanks: row.get(5)?,
                lines: row.get(6)?,
            })
        })
        .context("querying rust loc summary")?;

    rows.map(|row| row.context("reading rust loc row"))
        .collect()
}

/// Count the distinct `.rs` files behind `rust_loc_files`.
///
/// Regions overlap within a file — a module with a `#[cfg(test)]` block
/// contributes both a `main` and a `test` row — so summing the summary
/// view's per-region `files` column counts such a file twice. This is the
/// honest denominator for "N files scanned".
///
/// Returns 0 when the table is absent, matching
/// [`query_rust_loc_summary`]'s empty result for the same workspace.
///
/// # Errors
///
/// If the database lock is poisoned, or the query fails. A missing table
/// is not an error — it yields `0`.
pub fn query_rust_loc_file_count(db: &DuckDb) -> anyhow::Result<i64> {
    query_project_scalar(
        db,
        "rust_loc_files",
        "SELECT COUNT(DISTINCT file) FROM rust_loc_files",
        "query_rust_loc_file_count",
    )
}

/// Query per-crate lines of code from `tokei_files`.
///
/// Returns a map of member path -> total code lines. Members with no matching
/// files get 0.
///
/// # Errors
///
/// If the database lock is poisoned, or the query fails. A missing table
/// is not an error — every member maps to `0`.
pub fn query_crate_loc(db: &DuckDb, member_paths: &[&str]) -> anyhow::Result<HashMap<String, i64>> {
    query_per_crate_i64(&PerCrateI64Query {
        db,
        table: QueryTableName::new("tokei_files")?,
        member_paths,
        select_expr: "COALESCE(SUM(f.code), 0)",
        join_alias: ColumnAlias::new("f")?,
        join_column: ColumnName::new("file")?,
        label: "query_crate_loc",
    })
}
