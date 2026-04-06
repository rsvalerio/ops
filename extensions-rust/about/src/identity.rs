//! Rust-specific `project_identity` data provider.
//!
//! Reads Cargo.toml directly and queries DuckDB for LOC stats to build a
//! [`ProjectIdentity`](ops_core::project_identity::ProjectIdentity)
//! with Rust-specific fields (crates, edition, etc.).

use ops_cargo_toml::{CargoToml, CargoTomlProvider};
use ops_core::project_identity::ProjectIdentity;
use ops_extension::{Context, DataProvider, DataProviderError};

use crate::query::resolve_member_globs;

pub(crate) const PROVIDER_NAME: &str = "project_identity";

pub(crate) struct RustIdentityProvider;

impl DataProvider for RustIdentityProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        // Parse Cargo.toml directly (don't depend on registry ordering).
        let cargo_toml_value = CargoTomlProvider::new().provide(ctx)?;
        let mut manifest: CargoToml = serde_json::from_value(cargo_toml_value)
            .map_err(DataProviderError::computation_error)?;

        let cwd = ctx.working_directory.clone();

        // Expand workspace member globs
        if let Some(ws) = &mut manifest.workspace {
            ws.members = resolve_member_globs(&ws.members, &cwd);
        }

        let pkg = manifest.package.as_ref();
        let ws_pkg = manifest.workspace.as_ref().and_then(|w| w.package.as_ref());

        // Try [package] first, fall back to [workspace.package] for virtual workspaces.
        let name = pkg
            .map(|p| p.name.clone())
            .unwrap_or_else(|| dir_name(&cwd));

        let version = pkg
            .and_then(|p| p.version.as_str())
            .or(ws_pkg.and_then(|wp| wp.version.as_deref()))
            .map(|s| s.to_string());
        let description = pkg
            .and_then(|p| p.description.as_str())
            .or(ws_pkg.and_then(|wp| wp.description.as_deref()))
            .map(|s| s.to_string());
        let edition = pkg
            .and_then(|p| p.edition.as_str())
            .or(ws_pkg.and_then(|wp| wp.edition.as_deref()))
            .map(|s| s.to_string());
        let license = pkg
            .and_then(|p| p.license.as_str())
            .or(ws_pkg.and_then(|wp| wp.license.as_deref()))
            .map(|s| s.to_string());
        let repository = pkg
            .and_then(|p| p.repository.as_str())
            .or(ws_pkg.and_then(|wp| wp.repository.as_deref()))
            .map(|s| s.to_string());
        let authors = pkg
            .and_then(|p| p.authors.value())
            .cloned()
            .or_else(|| {
                ws_pkg
                    .filter(|wp| !wp.authors.is_empty())
                    .map(|wp| wp.authors.clone())
            })
            .unwrap_or_default();

        let module_count = manifest.workspace.as_ref().map(|w| w.members.len());
        let stack_detail = edition.map(|e| format!("Edition {e}"));

        // Try LOC from DuckDB if available
        let (loc, file_count) = query_loc_from_db(ctx);

        let identity = ProjectIdentity {
            name,
            version,
            description,
            stack_label: "Rust".to_string(),
            stack_detail,
            license,
            project_path: cwd.display().to_string(),
            module_count,
            module_label: "crates".to_string(),
            loc,
            file_count,
            authors,
            repository,
        };

        serde_json::to_value(&identity).map_err(DataProviderError::from)
    }
}

fn dir_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string())
}

/// Try to get project LOC and file count from DuckDB.
fn query_loc_from_db(ctx: &Context) -> (Option<i64>, Option<i64>) {
    let db = match ctx
        .db
        .as_ref()
        .and_then(|h| h.as_any().downcast_ref::<ops_duckdb::DuckDb>())
    {
        Some(db) => db,
        None => return (None, None),
    };

    let loc = ops_duckdb::sql::query_project_loc(db).ok();
    let files = ops_duckdb::sql::query_project_file_count(db).ok();
    (loc, files)
}
