//! Generic about command: displays project identity card for any stack.
//!
//! Stack-specific extensions provide a `"project_identity"` data provider
//! returning [`ops_core::project_identity::ProjectIdentity`] as JSON.
//! When no provider is available, a minimal identity is built from the filesystem.

use std::io::IsTerminal;

use ops_core::project_identity::{AboutCard, ProjectIdentity};
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
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = std::sync::Arc::new(ops_core::config::Config::default());
    let mut ctx = ops_extension::Context::new(config, cwd.clone());
    if opts.refresh {
        ctx.refresh = true;
    }

    // Pre-initialize generic data sources so stack providers can query them.
    // These are best-effort — they may not be compiled in.
    let _ = ctx.get_or_provide("duckdb", data_registry);
    let _ = ctx.get_or_provide("tokei", data_registry);

    let mut identity = match ctx.get_or_provide("project_identity", data_registry) {
        Ok(value) => serde_json::from_value::<ProjectIdentity>((*value).clone())?,
        Err(DataProviderError::NotFound(_)) => build_fallback_identity(&cwd),
        Err(e) => return Err(e.into()),
    };

    // Enrich with DuckDB data if the provider didn't include it.
    if identity.loc.is_none()
        || identity.dependency_count.is_none()
        || identity.coverage_percent.is_none()
        || identity.languages.is_empty()
    {
        enrich_from_db(&ctx, &mut identity);
    }

    let card = AboutCard::from_identity_filtered(&identity, opts.visible_fields.as_deref());
    let is_tty = std::io::stdout().is_terminal();
    println!("{}", card.render(columns, is_tty));

    Ok(())
}

/// Enrich identity with LOC/file count from DuckDB if available.
#[cfg(feature = "duckdb")]
fn enrich_from_db(ctx: &ops_extension::Context, identity: &mut ProjectIdentity) {
    let db = match ctx
        .db
        .as_ref()
        .and_then(|h| h.as_any().downcast_ref::<ops_duckdb::DuckDb>())
    {
        Some(db) => db,
        None => return,
    };

    if let Ok(loc) = ops_duckdb::sql::query_project_loc(db) {
        identity.loc = Some(loc);
    }
    if let Ok(files) = ops_duckdb::sql::query_project_file_count(db) {
        if files > 0 {
            identity.file_count = Some(files);
        }
    }
    if identity.dependency_count.is_none() {
        if let Ok(count) = ops_duckdb::sql::query_dependency_count(db) {
            if count > 0 {
                identity.dependency_count = Some(count);
            }
        }
    }
    if identity.coverage_percent.is_none() {
        if let Ok(cov) = ops_duckdb::sql::query_project_coverage(db) {
            if cov.lines_count > 0 {
                identity.coverage_percent = Some(cov.lines_percent);
            }
        }
    }
    if identity.languages.is_empty() {
        if let Ok(langs) = ops_duckdb::sql::query_project_languages(db) {
            identity.languages = langs;
        }
    }
}

#[cfg(not(feature = "duckdb"))]
fn enrich_from_db(_ctx: &ops_extension::Context, _identity: &mut ProjectIdentity) {}

/// Build a minimal identity from the filesystem when no stack provider exists.
fn build_fallback_identity(cwd: &std::path::Path) -> ProjectIdentity {
    use ops_core::stack::Stack;

    let name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let stack = Stack::detect(cwd);
    let stack_label = stack
        .map(|s| capitalize(s.as_str()))
        .unwrap_or_else(|| "Generic".to_string());

    ProjectIdentity {
        name,
        version: None,
        description: None,
        stack_label,
        stack_detail: None,
        license: None,
        project_path: cwd.display().to_string(),
        module_count: None,
        module_label: "modules".to_string(),
        loc: None,
        file_count: None,
        authors: vec![],
        repository: None,
        homepage: None,
        msrv: None,
        dependency_count: None,
        coverage_percent: None,
        languages: vec![],
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
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
}
