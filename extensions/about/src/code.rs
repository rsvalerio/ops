//! Stack-agnostic `about code` subpage: language statistics table.
//!
//! Reads language LOC and file counts from the `tokei_files` DuckDB view
//! (populated by the tokei data provider) and renders a table.

use std::io::Write;

use ops_core::project_identity::LanguageStat;
use ops_core::table::{Cell, OpsTable};
use ops_core::text::format_number;
use ops_extension::{Context, DataRegistry};

use crate::providers::warm_providers;

/// ARCH-2 / TASK-0370: delegate to the shared `query_project_languages`
/// helper so the `about code` page and any other LOC consumer share one
/// implementation. The previous inline aggregate query lacked
/// percentages, used a different `LanguageStat` shape, and could drift
/// from the canonical query without anyone noticing.
pub fn query_language_stats(
    ctx: &mut Context,
    data_registry: &DataRegistry,
) -> Option<Vec<LanguageStat>> {
    warm_providers(ctx, data_registry, &["duckdb", "tokei"], "code");

    let db = ops_duckdb::get_db(ctx)?;
    match ops_duckdb::sql::query_project_languages(db) {
        Ok(stats) if stats.is_empty() => None,
        Ok(stats) => Some(stats),
        Err(e) => {
            tracing::warn!("language_stats: query_project_languages failed: {e:#}");
            None
        }
    }
}

pub fn format_language_stats_section(stats: Option<&[LanguageStat]>) -> Vec<String> {
    let stats = match stats {
        Some(s) if !s.is_empty() => s,
        _ => return vec![],
    };

    let mut table = OpsTable::new();
    table.set_header(vec!["Language", "Lines of Code", "Files"]);

    for stat in stats {
        let loc_str = format!("{} ({:.1}%)", format_number(stat.loc), stat.loc_pct);
        let files_str = format!("{} ({:.1}%)", format_number(stat.files), stat.files_pct);
        table.add_row(vec![
            Cell::new(&stat.name),
            Cell::new(&loc_str),
            Cell::new(&files_str),
        ]);
    }

    let mut lines = vec![String::new()];
    lines.extend(table.to_string().lines().map(|l| format!("    {l}")));
    lines
}

pub fn run_about_code(data_registry: &DataRegistry) -> anyhow::Result<()> {
    run_about_code_with(data_registry, &mut std::io::stdout())
}

pub fn run_about_code_with(
    data_registry: &DataRegistry,
    writer: &mut dyn Write,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = std::sync::Arc::new(ops_core::config::Config::empty());
    let mut ctx = Context::new(config, cwd);

    let stats = query_language_stats(&mut ctx, data_registry);
    let lines = format_language_stats_section(stats.as_deref());
    writeln!(writer, "{}", lines.join("\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_language_stats_section_empty() {
        assert!(format_language_stats_section(None).is_empty());
        assert!(format_language_stats_section(Some(&[])).is_empty());
    }

    #[test]
    fn format_language_stats_section_with_data() {
        let stats = vec![
            LanguageStat::new("Rust", 8000, 40, 97.6, 88.9),
            LanguageStat::new("TOML", 200, 5, 2.4, 11.1),
        ];
        let output = format_language_stats_section(Some(&stats)).join("\n");
        assert!(output.contains("Rust"));
        assert!(output.contains("TOML"));
    }

    #[test]
    fn format_language_stats_section_single_language_shows_100_percent() {
        let stats = vec![LanguageStat::new("Rust", 5000, 25, 100.0, 100.0)];
        let output = format_language_stats_section(Some(&stats)).join("\n");
        assert!(output.contains("100.0%"));
    }

    /// Regression for ERR-4: a poisoned DuckDb mutex must produce graceful
    /// `None` (not a panic) and exercise the warn-and-continue path. We
    /// verify behavior; tracing assertion would require pulling in
    /// `tracing-subscriber` as a dev-dep, which we deliberately avoid for a
    /// single test (see also TASK-0154).
    #[test]
    fn query_language_stats_returns_none_when_db_lock_poisoned() {
        use ops_core::config::Config;
        use ops_extension::{Context, DataRegistry};
        use std::sync::Arc;

        let db = Arc::new(ops_duckdb::DuckDb::open_in_memory().expect("db"));
        ops_duckdb::init_schema(&db).expect("init_schema");

        // Poison the inner Mutex by panicking inside a guard.
        let poisoner = Arc::clone(&db);
        let _ = std::thread::spawn(move || {
            let _guard = poisoner.lock().expect("lock");
            panic!("intentional poison");
        })
        .join();

        // Lock now poisoned — query path should log + return None.
        let config = Arc::new(Config::empty());
        let mut ctx = Context::new(config, std::path::PathBuf::from("/tmp"));
        ctx.db = Some(db);

        let registry = DataRegistry::new();
        assert!(query_language_stats(&mut ctx, &registry).is_none());
    }

    #[test]
    fn format_language_stats_section_percentages_add_up() {
        let stats = vec![
            LanguageStat::new("Rust", 750, 10, 75.0, 66.6),
            LanguageStat::new("TOML", 250, 5, 25.0, 33.3),
        ];
        let output = format_language_stats_section(Some(&stats)).join("\n");
        assert!(output.contains("75.0%"));
        assert!(output.contains("25.0%"));
    }

    /// DUP-1 regression (TASK-0549): when the upstream `loc_pct` reflects a
    /// full-project denominator that includes languages elided from the
    /// rendered slice (e.g. sub-0.1% languages dropped before display), the
    /// renderer must surface the stored `loc_pct` rather than re-deriving it
    /// from the truncated subset. Likewise `files_pct` must be rendered.
    #[test]
    fn format_language_stats_section_uses_stored_pct_after_filter() {
        // Upstream computed pcts against a denominator of 10_000 LOC / 100
        // files, but only the top two languages are passed to the renderer.
        // A naive recompute over the slice would yield 80%/20%; the stored
        // values (60%/30%) must win.
        let stats = vec![
            LanguageStat::new("Rust", 6000, 60, 60.0, 60.0),
            LanguageStat::new("TOML", 1500, 30, 15.0, 30.0),
        ];
        let output = format_language_stats_section(Some(&stats)).join("\n");
        assert!(
            output.contains("60.0%"),
            "stored loc_pct expected: {output}"
        );
        assert!(
            output.contains("15.0%"),
            "stored loc_pct expected: {output}"
        );
        assert!(output.contains("30.0%"), "files_pct expected: {output}");
        assert!(
            !output.contains("80.0%"),
            "renderer must not recompute pct from truncated slice: {output}"
        );
    }
}
