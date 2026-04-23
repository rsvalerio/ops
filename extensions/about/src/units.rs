//! Stack-agnostic `about units` subpage: grid of project unit cards.
//!
//! Calls the `project_units` data provider registered by the active stack and
//! renders each returned [`ProjectUnit`] as a card.

use std::io::{IsTerminal, Write};

use ops_core::project_identity::ProjectUnit;
use ops_extension::{Context, DataProviderError, DataRegistry};

use crate::cards::{layout_cards_in_grid, render_card};

pub const PROJECT_UNITS_PROVIDER: &str = "project_units";

pub fn run_about_units(data_registry: &DataRegistry) -> anyhow::Result<()> {
    run_about_units_with(data_registry, &mut std::io::stdout())
}

pub fn run_about_units_with(
    data_registry: &DataRegistry,
    writer: &mut dyn Write,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = std::sync::Arc::new(ops_core::config::Config::default());
    let mut ctx = Context::new(config, cwd);

    // Warm duckdb + tokei so the stack provider can enrich Rust-specific
    // fields (e.g. dep_count) and so we can fill loc/file_count below.
    // NotFound is expected per stack; anything else is logged for debugging.
    for provider in ["duckdb", "tokei"] {
        match ctx.get_or_provide(provider, data_registry) {
            Ok(_) | Err(DataProviderError::NotFound(_)) => {}
            Err(e) => tracing::debug!("about/units: warm-up {provider} failed: {e:#}"),
        }
    }

    let mut units = match ctx.get_or_provide(PROJECT_UNITS_PROVIDER, data_registry) {
        Ok(value) => serde_json::from_value::<Vec<ProjectUnit>>((*value).clone())?,
        Err(DataProviderError::NotFound(_)) => Vec::new(),
        Err(e) => return Err(e.into()),
    };

    if units.is_empty() {
        writeln!(writer, "No project units found.")?;
        return Ok(());
    }

    enrich_from_db(&ctx, &mut units);

    let is_tty = std::io::stdout().is_terminal();
    let cards: Vec<Vec<String>> = units.iter().map(|u| render_card(u, is_tty)).collect();

    let mut lines = vec![String::new()];
    lines.extend(layout_cards_in_grid(&cards));
    writeln!(writer, "{}", lines.join("\n"))?;
    Ok(())
}

#[cfg(feature = "duckdb")]
fn enrich_from_db(ctx: &Context, units: &mut [ProjectUnit]) {
    let Some(db) = ops_duckdb::get_db(ctx) else {
        return;
    };
    // Root-module entries (path == "" or ".") need project-wide totals; the
    // per-crate `starts_with(file, path || '/')` join never matches them.
    let per_crate_paths: Vec<&str> = units
        .iter()
        .map(|u| u.path.as_str())
        .filter(|p| !p.is_empty() && *p != ".")
        .collect();

    let locs = ops_duckdb::sql::query_crate_loc(db, &per_crate_paths).unwrap_or_else(|e| {
        tracing::warn!("about/units: query_crate_loc failed: {e:#}");
        Default::default()
    });
    let files = ops_duckdb::sql::query_crate_file_count(db, &per_crate_paths).unwrap_or_else(|e| {
        tracing::warn!("about/units: query_crate_file_count failed: {e:#}");
        Default::default()
    });
    let project_loc = match ops_duckdb::sql::query_project_loc(db) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("about/units: query_project_loc failed: {e:#}");
            None
        }
    };
    let project_files = match ops_duckdb::sql::query_project_file_count(db) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("about/units: query_project_file_count failed: {e:#}");
            None
        }
    };

    for unit in units.iter_mut() {
        let is_root = unit.path.is_empty() || unit.path == ".";
        if unit.loc.is_none() {
            unit.loc = if is_root {
                project_loc
            } else {
                locs.get(&unit.path).copied()
            };
        }
        if unit.file_count.is_none() {
            unit.file_count = if is_root {
                project_files
            } else {
                files.get(&unit.path).copied()
            };
        }
    }
}

#[cfg(not(feature = "duckdb"))]
fn enrich_from_db(_ctx: &Context, _units: &mut [ProjectUnit]) {}
