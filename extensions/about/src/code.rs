//! Stack-agnostic `about code` subpage: language statistics table.
//!
//! Reads language LOC and file counts from the `tokei_files` DuckDB view
//! (populated by the tokei data provider) and renders a table.

use std::io::Write;

use ops_core::table::{Cell, OpsTable};
use ops_core::text::format_number;
use ops_extension::{Context, DataRegistry};

/// Per-language LOC breakdown.
pub struct LanguageStat {
    pub language: String,
    pub loc: i64,
    pub file_count: i64,
}

pub fn query_language_stats(
    ctx: &mut Context,
    data_registry: &DataRegistry,
) -> Option<Vec<LanguageStat>> {
    if let Err(e) = ctx.get_or_provide("duckdb", data_registry) {
        tracing::debug!("language_stats: duckdb provider failed: {e:#}");
        return None;
    }
    if let Err(e) = ctx.get_or_provide("tokei", data_registry) {
        tracing::debug!("language_stats: tokei provider failed: {e:#}");
        return None;
    }

    let db = ops_duckdb::get_db(ctx)?;
    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => {
            // Mutex poisoning is a correctness failure (a previous holder
            // panicked mid-transaction); surface at error so operators do
            // not miss it. See TASK-0262 / TASK-0196.
            tracing::error!(
                db_path = %db.path().display(),
                "language_stats: db lock poisoned: {e}"
            );
            return None;
        }
    };

    let mut stmt = match conn.prepare(
        "SELECT language, SUM(code) as loc, COUNT(*) as file_count \
         FROM tokei_files GROUP BY language ORDER BY loc DESC",
    ) {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!("language_stats: prepare failed: {e:#}");
            return None;
        }
    };

    let rows = match stmt.query_map([], |row| {
        Ok(LanguageStat {
            language: row.get(0)?,
            loc: row.get(1)?,
            file_count: row.get(2)?,
        })
    }) {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!("language_stats: query_map failed: {e:#}");
            return None;
        }
    };

    let mut stats = Vec::new();
    for row in rows {
        match row {
            Ok(s) => stats.push(s),
            Err(e) => {
                tracing::warn!("language_stats: row decode failed: {e:#}");
                return None;
            }
        }
    }
    // CONC-1: scope the critical section explicitly. `stats` owns its data;
    // the lock is no longer needed for empty-check / return.
    drop(stmt);
    drop(conn);

    if stats.is_empty() {
        None
    } else {
        Some(stats)
    }
}

pub fn format_language_stats_section(stats: Option<&[LanguageStat]>) -> Vec<String> {
    let stats = match stats {
        Some(s) if !s.is_empty() => s,
        _ => return vec![],
    };

    let mut table = OpsTable::new();
    table.set_header(vec!["Language", "Lines of Code", "Files"]);

    let total_loc: i64 = stats.iter().map(|s| s.loc).sum();

    for stat in stats {
        let pct = if total_loc > 0 {
            format!("{:.1}%", (stat.loc as f64 / total_loc as f64) * 100.0)
        } else {
            String::new()
        };
        let loc_str = format!("{} ({})", format_number(stat.loc), pct);
        table.add_row(vec![
            Cell::new(&stat.language),
            Cell::new(&loc_str),
            Cell::new(format_number(stat.file_count)),
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
    let config = std::sync::Arc::new(ops_core::config::Config::default());
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
            LanguageStat {
                language: "Rust".to_string(),
                loc: 8000,
                file_count: 40,
            },
            LanguageStat {
                language: "TOML".to_string(),
                loc: 200,
                file_count: 5,
            },
        ];
        let output = format_language_stats_section(Some(&stats)).join("\n");
        assert!(output.contains("Rust"));
        assert!(output.contains("TOML"));
    }

    #[test]
    fn format_language_stats_section_single_language_shows_100_percent() {
        let stats = vec![LanguageStat {
            language: "Rust".to_string(),
            loc: 5000,
            file_count: 25,
        }];
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
        let config = Arc::new(Config::default());
        let mut ctx = Context::new(config, std::path::PathBuf::from("/tmp"));
        ctx.db = Some(db);

        let registry = DataRegistry::new();
        assert!(query_language_stats(&mut ctx, &registry).is_none());
    }

    #[test]
    fn format_language_stats_section_percentages_add_up() {
        let stats = vec![
            LanguageStat {
                language: "Rust".to_string(),
                loc: 750,
                file_count: 10,
            },
            LanguageStat {
                language: "TOML".to_string(),
                loc: 250,
                file_count: 5,
            },
        ];
        let output = format_language_stats_section(Some(&stats)).join("\n");
        assert!(output.contains("75.0%"));
        assert!(output.contains("25.0%"));
    }
}
