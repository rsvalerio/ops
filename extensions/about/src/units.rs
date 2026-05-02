//! Stack-agnostic `about units` subpage: grid of project unit cards.
//!
//! Calls the `project_units` data provider registered by the active stack and
//! renders each returned [`ProjectUnit`] as a card.

use std::io::{IsTerminal, Write};

use ops_core::project_identity::ProjectUnit;
use ops_extension::{Context, DataRegistry};

use crate::cards::{layout_cards_in_grid_with_width, render_card};
use crate::providers::{load_or_default, warm_providers};
use crate::text_util::get_terminal_width;

pub const PROJECT_UNITS_PROVIDER: &str = "project_units";

pub fn run_about_units(data_registry: &DataRegistry) -> anyhow::Result<()> {
    let is_tty = std::io::stdout().is_terminal();
    // ERR-1 / TASK-0784: only the direct-stdout entry point probes the
    // terminal/`COLUMNS`. Buffer-writing call sites must hand in an explicit
    // width via `run_about_units_with` so the 120-column fallback never
    // sneaks into output destined for a `Vec<u8>` or pipe.
    run_about_units_with(
        data_registry,
        &mut std::io::stdout(),
        is_tty,
        get_terminal_width(),
    )
}

/// READ-5/TASK-0411: `is_tty` is supplied by the caller and reflects the
/// `writer` they hand in, not stdout. Passing a `Vec<u8>` writer with
/// `is_tty = false` guarantees no ANSI escapes regardless of stdout state.
///
/// ERR-1/TASK-0784: `term_width` is also caller-supplied — buffer-writing
/// call sites pick a width matching their destination instead of inheriting
/// the stdout TTY/`COLUMNS` probe, which silently falls back to 120 columns
/// in non-TTY contexts.
pub fn run_about_units_with(
    data_registry: &DataRegistry,
    writer: &mut dyn Write,
    is_tty: bool,
    term_width: usize,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = std::sync::Arc::new(ops_core::config::Config::empty());
    let mut ctx = Context::new(config, cwd);

    // Warm duckdb + tokei so the stack provider can enrich Rust-specific
    // fields (e.g. dep_count) and so we can fill loc/file_count below.
    warm_providers(&mut ctx, data_registry, &["duckdb", "tokei"], "units");

    let mut units: Vec<ProjectUnit> =
        load_or_default(&mut ctx, data_registry, PROJECT_UNITS_PROVIDER)?;

    if units.is_empty() {
        writeln!(writer, "No project units found.")?;
        return Ok(());
    }

    enrich_from_db(&ctx, &mut units);

    let cards: Vec<Vec<String>> = units.iter().map(|u| render_card(u, is_tty)).collect();

    let mut lines = vec![String::new()];
    lines.extend(layout_cards_in_grid_with_width(&cards, term_width));
    writeln!(writer, "{}", lines.join("\n"))?;
    Ok(())
}

/// Enrich `units` with LOC and file-count data sampled from the duckdb
/// `tokei_files` table.
///
/// ERR-1 (TASK-0431): the four underlying queries each acquire `db.lock()`
/// independently, so a concurrent ingestion that runs between samples can
/// leave per-crate sums inconsistent with the project totals shown in the
/// same render. This is accepted as a render-time visual artefact: the
/// alternative — holding one lock across all four queries — would require
/// reshaping the helper layer to take an already-held `&Connection`. About
/// pages re-render on every invocation, so a stale frame is self-correcting.
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

    let unit_count = units.len();
    // ERR-1 (TASK-0463): a query failure must NOT silently overwrite
    // provider-supplied unit fields with None. Track each query's outcome
    // separately and only enrich units when we actually have data.
    //
    // READ-5 (TASK-0786): collect per-query failures into a single
    // consolidated warn so an operator scanning logs sees one line that
    // names every field left stale, instead of four scattered messages
    // from which the partial-frame nature has to be reconstructed.
    let mut partial_failures: Vec<(&'static str, String)> = Vec::new();
    let locs = match ops_duckdb::sql::query_crate_loc(db, &per_crate_paths) {
        Ok(map) => Some(map),
        Err(e) => {
            partial_failures.push(("crate_loc", format!("{e:#}")));
            None
        }
    };
    let files = match ops_duckdb::sql::query_crate_file_count(db, &per_crate_paths) {
        Ok(map) => Some(map),
        Err(e) => {
            partial_failures.push(("crate_file_count", format!("{e:#}")));
            None
        }
    };
    let project_loc = match ops_duckdb::sql::query_project_loc(db) {
        Ok(v) => Some(v),
        Err(e) => {
            partial_failures.push(("project_loc", format!("{e:#}")));
            None
        }
    };
    let project_files = match ops_duckdb::sql::query_project_file_count(db) {
        Ok(v) => Some(v),
        Err(e) => {
            partial_failures.push(("project_file_count", format!("{e:#}")));
            None
        }
    };
    if !partial_failures.is_empty() {
        let fields: Vec<&'static str> = partial_failures.iter().map(|(f, _)| *f).collect();
        let detail = partial_failures
            .iter()
            .map(|(f, e)| format!("{f}: {e}"))
            .collect::<Vec<_>>()
            .join("; ");
        tracing::warn!(
            "about/units: rendered frame is partial across {unit_count} units; \
             leaving provider-supplied fields untouched for queries [{fields}]: {detail}",
            fields = fields.join(", "),
        );
    }

    for unit in units.iter_mut() {
        let is_root = unit.path.is_empty() || unit.path == ".";
        if unit.loc.is_none() {
            let candidate = if is_root {
                project_loc
            } else {
                locs.as_ref().and_then(|m| m.get(&unit.path).copied())
            };
            if candidate.is_some() {
                unit.loc = candidate;
            }
        }
        if unit.file_count.is_none() {
            let candidate = if is_root {
                project_files
            } else {
                files.as_ref().and_then(|m| m.get(&unit.path).copied())
            };
            if candidate.is_some() {
                unit.file_count = candidate;
            }
        }
    }
}

#[cfg(not(feature = "duckdb"))]
fn enrich_from_db(_ctx: &Context, _units: &mut [ProjectUnit]) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression for TASK-0431: when no DuckDB is wired up, enrich_from_db is
    /// a no-op and leaves caller-supplied unit fields untouched. Codifies the
    /// "independent samples are acceptable" contract — the function never
    /// fails the render pipeline even if the underlying data is inconsistent
    /// or absent.
    #[test]
    fn enrich_from_db_without_db_is_noop() {
        let cwd = std::env::current_dir().expect("cwd");
        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let ctx = Context::new(config, cwd);
        let mut u = ProjectUnit::new("demo", "demo");
        u.loc = Some(42);
        u.file_count = Some(7);
        let mut units = vec![u];
        enrich_from_db(&ctx, &mut units);
        assert_eq!(units[0].loc, Some(42));
        assert_eq!(units[0].file_count, Some(7));
    }
}
