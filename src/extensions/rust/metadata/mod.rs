//! Metadata extension: runs `cargo metadata` and provides workspace info as JSON.
//! DuckDB is the single source of truth - metadata is loaded into metadata_raw table.

mod ingestor;
#[cfg(test)]
mod tests;
mod types;
mod views;

pub use ingestor::MetadataIngestor;
pub use types::{Dependency, DependencyKind, Metadata, Package, Target};

use crate::config::Config;
use crate::extension::{
    Context, DataField, DataProvider, DataProviderError, DataProviderSchema, DataRegistry,
    Extension, ExtensionType,
};
use crate::extensions::ops_db::{init_schema, DataIngestor, OpsDb};
use crate::output::format_error_tail;
use anyhow::Context as AnyhowContext;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub const NAME: &str = "metadata";
#[allow(dead_code)]
pub const DESCRIPTION: &str = "Cargo metadata provider (workspace info, dependencies)";
#[allow(dead_code)]
pub const SHORTNAME: &str = "meta";
pub const DATA_PROVIDER_NAME: &str = "metadata";

pub fn run_cargo_metadata(working_dir: &Path) -> io::Result<Output> {
    Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(working_dir)
        .output()
}

pub fn check_metadata_output(output: &Output) -> Result<(), anyhow::Error> {
    if !output.status.success() {
        let tail = format_error_tail(&output.stderr, 5);
        anyhow::bail!("cargo metadata failed: {}", tail);
    }
    Ok(())
}

pub struct MetadataExtension;

impl Extension for MetadataExtension {
    fn name(&self) -> &'static str {
        NAME
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn shortname(&self) -> &'static str {
        SHORTNAME
    }

    fn types(&self) -> ExtensionType {
        ExtensionType::DATASOURCE
    }

    fn data_provider_name(&self) -> Option<&'static str> {
        Some(DATA_PROVIDER_NAME)
    }

    fn register_commands(&self, _registry: &mut crate::extension::CommandRegistry) {}

    fn register_data_providers(&self, registry: &mut DataRegistry) {
        registry.register(DATA_PROVIDER_NAME, Box::new(MetadataProvider));
    }
}

struct MetadataProvider;

impl DataProvider for MetadataProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let db_ref = ctx
            .db
            .as_ref()
            .and_then(|h| h.as_any().downcast_ref::<OpsDb>());
        if let Some(db) = db_ref {
            return provide_from_db(db, ctx).map_err(Into::into);
        }
        provide_via_cargo_metadata(ctx).map_err(Into::into)
    }

    fn schema(&self) -> DataProviderSchema {
        DataProviderSchema {
            description: "Cargo workspace metadata from `cargo metadata`",
            fields: vec![
                DataField {
                    name: "workspace_root",
                    type_name: "str",
                    description: "Absolute path to the workspace root directory",
                },
                DataField {
                    name: "target_directory",
                    type_name: "str",
                    description: "Absolute path to the build artifacts directory",
                },
                DataField {
                    name: "build_directory",
                    type_name: "Option<str>",
                    description: "Build directory if specified via config",
                },
                DataField {
                    name: "packages",
                    type_name: "Iterator<Package>",
                    description: "All packages in the dependency graph",
                },
                DataField {
                    name: "members",
                    type_name: "Iterator<Package>",
                    description: "Workspace member packages only",
                },
                DataField {
                    name: "default_members",
                    type_name: "Iterator<Package>",
                    description: "Default workspace member packages",
                },
                DataField {
                    name: "root_package",
                    type_name: "Option<Package>",
                    description: "Root package (None for virtual workspaces)",
                },
                DataField {
                    name: "package_by_name",
                    type_name: "fn(&str) -> Option<Package>",
                    description: "Find a package by name",
                },
                DataField {
                    name: "Package.name",
                    type_name: "str",
                    description: "Package name",
                },
                DataField {
                    name: "Package.version",
                    type_name: "str",
                    description: "Package version string",
                },
                DataField {
                    name: "Package.edition",
                    type_name: "str",
                    description: "Rust edition (e.g., 2021)",
                },
                DataField {
                    name: "Package.license",
                    type_name: "Option<str>",
                    description: "License identifier",
                },
                DataField {
                    name: "Package.dependencies",
                    type_name: "Iterator<Dependency>",
                    description: "Normal dependencies",
                },
                DataField {
                    name: "Package.dev_dependencies",
                    type_name: "Iterator<Dependency>",
                    description: "Dev dependencies",
                },
                DataField {
                    name: "Package.build_dependencies",
                    type_name: "Iterator<Dependency>",
                    description: "Build dependencies",
                },
                DataField {
                    name: "Package.targets",
                    type_name: "Iterator<Target>",
                    description: "All build targets (lib, bins, tests, examples, benches)",
                },
                DataField {
                    name: "Dependency.name",
                    type_name: "str",
                    description: "Dependency name",
                },
                DataField {
                    name: "Dependency.version_req",
                    type_name: "str",
                    description: "Version requirement (e.g., ^1.0)",
                },
                DataField {
                    name: "Dependency.kind",
                    type_name: "enum",
                    description: "Normal, Dev, or Build",
                },
                DataField {
                    name: "Dependency.features",
                    type_name: "Iterator<str>",
                    description: "Enabled features",
                },
                DataField {
                    name: "Target.name",
                    type_name: "str",
                    description: "Target name",
                },
                DataField {
                    name: "Target.kind",
                    type_name: "Iterator<str>",
                    description: "Target kinds (lib, bin, test, example, bench)",
                },
                DataField {
                    name: "Target.src_path",
                    type_name: "str",
                    description: "Source file path",
                },
            ],
        }
    }
}

fn data_dir_for_db(db_path: &Path) -> PathBuf {
    let mut path = db_path.as_os_str().to_os_string();
    path.push(".ingest");
    PathBuf::from(path)
}

fn metadata_raw_has_data(db: &OpsDb) -> Result<bool, anyhow::Error> {
    let conn = db
        .lock()
        .map_err(|e| anyhow::anyhow!("db lock failed: {}", e))
        .context("EFF-002: acquiring db lock for metadata_raw check")?;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'metadata_raw'",
            [],
            |row: &duckdb::Row| row.get(0),
        )
        .context("checking if metadata_raw table exists")?;
    if count == 0 {
        return Ok(false);
    }
    let row_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM metadata_raw",
            [],
            |row: &duckdb::Row| row.get(0),
        )
        .context("counting rows in metadata_raw")?;
    Ok(row_count > 0)
}

fn query_metadata_raw(db: &OpsDb) -> Result<serde_json::Value, anyhow::Error> {
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

fn provide_from_db(db: &OpsDb, ctx: &Context) -> Result<serde_json::Value, anyhow::Error> {
    if !metadata_raw_has_data(db)? {
        let data_dir = data_dir_for_db(db.path());
        refresh_metadata(ctx, &data_dir, db)?;
    }
    query_metadata_raw(db)
}

pub fn collect_metadata(ctx: &Context, data_dir: &Path) -> Result<(), anyhow::Error> {
    let ingestor = MetadataIngestor;
    ingestor.collect(ctx, data_dir)?;
    Ok(())
}

pub fn load_metadata(data_dir: &Path, db: &OpsDb) -> Result<(), anyhow::Error> {
    let json_path = data_dir.join("metadata.json");
    if !json_path.exists() {
        anyhow::bail!("metadata.json not found. Run `cargo ops metadata collect` first.");
    }
    init_schema(db)?;
    let ingestor = MetadataIngestor;
    ingestor.load(data_dir, db)?;
    Ok(())
}

pub fn refresh_metadata(ctx: &Context, data_dir: &Path, db: &OpsDb) -> Result<(), anyhow::Error> {
    collect_metadata(ctx, data_dir)?;
    load_metadata(data_dir, db)?;
    Ok(())
}

pub fn default_db_path(workspace_root: &Path) -> PathBuf {
    OpsDb::resolve_path(&Config::default().data, workspace_root)
}

pub fn default_data_dir(workspace_root: &Path) -> PathBuf {
    data_dir_for_db(&default_db_path(workspace_root))
}

fn provide_via_cargo_metadata(ctx: &Context) -> Result<serde_json::Value, anyhow::Error> {
    let output = run_cargo_metadata(&ctx.working_directory)?;
    check_metadata_output(&output)?;
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    Ok(json)
}
