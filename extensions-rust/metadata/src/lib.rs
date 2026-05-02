//! Metadata extension: runs `cargo metadata` and provides workspace info as JSON.
//! DuckDB is the single source of truth - metadata is loaded into metadata_raw table.

mod ingestor;
#[cfg(test)]
mod tests;
mod types;
mod views;

pub use types::{Dependency, DependencyKind, Metadata, Package, Target};

use ingestor::MetadataIngestor;
use ops_core::output::format_error_tail;
use ops_core::subprocess::{run_cargo, RunError};
use ops_duckdb::DuckDb;
use ops_extension::{Context, DataProvider, DataProviderError, DataProviderSchema, ExtensionType};
use std::path::Path;
use std::process::Output;
use std::time::Duration;

const NAME: &str = "metadata";
const DESCRIPTION: &str = "Cargo metadata provider (workspace info, dependencies)";
const SHORTNAME: &str = "meta";
const DATA_PROVIDER_NAME: &str = "metadata";

/// Default timeout for `cargo metadata`; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`.
pub(crate) const CARGO_METADATA_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) fn run_cargo_metadata(working_dir: &Path) -> Result<Output, RunError> {
    run_cargo(
        &["metadata", "--format-version", "1"],
        working_dir,
        CARGO_METADATA_TIMEOUT,
        "cargo metadata",
    )
}

pub(crate) fn check_metadata_output(output: &Output) -> Result<(), anyhow::Error> {
    if !output.status.success() {
        let tail = format_error_tail(&output.stderr, 5);
        anyhow::bail!("cargo metadata failed: {}", tail);
    }
    Ok(())
}

/// API-9 / TASK-0922: construct via the registered extension factory only.
#[non_exhaustive]
pub struct MetadataExtension;

ops_extension::impl_extension! {
    MetadataExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Rust),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(MetadataProvider));
    },
    factory: METADATA_FACTORY = |_, _| {
        Some((NAME, Box::new(MetadataExtension)))
    },
}

struct MetadataProvider;

impl DataProvider for MetadataProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        ops_duckdb::try_provide_from_db(ctx, provide_from_db, |ctx| provide_via_cargo_metadata(ctx))
    }

    fn schema(&self) -> DataProviderSchema {
        use ops_extension::data_field;
        DataProviderSchema::new(
            "Cargo workspace metadata from `cargo metadata`",
            vec![
                data_field!(
                    "workspace_root",
                    "str",
                    "Absolute path to the workspace root directory"
                ),
                data_field!(
                    "target_directory",
                    "str",
                    "Absolute path to the build artifacts directory"
                ),
                data_field!(
                    "build_directory",
                    "Option<str>",
                    "Build directory if specified via config"
                ),
                data_field!(
                    "packages",
                    "Iterator<Package>",
                    "All packages in the dependency graph"
                ),
                data_field!(
                    "members",
                    "Iterator<Package>",
                    "Workspace member packages only"
                ),
                data_field!(
                    "default_members",
                    "Iterator<Package>",
                    "Default workspace member packages"
                ),
                data_field!(
                    "root_package",
                    "Option<Package>",
                    "Root package (None for virtual workspaces)"
                ),
                data_field!(
                    "package_by_name",
                    "fn(&str) -> Option<Package>",
                    "Find a package by name"
                ),
                data_field!("Package.name", "str", "Package name"),
                data_field!("Package.version", "str", "Package version string"),
                data_field!("Package.edition", "str", "Rust edition (e.g., 2021)"),
                data_field!("Package.license", "Option<str>", "License identifier"),
                data_field!(
                    "Package.dependencies",
                    "Iterator<Dependency>",
                    "Normal dependencies"
                ),
                data_field!(
                    "Package.dev_dependencies",
                    "Iterator<Dependency>",
                    "Dev dependencies"
                ),
                data_field!(
                    "Package.build_dependencies",
                    "Iterator<Dependency>",
                    "Build dependencies"
                ),
                data_field!(
                    "Package.targets",
                    "Iterator<Target>",
                    "All build targets (lib, bins, tests, examples, benches)"
                ),
                data_field!("Dependency.name", "str", "Dependency name"),
                data_field!(
                    "Dependency.version_req",
                    "str",
                    "Version requirement (e.g., ^1.0)"
                ),
                data_field!("Dependency.kind", "enum", "Normal, Dev, or Build"),
                data_field!("Dependency.features", "Iterator<str>", "Enabled features"),
                data_field!("Target.name", "str", "Target name"),
                data_field!(
                    "Target.kind",
                    "Iterator<str>",
                    "Target kinds (lib, bin, test, example, bench)"
                ),
                data_field!("Target.src_path", "str", "Source file path"),
            ],
        )
    }
}

fn query_metadata_raw(db: &DuckDb) -> Result<serde_json::Value, anyhow::Error> {
    use anyhow::Context as AnyhowContext;
    let conn = db.lock().context("acquiring db lock for metadata query")?;
    // ERR-1 / TASK-0599: `metadata_raw` is a singleton table — the prior
    // `LIMIT 1` form silently picked an arbitrary row if a future ingest
    // path inserted more than one (re-collect without truncate, schema
    // version row). Read every row, assert one, and surface a clear error
    // if the invariant breaks.
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM metadata_raw",
            [],
            |row: &duckdb::Row| row.get(0),
        )
        .context("counting metadata_raw rows")?;
    anyhow::ensure!(
        count == 1,
        "metadata_raw must contain exactly one row, found {count}"
    );
    let json_text: String = conn
        .query_row(
            "SELECT to_json(m)::VARCHAR FROM metadata_raw m",
            [],
            |row: &duckdb::Row| row.get(0),
        )
        .context("reading from metadata_raw table")?;
    drop(conn);
    let json: serde_json::Value =
        serde_json::from_str(&json_text).context("parsing metadata JSON")?;
    Ok(json)
}

fn provide_from_db(db: &DuckDb, ctx: &Context) -> Result<serde_json::Value, anyhow::Error> {
    ops_duckdb::sql::provide_via_ingestor(
        db,
        ctx,
        "metadata_raw",
        &MetadataIngestor,
        query_metadata_raw,
    )
}

fn provide_via_cargo_metadata(ctx: &Context) -> Result<serde_json::Value, anyhow::Error> {
    let output = run_cargo_metadata(&ctx.working_directory)?;
    check_metadata_output(&output)?;
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    Ok(json)
}
