//! Metadata extension: runs `cargo metadata` and provides workspace info as JSON.
//! DuckDB is the single source of truth - metadata is loaded into metadata_raw table.

mod ingestor;
#[cfg(test)]
mod tests;
mod types;
mod views;

pub use types::{Dependency, DependencyKind, Metadata, Package, Target};

use cargo_ops_core::output::format_error_tail;
use cargo_ops_duckdb::DuckDb;
use cargo_ops_extension::{
    Context, DataProvider, DataProviderError, DataProviderSchema, ExtensionType,
};
use ingestor::MetadataIngestor;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

const NAME: &str = "metadata";
const DESCRIPTION: &str = "Cargo metadata provider (workspace info, dependencies)";
const SHORTNAME: &str = "meta";
const DATA_PROVIDER_NAME: &str = "metadata";

pub(crate) fn run_cargo_metadata(working_dir: &Path) -> io::Result<Output> {
    Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(working_dir)
        .output()
}

pub(crate) fn check_metadata_output(output: &Output) -> Result<(), anyhow::Error> {
    if !output.status.success() {
        let tail = format_error_tail(&output.stderr, 5);
        anyhow::bail!("cargo metadata failed: {}", tail);
    }
    Ok(())
}

pub struct MetadataExtension;

cargo_ops_extension::impl_extension! {
    MetadataExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(MetadataProvider));
    },
}

struct MetadataProvider;

impl DataProvider for MetadataProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        cargo_ops_duckdb::try_provide_from_db(ctx, provide_from_db, |ctx| {
            provide_via_cargo_metadata(ctx)
        })
    }

    fn schema(&self) -> DataProviderSchema {
        use cargo_ops_extension::data_field;
        DataProviderSchema {
            description: "Cargo workspace metadata from `cargo metadata`",
            fields: vec![
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
        }
    }
}

fn query_metadata_raw(db: &DuckDb) -> Result<serde_json::Value, anyhow::Error> {
    use anyhow::Context as AnyhowContext;
    let conn = db.lock().context("acquiring db lock for metadata query")?;
    let json_text: String = conn
        .query_row(
            "SELECT to_json(m)::VARCHAR FROM metadata_raw m LIMIT 1",
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
    cargo_ops_duckdb::sql::provide_via_ingestor(
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
