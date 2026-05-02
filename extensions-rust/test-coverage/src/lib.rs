//! Coverage extension: LLVM code coverage via `cargo llvm-cov`.
//! Collects per-file coverage data and loads into DuckDB.

mod ingestor;
#[cfg(test)]
mod tests;
pub mod views;

pub use ingestor::CoverageIngestor;

use anyhow::Context as AnyhowContext;
use ops_core::output::format_error_tail;
use ops_core::subprocess::{run_cargo, RunError};
use ops_duckdb::{init_schema, DataIngestor, DuckDb, LoadResult};
use ops_extension::{Context, DataProvider, DataProviderError, DataProviderSchema, ExtensionType};
use std::path::Path;
use std::process::Output;
use std::time::Duration;

pub const NAME: &str = "coverage";
#[allow(dead_code)]
pub const DESCRIPTION: &str =
    "LLVM code coverage provider (per-file line, function, region, branch coverage)";
#[allow(dead_code)]
pub const SHORTNAME: &str = "cov";
pub const DATA_PROVIDER_NAME: &str = "coverage";

/// API-9 / TASK-0922: construct via the registered extension factory only.
#[non_exhaustive]
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
    factory: COVERAGE_FACTORY = |_, _| {
        Some((NAME, Box::new(CoverageExtension)))
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

/// Default timeout for `cargo llvm-cov`; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`. Coverage runs the full test suite, so this
/// is the largest of the cargo-subprocess defaults.
pub(crate) const CARGO_LLVM_COV_TIMEOUT: Duration = Duration::from_secs(900);

pub(crate) fn run_cargo_llvm_cov(working_dir: &Path) -> Result<Output, RunError> {
    run_cargo(
        &[
            "llvm-cov",
            "--workspace",
            "--no-cfg-coverage",
            "--tests",
            "--json",
        ],
        working_dir,
        CARGO_LLVM_COV_TIMEOUT,
        "cargo llvm-cov",
    )
}

pub(crate) fn check_llvm_cov_output(output: &Output) -> Result<(), anyhow::Error> {
    if !output.status.success() {
        let tail = format_error_tail(&output.stderr, 5);
        anyhow::bail!("cargo llvm-cov failed: {}", tail);
    }
    Ok(())
}

/// Coverage section counters extracted from one of `lines` / `functions` /
/// `regions` / `branches` in the llvm-cov per-file `summary` block. `notcovered`
/// is only meaningful for region- and branch-level sections; for lines and
/// functions it is always zero.
#[derive(Default)]
struct Section {
    count: i64,
    covered: i64,
    notcovered: i64,
    percent: f64,
}

fn extract_section(summary: &serde_json::Value, key: &str) -> Section {
    let Some(s) = summary.get(key) else {
        return Section::default();
    };
    Section {
        count: read_i64_field(s, key, "count"),
        covered: read_i64_field(s, key, "covered"),
        notcovered: read_i64_field(s, key, "notcovered"),
        percent: read_f64_field(s, key, "percent"),
    }
}

/// ERR-1: an absent field is legitimately empty (default); a field that is
/// present but the wrong shape (e.g. llvm-cov bumping `count` to a string or
/// float) is a schema-drift signal that must surface as a warn so coverage
/// does not silently drop to zero.
fn read_field<T: Default>(
    section: &serde_json::Value,
    section_key: &str,
    field: &str,
    accessor: impl FnOnce(&serde_json::Value) -> Option<T>,
    type_name: &'static str,
) -> T {
    match section.get(field) {
        None => T::default(),
        Some(v) => accessor(v).unwrap_or_else(|| {
            tracing::warn!(
                section = section_key,
                field,
                value = %v,
                "coverage field present but not {type_name}; coercing to default (llvm-cov schema drift?)"
            );
            T::default()
        }),
    }
}

fn read_i64_field(section: &serde_json::Value, section_key: &str, field: &str) -> i64 {
    read_field(
        section,
        section_key,
        field,
        serde_json::Value::as_i64,
        "an integer",
    )
}

fn read_f64_field(section: &serde_json::Value, section_key: &str, field: &str) -> f64 {
    read_field(
        section,
        section_key,
        field,
        serde_json::Value::as_f64,
        "a float",
    )
}

pub fn flatten_coverage_json(raw: &serde_json::Value) -> Result<serde_json::Value, anyhow::Error> {
    let data = raw
        .get("data")
        .and_then(|d| d.as_array())
        .context("missing or invalid 'data' array in coverage JSON")?;

    if data.is_empty() {
        anyhow::bail!("'data' array is empty in coverage JSON");
    }
    // ERR-1: cargo llvm-cov --json's `data` is an array (one entry per
    // export); future per-target merging produces multiple exports. Iterate
    // every entry instead of silently dropping data[1..].
    if data.len() > 1 {
        tracing::warn!(
            entries = data.len(),
            "coverage JSON contains more than one data export; flattening all entries"
        );
    }

    let empty = serde_json::json!({});
    let file_arrays: Vec<&Vec<serde_json::Value>> = data
        .iter()
        .map(|entry| {
            entry
                .get("files")
                .and_then(|f| f.as_array())
                .context("missing or invalid 'files' array in coverage data")
        })
        .collect::<Result<_, _>>()?;
    let total: usize = file_arrays.iter().map(|f| f.len()).sum();
    let mut records = Vec::with_capacity(total);
    for file in file_arrays.into_iter().flat_map(|f| f.iter()) {
        let filename = file.get("filename").and_then(|f| f.as_str()).unwrap_or("");
        let summary = file.get("summary").unwrap_or(&empty);

        let lines = extract_section(summary, "lines");
        let functions = extract_section(summary, "functions");
        let regions = extract_section(summary, "regions");
        let branches = extract_section(summary, "branches");

        records.push(serde_json::json!({
            "filename": filename,
            "lines_count": lines.count,
            "lines_covered": lines.covered,
            "lines_percent": lines.percent,
            "functions_count": functions.count,
            "functions_covered": functions.covered,
            "functions_percent": functions.percent,
            "regions_count": regions.count,
            "regions_covered": regions.covered,
            "regions_notcovered": regions.notcovered,
            "regions_percent": regions.percent,
            "branches_count": branches.count,
            "branches_covered": branches.covered,
            "branches_notcovered": branches.notcovered,
            "branches_percent": branches.percent,
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

/// Ingest coverage sidecar data into DuckDB and return the structured load
/// report.
///
/// READ-5 (TASK-0808): the previous signature returned `()` and silently
/// dropped the [`LoadResult`], leaving callers unable to distinguish a
/// zero-row load from a healthy one. The signature now surfaces the report;
/// a zero-record load is also logged at `warn` so even fire-and-forget
/// callers see the health signal.
pub fn load_coverage(data_dir: &Path, db: &DuckDb) -> Result<LoadResult, anyhow::Error> {
    init_schema(db)?;
    let ingestor = CoverageIngestor;
    let load_result = ingestor.load(data_dir, db)?;
    if load_result.record_count == 0 {
        tracing::warn!(
            source = load_result.source_name,
            data_dir = %data_dir.display(),
            "coverage load completed with zero records"
        );
    }
    Ok(load_result)
}
