//! Coverage extension: LLVM code coverage via `cargo llvm-cov`.
//! Collects per-file coverage data and loads into DuckDB.

mod ingestor;
#[cfg(test)]
mod tests;
pub mod views;

pub use ingestor::CoverageIngestor;

use anyhow::Context as AnyhowContext;
use ops_core::output::format_error_tail;
use ops_duckdb::{init_schema, DataIngestor, DuckDb};
use ops_extension::{Context, DataProvider, DataProviderError, DataProviderSchema, ExtensionType};
use std::io;
use std::path::Path;
use std::process::{Command, Output};

pub const NAME: &str = "coverage";
#[allow(dead_code)]
pub const DESCRIPTION: &str =
    "LLVM code coverage provider (per-file line, function, region, branch coverage)";
#[allow(dead_code)]
pub const SHORTNAME: &str = "cov";
pub const DATA_PROVIDER_NAME: &str = "coverage";

pub struct CoverageExtension;

ops_extension::impl_extension! {
    CoverageExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(CoverageProvider));
    },
}

struct CoverageProvider;

impl DataProvider for CoverageProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        ops_duckdb::try_provide_from_db(ctx, provide_from_db, |ctx| {
            collect_coverage(&ctx.working_directory)
        })
    }

    fn schema(&self) -> DataProviderSchema {
        use ops_extension::data_field;
        DataProviderSchema {
            description: "LLVM code coverage from `cargo llvm-cov` (per-file metrics)",
            fields: vec![
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
        }
    }
}

pub(crate) fn run_cargo_llvm_cov(working_dir: &Path) -> io::Result<Output> {
    Command::new("cargo")
        .args([
            "llvm-cov",
            "--workspace",
            "--no-cfg-coverage",
            "--all-features",
            "--tests",
            "--json",
        ])
        .current_dir(working_dir)
        .output()
}

pub(crate) fn check_llvm_cov_output(output: &Output) -> Result<(), anyhow::Error> {
    if !output.status.success() {
        let tail = format_error_tail(&output.stderr, 5);
        anyhow::bail!("cargo llvm-cov failed: {}", tail);
    }
    Ok(())
}

pub fn flatten_coverage_json(raw: &serde_json::Value) -> Result<serde_json::Value, anyhow::Error> {
    let data = raw
        .get("data")
        .and_then(|d| d.as_array())
        .context("missing or invalid 'data' array in coverage JSON")?;

    let first = data
        .first()
        .context("'data' array is empty in coverage JSON")?;

    let files = first
        .get("files")
        .and_then(|f| f.as_array())
        .context("missing or invalid 'files' array in coverage data")?;

    let mut records = Vec::with_capacity(files.len());
    for file in files {
        let filename = file.get("filename").and_then(|f| f.as_str()).unwrap_or("");
        let summary = file
            .get("summary")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let lines = summary
            .get("lines")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        let functions = summary
            .get("functions")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        let regions = summary
            .get("regions")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        let branches = summary
            .get("branches")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        records.push(serde_json::json!({
            "filename": filename,
            "lines_count": lines.get("count").and_then(|v| v.as_i64()).unwrap_or(0),
            "lines_covered": lines.get("covered").and_then(|v| v.as_i64()).unwrap_or(0),
            "lines_percent": lines.get("percent").and_then(|v| v.as_f64()).unwrap_or(0.0),
            "functions_count": functions.get("count").and_then(|v| v.as_i64()).unwrap_or(0),
            "functions_covered": functions.get("covered").and_then(|v| v.as_i64()).unwrap_or(0),
            "functions_percent": functions.get("percent").and_then(|v| v.as_f64()).unwrap_or(0.0),
            "regions_count": regions.get("count").and_then(|v| v.as_i64()).unwrap_or(0),
            "regions_covered": regions.get("covered").and_then(|v| v.as_i64()).unwrap_or(0),
            "regions_notcovered": regions.get("notcovered").and_then(|v| v.as_i64()).unwrap_or(0),
            "regions_percent": regions.get("percent").and_then(|v| v.as_f64()).unwrap_or(0.0),
            "branches_count": branches.get("count").and_then(|v| v.as_i64()).unwrap_or(0),
            "branches_covered": branches.get("covered").and_then(|v| v.as_i64()).unwrap_or(0),
            "branches_notcovered": branches.get("notcovered").and_then(|v| v.as_i64()).unwrap_or(0),
            "branches_percent": branches.get("percent").and_then(|v| v.as_f64()).unwrap_or(0.0),
        }));
    }
    Ok(serde_json::Value::Array(records))
}

pub fn collect_coverage(working_dir: &Path) -> Result<serde_json::Value, anyhow::Error> {
    let output = run_cargo_llvm_cov(working_dir)?;
    check_llvm_cov_output(&output)?;
    let raw: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parsing llvm-cov JSON output")?;
    flatten_coverage_json(&raw)
}

fn query_coverage_files(db: &DuckDb) -> Result<serde_json::Value, anyhow::Error> {
    ops_duckdb::sql::query_rows_to_json(
        db,
        "SELECT filename, lines_count, lines_covered, lines_percent, \
         functions_count, functions_covered, functions_percent, \
         regions_count, regions_covered, regions_notcovered, regions_percent, \
         branches_count, branches_covered, branches_notcovered, branches_percent \
         FROM coverage_files",
        |row| {
            Ok(serde_json::json!({
                "filename": row.get::<_, String>(0)?,
                "lines_count": row.get::<_, i64>(1)?,
                "lines_covered": row.get::<_, i64>(2)?,
                "lines_percent": row.get::<_, f64>(3)?,
                "functions_count": row.get::<_, i64>(4)?,
                "functions_covered": row.get::<_, i64>(5)?,
                "functions_percent": row.get::<_, f64>(6)?,
                "regions_count": row.get::<_, i64>(7)?,
                "regions_covered": row.get::<_, i64>(8)?,
                "regions_notcovered": row.get::<_, i64>(9)?,
                "regions_percent": row.get::<_, f64>(10)?,
                "branches_count": row.get::<_, i64>(11)?,
                "branches_covered": row.get::<_, i64>(12)?,
                "branches_notcovered": row.get::<_, i64>(13)?,
                "branches_percent": row.get::<_, f64>(14)?,
            }))
        },
    )
}

fn provide_from_db(db: &DuckDb, ctx: &Context) -> Result<serde_json::Value, anyhow::Error> {
    ops_duckdb::sql::provide_via_ingestor(
        db,
        ctx,
        "coverage_files",
        &CoverageIngestor,
        query_coverage_files,
    )
}

pub fn load_coverage(data_dir: &Path, db: &DuckDb) -> Result<(), anyhow::Error> {
    let json_path = data_dir.join("coverage_files.json");
    if !json_path.exists() {
        anyhow::bail!("coverage_files.json not found. Run collect first.");
    }
    init_schema(db)?;
    let ingestor = CoverageIngestor;
    ingestor.load(data_dir, db)?;
    Ok(())
}
