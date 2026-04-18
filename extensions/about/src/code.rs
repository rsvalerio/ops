//! Stack-agnostic `about code` subpage: language statistics table.
//!
//! Reads language LOC and file counts from the `tokei_files` DuckDB view
//! (populated by the tokei data provider) and renders a table.

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
    let conn = db.lock().ok()?;

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

    let rows = stmt
        .query_map([], |row| {
            Ok(LanguageStat {
                language: row.get(0)?,
                loc: row.get(1)?,
                file_count: row.get(2)?,
            })
        })
        .ok()?;

    let stats: Vec<LanguageStat> = rows.filter_map(|r| r.ok()).collect();
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
    let cwd = std::env::current_dir()?;
    let config = std::sync::Arc::new(ops_core::config::Config::default());
    let mut ctx = Context::new(config, cwd);

    let stats = query_language_stats(&mut ctx, data_registry);
    let lines = format_language_stats_section(stats.as_deref());
    println!("{}", lines.join("\n"));
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
