//! `CoverageProvider` impl and `DuckDB` readback path.
//!
//! ARCH-1 / TASK-1559: lifted out of `lib.rs`.

use crate::ingestor::CoverageIngestor;
use crate::parse::{collect_coverage, CoverageRow};
use ops_duckdb::DuckDb;
use ops_extension::{Context, DataProvider, DataProviderError, DataProviderSchema};

pub struct CoverageProvider;

impl DataProvider for CoverageProvider {
    fn name(&self) -> &'static str {
        crate::DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        ops_duckdb::try_provide_from_db(ctx, provide_from_db, |ctx| {
            collect_coverage(&ctx.working_directory)
        })
    }

    fn schema(&self) -> DataProviderSchema {
        use ops_extension::data_field;
        // DUP-3 / TASK-1555: the field list mirrors `CoverageRow`'s struct
        // layout. Adding a new metric flows through one struct edit + this
        // list; the `query_coverage_files` projection and the `flatten`
        // builder no longer need parallel edits because they consume the
        // struct directly via `serde`.
        DataProviderSchema::new(
            "LLVM code coverage from `cargo llvm-cov` (per-file metrics)",
            vec![
                data_field!("filename", "str", "Source file path"),
                data_field!("lines_count", "int", "Total lines instrumented"),
                data_field!("lines_covered", "int", "Lines covered by tests"),
                data_field!("lines_percent", "float", "Line coverage percentage"),
                data_field!("functions_count", "int", "Total functions instrumented"),
                data_field!("functions_covered", "int", "Functions covered by tests"),
                data_field!("functions_percent", "float", "Function coverage percentage"),
                data_field!("regions_count", "int", "Total code regions"),
                data_field!("regions_covered", "int", "Regions covered by tests"),
                data_field!("regions_notcovered", "int", "Regions not covered by tests"),
                data_field!("regions_percent", "float", "Region coverage percentage"),
                data_field!("branches_count", "int", "Total branches"),
                data_field!("branches_covered", "int", "Branches covered by tests"),
                data_field!(
                    "branches_notcovered",
                    "int",
                    "Branches not covered by tests"
                ),
                data_field!("branches_percent", "float", "Branch coverage percentage"),
            ],
        )
    }
}

/// DUP-3 / TASK-1555: project rows through `CoverageRow` so the SELECT list
/// and the JSON row builder share one schema. Column binding is by name
/// (TASK-1610) so reordering the SELECT or swapping same-typed columns
/// produces a clear runtime error instead of silent data corruption.
pub fn query_coverage_files(db: &DuckDb) -> Result<serde_json::Value, anyhow::Error> {
    ops_duckdb::sql::query_rows_to_json(
        db,
        "SELECT filename, lines_count, lines_covered, lines_percent, \
         functions_count, functions_covered, functions_percent, \
         regions_count, regions_covered, regions_notcovered, regions_percent, \
         branches_count, branches_covered, branches_notcovered, branches_percent \
         FROM coverage_files",
        |row| {
            let coverage = CoverageRow {
                filename: row.get::<_, String>("filename")?,
                lines_count: row.get::<_, i64>("lines_count")?,
                lines_covered: row.get::<_, i64>("lines_covered")?,
                lines_percent: row.get::<_, f64>("lines_percent")?,
                functions_count: row.get::<_, i64>("functions_count")?,
                functions_covered: row.get::<_, i64>("functions_covered")?,
                functions_percent: row.get::<_, f64>("functions_percent")?,
                regions_count: row.get::<_, i64>("regions_count")?,
                regions_covered: row.get::<_, i64>("regions_covered")?,
                regions_notcovered: row.get::<_, i64>("regions_notcovered")?,
                regions_percent: row.get::<_, f64>("regions_percent")?,
                branches_count: row.get::<_, i64>("branches_count")?,
                branches_covered: row.get::<_, i64>("branches_covered")?,
                branches_notcovered: row.get::<_, i64>("branches_notcovered")?,
                branches_percent: row.get::<_, f64>("branches_percent")?,
            };
            serde_json::to_value(coverage).map_err(|e| {
                duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Any, Box::new(e))
            })
        },
    )
}

pub fn provide_from_db(db: &DuckDb, ctx: &Context) -> Result<serde_json::Value, anyhow::Error> {
    ops_duckdb::sql::provide_via_ingestor(
        db,
        ctx,
        "coverage_files",
        &CoverageIngestor,
        query_coverage_files,
    )
}
