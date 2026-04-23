//! Generic about command: displays project identity card for any stack.
//!
//! Stack-specific extensions provide a `"project_identity"` data provider
//! returning [`ops_core::project_identity::ProjectIdentity`] as JSON.
//! When no provider is available, a minimal identity is built from the filesystem.

pub mod cards;
pub mod coverage;
pub mod deps;
pub mod text_util;
pub mod units;
pub use coverage::run_about_coverage;
pub use deps::run_about_deps;
pub use units::run_about_units;

#[cfg(feature = "duckdb")]
pub mod code;
#[cfg(feature = "duckdb")]
pub use code::run_about_code;

use std::io::{IsTerminal, Write};
use std::path::Path;

use ops_core::project_identity::{AboutCard, ProjectIdentity};
use ops_core::text::{capitalize, dir_name};
use ops_extension::{DataProviderError, ExtensionType};

const NAME: &str = "about";
const DESCRIPTION: &str = "Project identity card";
const SHORTNAME: &str = "about";

pub struct AboutExtension;

ops_extension::impl_extension! {
    AboutExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::COMMAND,
    command_names: &["about"],
    data_provider_name: None,
    register_commands: |_self, registry| {
        registry.insert(
            "about".into(),
            ops_core::config::CommandSpec::Exec(ops_core::config::ExecCommandSpec {
                program: "ops".to_string(),
                args: vec!["about".to_string()],
                ..Default::default()
            }),
        );
    },
    register_data_providers: |_self, _registry| {},
    factory: ABOUT_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutExtension)))
    },
}

/// Options for the about command.
pub struct AboutOptions {
    pub refresh: bool,
    pub visible_fields: Option<Vec<String>>,
}

/// Run the generic about command.
///
/// Tries the `"project_identity"` data provider first. If no stack-specific
/// provider is registered, builds a minimal identity from the filesystem.
pub fn run_about(
    data_registry: &ops_extension::DataRegistry,
    opts: &AboutOptions,
    columns: u16,
    cwd: &Path,
    writer: &mut dyn Write,
) -> anyhow::Result<()> {
    let config = std::sync::Arc::new(ops_core::config::Config::default());
    let mut ctx = ops_extension::Context::new(config, cwd.to_path_buf());
    ctx.refresh = opts.refresh;

    warm_generic_providers(&mut ctx, data_registry, opts.refresh);
    let mut identity = resolve_identity(&mut ctx, data_registry, cwd)?;

    if identity.loc.is_none()
        || identity.dependency_count.is_none()
        || identity.coverage_percent.is_none()
        || identity.languages.is_empty()
    {
        enrich_from_db(&ctx, &mut identity);
    }

    let card = AboutCard::from_identity_filtered(&identity, opts.visible_fields.as_deref());
    let is_tty = std::io::stdout().is_terminal();
    writeln!(writer, "{}", card.render(columns, is_tty))?;

    Ok(())
}

fn warm_generic_providers(
    ctx: &mut ops_extension::Context,
    data_registry: &ops_extension::DataRegistry,
    refresh: bool,
) {
    let _ = ctx.get_or_provide("duckdb", data_registry);
    let _ = ctx.get_or_provide("tokei", data_registry);
    if refresh {
        match ctx.get_or_provide("coverage", data_registry) {
            Ok(_) | Err(DataProviderError::NotFound(_)) => {}
            Err(e) => tracing::warn!("coverage collection failed: {e:#}"),
        }
    }
}

fn resolve_identity(
    ctx: &mut ops_extension::Context,
    data_registry: &ops_extension::DataRegistry,
    cwd: &Path,
) -> anyhow::Result<ProjectIdentity> {
    match ctx.get_or_provide("project_identity", data_registry) {
        Ok(value) => Ok(serde_json::from_value::<ProjectIdentity>((*value).clone())?),
        Err(DataProviderError::NotFound(_)) => Ok(build_fallback_identity(cwd)),
        Err(e) => Err(e.into()),
    }
}

/// Enrich identity with LOC/file count from DuckDB if available.
#[cfg(feature = "duckdb")]
fn enrich_from_db(ctx: &ops_extension::Context, identity: &mut ProjectIdentity) {
    let db = match ops_duckdb::get_db(ctx) {
        Some(db) => db,
        None => return,
    };

    match ops_duckdb::sql::query_project_loc(db) {
        Ok(loc) => identity.loc = Some(loc),
        Err(e) => tracing::warn!("about: query_project_loc failed: {e:#}"),
    }
    match ops_duckdb::sql::query_project_file_count(db) {
        Ok(files) if files > 0 => identity.file_count = Some(files),
        Ok(_) => {}
        Err(e) => tracing::warn!("about: query_project_file_count failed: {e:#}"),
    }
    if identity.dependency_count.is_none() {
        match ops_duckdb::sql::query_dependency_count(db) {
            Ok(count) if count > 0 => identity.dependency_count = Some(count),
            Ok(_) => {}
            Err(e) => tracing::warn!("about: query_dependency_count failed: {e:#}"),
        }
    }
    if identity.coverage_percent.is_none() {
        match ops_duckdb::sql::query_project_coverage(db) {
            Ok(cov) if cov.lines_count > 0 => {
                identity.coverage_percent = Some(cov.lines_percent);
            }
            Ok(_) => {}
            Err(e) => tracing::warn!("about: query_project_coverage failed: {e:#}"),
        }
    }
    if identity.languages.is_empty() {
        match ops_duckdb::sql::query_project_languages(db) {
            Ok(langs) => identity.languages = langs,
            Err(e) => tracing::warn!("about: query_project_languages failed: {e:#}"),
        }
    }
}

#[cfg(not(feature = "duckdb"))]
fn enrich_from_db(_ctx: &ops_extension::Context, _identity: &mut ProjectIdentity) {}

/// Build a minimal identity from the filesystem when no stack provider exists.
fn build_fallback_identity(cwd: &std::path::Path) -> ProjectIdentity {
    use ops_core::stack::Stack;

    let name = dir_name(cwd).to_string();

    let stack = Stack::detect(cwd);
    let stack_label = stack
        .map(|s| capitalize(s.as_str()))
        .unwrap_or_else(|| "Generic".to_string());

    ProjectIdentity {
        name,
        stack_label,
        project_path: cwd.display().to_string(),
        module_label: "modules".to_string(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_identity_uses_dir_name() {
        let cwd = std::path::Path::new("/tmp/my-project");
        let id = build_fallback_identity(cwd);
        assert_eq!(id.name, "my-project");
        assert_eq!(id.project_path, "/tmp/my-project");
        assert!(id.version.is_none());
        assert!(id.module_count.is_none());
    }

    #[test]
    fn fallback_identity_module_label_is_modules() {
        let cwd = std::path::Path::new("/tmp/test");
        let id = build_fallback_identity(cwd);
        assert_eq!(id.module_label, "modules");
    }

    #[test]
    fn fallback_identity_defaults_are_empty() {
        let cwd = std::path::Path::new("/tmp/test");
        let id = build_fallback_identity(cwd);
        assert!(id.description.is_none());
        assert!(id.license.is_none());
        assert!(id.repository.is_none());
        assert!(id.homepage.is_none());
        assert!(id.msrv.is_none());
        assert!(id.loc.is_none());
        assert!(id.file_count.is_none());
        assert!(id.dependency_count.is_none());
        assert!(id.coverage_percent.is_none());
        assert!(id.languages.is_empty());
        assert!(id.authors.is_empty());
        assert!(id.stack_detail.is_none());
    }

    #[test]
    fn fallback_identity_detects_rust_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        let id = build_fallback_identity(dir.path());
        assert_eq!(id.stack_label, "Rust");
    }

    #[test]
    fn fallback_identity_detects_go_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module test").unwrap();
        let id = build_fallback_identity(dir.path());
        assert_eq!(id.stack_label, "Go");
    }

    #[test]
    fn fallback_identity_detects_node_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        let id = build_fallback_identity(dir.path());
        assert_eq!(id.stack_label, "Node");
    }

    #[test]
    fn fallback_identity_detects_python_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        let id = build_fallback_identity(dir.path());
        assert_eq!(id.stack_label, "Python");
    }

    #[test]
    fn fallback_identity_generic_when_no_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let id = build_fallback_identity(dir.path());
        assert_eq!(id.stack_label, "Generic");
    }

    #[test]
    fn fallback_identity_name_from_dir() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let id = build_fallback_identity(dir.path());
        assert_eq!(id.name, expected);
    }

    #[test]
    fn about_options_fields() {
        let opts = AboutOptions {
            refresh: true,
            visible_fields: Some(vec!["project".to_string(), "codebase".to_string()]),
        };
        assert!(opts.refresh);
        assert_eq!(opts.visible_fields.unwrap().len(), 2);

        let opts_default = AboutOptions {
            refresh: false,
            visible_fields: None,
        };
        assert!(!opts_default.refresh);
        assert!(opts_default.visible_fields.is_none());
    }

    #[cfg(feature = "duckdb")]
    #[test]
    fn enrich_from_db_logs_and_defaults_when_tables_missing() {
        // No tokei/coverage/metadata tables exist — every query returns 0 / empty
        // and per-query failures are warned (we exercise the fallible branches).
        let db = ops_duckdb::DuckDb::open_in_memory().expect("open in-memory db");
        ops_duckdb::init_schema(&db).expect("init_schema");

        let config = std::sync::Arc::new(ops_core::config::Config::default());
        let mut ctx = ops_extension::Context::new(config, std::path::PathBuf::from("/tmp"));
        ctx.db = Some(std::sync::Arc::new(db));

        let mut identity = ProjectIdentity::default();
        enrich_from_db(&ctx, &mut identity);

        // Defaults preserved when underlying tables are absent
        assert_eq!(identity.loc, Some(0));
        assert!(identity.file_count.is_none());
        assert!(identity.dependency_count.is_none());
        assert!(identity.coverage_percent.is_none());
        assert!(identity.languages.is_empty());
    }

    #[cfg(not(feature = "duckdb"))]
    #[test]
    fn enrich_from_db_noop_without_duckdb() {
        let config = std::sync::Arc::new(ops_core::config::Config::default());
        let ctx = ops_extension::Context::new(config, std::path::PathBuf::from("/tmp"));
        let mut identity = ProjectIdentity::default();
        enrich_from_db(&ctx, &mut identity);
        // Should be a no-op — all fields remain default
        assert!(identity.loc.is_none());
        assert!(identity.file_count.is_none());
        assert!(identity.dependency_count.is_none());
        assert!(identity.coverage_percent.is_none());
        assert!(identity.languages.is_empty());
    }
}
