//! `about loc` subpage: Rust production / test / example line counts.
//!
//! Reads the `rust_loc_summary` DuckDB view (populated by the `rust-loc`
//! data provider) and renders a per-region table plus a totals line.
//!
//! Complements [`crate::code`], which reports cross-language LOC from
//! `tokei_files`: tokei has no model for test versus production code, so
//! the two pages answer different questions over the same sources and the
//! numbers are not expected to match — this page counts only `.rs` files
//! and splits `#[cfg(test)]` blocks out of the file that contains them.

use std::io::Write;

use ops_core::table::{Cell, OpsTable};
use ops_core::text::{capitalize, format_number};
use ops_duckdb::sql::RustLocStat;
use ops_extension::{Context, DataRegistry};

use crate::providers::warm_providers;

/// Region keys the `rust-loc` provider emits, in display order.
///
/// The provider's own key (`main`) is spelled "production" here because
/// that is what the split is *for*; the raw key stays the wire format.
/// A region outside this list still renders — it sorts last, under its
/// own capitalised name — so a region added upstream shows up on the page
/// instead of silently vanishing from the totals.
const REGION_ORDER: &[(&str, &str)] = &[
    ("main", "production"),
    ("test", "test"),
    ("example", "example"),
];

/// The lower-case display name for a region key, and its rank in
/// [`REGION_ORDER`]. Unknown regions rank last and keep their raw name.
fn region_display(region: &str) -> (usize, &str) {
    REGION_ORDER
        .iter()
        .position(|(key, _)| *key == region)
        .map_or((REGION_ORDER.len(), region), |i| (i, REGION_ORDER[i].1))
}

/// Everything the page renders, read from one DuckDB snapshot.
///
/// `files` is a separate figure rather than a sum of the rows' own
/// `files`: a module with a `#[cfg(test)]` block appears in both the
/// `main` and `test` rows, so adding those columns counts it twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustLocPage {
    pub regions: Vec<RustLocStat>,
    pub files: i64,
}

/// Fetch the per-region Rust LOC breakdown, or `None` when it is
/// unavailable.
///
/// `None` covers every "this workspace has no Rust LOC data" case —
/// non-Rust stack (the provider is not registered), a build without
/// DuckDB support, or a failed query — because the page renders the same
/// message for all of them. Query failures are warn-logged so a real
/// error is still diagnosable.
pub fn query_rust_loc_stats(
    ctx: &mut Context,
    data_registry: &DataRegistry,
) -> Option<RustLocPage> {
    warm_providers(ctx, data_registry, &["duckdb", "rust-loc"], "loc");

    let db = ops_duckdb::get_db(ctx)?;
    let regions = match ops_duckdb::sql::query_rust_loc_summary(db) {
        Ok(regions) if regions.is_empty() => return None,
        Ok(regions) => regions,
        Err(e) => {
            tracing::warn!("about/loc: query_rust_loc_summary failed: {e:#}");
            return None;
        }
    };

    // The file count is a second query against the same connection, so a
    // concurrent re-ingest between the two could pair regions with a file
    // count from a different snapshot. Accepted for the same reason as the
    // about card's five-query enrich (see `lib::enrich_from_db`): the page
    // re-renders on every invocation, so a stale frame self-corrects.
    let files = ops_duckdb::sql::query_rust_loc_file_count(db).unwrap_or_else(|e| {
        tracing::warn!("about/loc: query_rust_loc_file_count failed: {e:#}");
        0
    });

    Some(RustLocPage { regions, files })
}

/// Format the region breakdown as displayable lines.
///
/// Returns `None` when there is nothing to show, signalling the caller to
/// emit a user-facing message instead of an empty table.
pub fn format_rust_loc_section(page: Option<&RustLocPage>) -> Option<Vec<String>> {
    let page = match page {
        Some(p) if !p.regions.is_empty() => p,
        _ => return None,
    };

    let mut sorted: Vec<&RustLocStat> = page.regions.iter().collect();
    sorted.sort_by(|a, b| {
        let (a_rank, _) = region_display(&a.region);
        let (b_rank, _) = region_display(&b.region);
        // Rank first, then the raw key, so two unknown regions still order
        // deterministically instead of inheriting the view's row order.
        a_rank.cmp(&b_rank).then_with(|| a.region.cmp(&b.region))
    });

    let mut lines = vec![String::new()];
    lines.extend(
        format_region_table(&sorted)
            .lines()
            .map(|l| format!("    {l}")),
    );
    lines.push(String::new());
    lines.push(format_totals_line(&sorted, page.files));

    Some(lines)
}

fn format_region_table(stats: &[&RustLocStat]) -> String {
    let mut table = OpsTable::new();
    table.set_header(vec![
        "Region", "Files", "Code", "Docs", "Comments", "Blanks", "Lines",
    ]);

    for stat in stats {
        let (_, name) = region_display(&stat.region);
        table.add_row(vec![
            Cell::new(capitalize(name)),
            Cell::new(format_number(stat.files)),
            Cell::new(format_number(stat.code)),
            Cell::new(format_number(stat.docs)),
            Cell::new(format_number(stat.comments)),
            Cell::new(format_number(stat.blanks)),
            Cell::new(format_number(stat.lines)),
        ]);
    }

    table.to_string()
}

/// Render the totals line: absolute code lines and files, then each
/// region's share of code.
///
/// The share clause is omitted when the total is zero (a workspace whose
/// `.rs` files hold only comments and blanks), since every percentage
/// would be a division by zero dressed up as `0.0%`.
fn format_totals_line(stats: &[&RustLocStat], files: i64) -> String {
    let total_code: i64 = stats.iter().map(|s| s.code).sum();

    let mut line = format!(
        "    total: {} code lines in {} files",
        format_number(total_code),
        format_number(files)
    );

    if total_code > 0 {
        let shares: Vec<String> = stats
            .iter()
            .map(|s| {
                let (_, name) = region_display(&s.region);
                // Line counts never approach f64's 2^53 exact-integer range,
                // and the result is rendered to one decimal place anyway.
                #[allow(clippy::cast_precision_loss)]
                let pct = s.code as f64 * 100.0 / total_code as f64;
                format!("{pct:.1}% {name}")
            })
            .collect();
        use std::fmt::Write as _;
        let _ = write!(line, " \u{2014} {}", shares.join(", "));
    }

    line
}

pub fn run_about_loc(data_registry: &DataRegistry) -> anyhow::Result<()> {
    run_about_loc_with(data_registry, &mut std::io::stdout())
}

pub fn run_about_loc_with(
    data_registry: &DataRegistry,
    writer: &mut dyn Write,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = std::sync::Arc::new(ops_core::config::Config::empty());
    let mut ctx = Context::new(config, cwd);

    let page = query_rust_loc_stats(&mut ctx, data_registry);
    match format_rust_loc_section(page.as_ref()) {
        Some(lines) => writeln!(writer, "{}", lines.join("\n"))?,
        // Named as a Rust-only page so a Go or Node user reads this as
        // "not applicable here" rather than "collection failed".
        None => writeln!(writer, "No Rust LOC data available.")?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A region row carrying only the fields the ordering and totals
    /// assertions read; the per-line-kind columns get their own fixture
    /// in `format_rust_loc_section_renders_all_columns`.
    fn stat(region: &str, files: i64, code: i64) -> RustLocStat {
        RustLocStat {
            region: region.to_string(),
            files,
            code,
            docs: 0,
            comments: 0,
            blanks: 0,
            lines: code,
        }
    }

    fn page(regions: Vec<RustLocStat>, files: i64) -> RustLocPage {
        RustLocPage { regions, files }
    }

    #[test]
    fn format_rust_loc_section_none_without_data() {
        assert!(format_rust_loc_section(None).is_none());
        assert!(format_rust_loc_section(Some(&page(vec![], 0))).is_none());
    }

    #[test]
    fn region_display_maps_main_to_production() {
        assert_eq!(region_display("main"), (0, "production"));
        assert_eq!(region_display("test"), (1, "test"));
        assert_eq!(region_display("example"), (2, "example"));
    }

    /// An unrecognised region keeps its raw name and ranks after the
    /// known ones, so upstream additions surface instead of disappearing.
    #[test]
    fn region_display_passes_through_unknown_region() {
        assert_eq!(region_display("bench"), (REGION_ORDER.len(), "bench"));
    }

    /// The view orders by code descending; the page must not inherit that
    /// — production/test/example is the order the reader expects.
    #[test]
    fn format_rust_loc_section_orders_regions_canonically() {
        let stats = page(
            vec![
                stat("test", 5, 900),
                stat("example", 1, 30),
                stat("main", 10, 500),
            ],
            12,
        );
        let lines = format_rust_loc_section(Some(&stats)).expect("data returns Some");
        let table = lines.join("\n");
        let production = table.find("Production").expect("production row");
        let test = table.find("Test").expect("test row");
        let example = table.find("Example").expect("example row");
        assert!(
            production < test && test < example,
            "regions must render production, test, example: {table}"
        );
    }

    /// The file count is the distinct-file figure the caller supplies, not
    /// the sum of the per-region `files` columns: a file holding both
    /// production code and a `#[cfg(test)]` block appears in two rows, so
    /// summing them (10 + 5 = 15 here) would over-report a 10-file crate.
    #[test]
    fn format_rust_loc_section_totals_line_uses_distinct_file_count() {
        let stats = page(vec![stat("main", 10, 750), stat("test", 5, 250)], 10);
        let lines = format_rust_loc_section(Some(&stats)).expect("data returns Some");
        let total = lines.last().expect("totals line");
        assert_eq!(
            total,
            "    total: 1,000 code lines in 10 files \u{2014} 75.0% production, 25.0% test"
        );
        assert!(
            !total.contains("15 files"),
            "must not sum overlapping per-region file counts: {total}"
        );
    }

    /// Structural contract, mirroring `coverage::format_coverage_section`:
    /// leading blank, indented table block, blank, totals line.
    #[test]
    fn format_rust_loc_section_structure() {
        let stats = page(vec![stat("main", 10, 500)], 10);
        let lines = format_rust_loc_section(Some(&stats)).expect("data returns Some");
        assert!(lines.len() >= 4, "got lines: {lines:?}");
        assert!(lines.first().expect("first").is_empty(), "leading blank");
        assert!(
            lines[lines.len() - 2].is_empty(),
            "blank before totals: {lines:?}"
        );
        let table_block = &lines[1..lines.len() - 2];
        assert!(
            table_block.iter().all(|l| l.starts_with("    ")),
            "table lines are indented: {table_block:?}"
        );
    }

    /// A `.rs` tree of pure comments has zero code lines; the shares
    /// clause must be dropped rather than printing `NaN%` or a row of
    /// meaningless zeroes.
    #[test]
    fn format_rust_loc_section_omits_shares_when_no_code() {
        let stats = page(
            vec![RustLocStat {
                region: "main".to_string(),
                files: 2,
                code: 0,
                docs: 40,
                comments: 10,
                blanks: 5,
                lines: 55,
            }],
            2,
        );
        let lines = format_rust_loc_section(Some(&stats)).expect("data returns Some");
        let total = lines.last().expect("totals line");
        assert_eq!(total, "    total: 0 code lines in 2 files");
        assert!(!total.contains('%'), "no share clause: {total}");
        assert!(!total.contains("NaN"), "no NaN: {total}");
    }

    /// Docs, comments and blanks reach the table — the doc-comment split
    /// is the reason this page exists next to `about code`.
    #[test]
    fn format_rust_loc_section_renders_all_columns() {
        let stats = page(
            vec![RustLocStat {
                region: "main".to_string(),
                files: 3,
                code: 500,
                docs: 120,
                comments: 45,
                blanks: 80,
                lines: 745,
            }],
            3,
        );
        let table = format_rust_loc_section(Some(&stats))
            .expect("data returns Some")
            .join("\n");
        for expected in ["Docs", "Comments", "Blanks", "120", "45", "80", "745"] {
            assert!(table.contains(expected), "missing {expected}: {table}");
        }
    }

    /// Mirrors `code::query_language_stats_returns_none_when_db_lock_poisoned`:
    /// a poisoned DuckDb mutex degrades to `None` (warn-logged), not a panic.
    #[test]
    fn query_rust_loc_stats_returns_none_when_db_lock_poisoned() {
        use ops_core::config::Config;
        use std::sync::Arc;

        let db = Arc::new(ops_duckdb::DuckDb::open_in_memory().expect("db"));
        ops_duckdb::init_schema(&db).expect("init_schema");

        let poisoner = Arc::clone(&db);
        let _ = std::thread::spawn(move || {
            let _guard = poisoner.lock().expect("lock");
            panic!("intentional poison");
        })
        .join();

        let config = Arc::new(Config::empty());
        let mut ctx = Context::new(config, std::path::PathBuf::from("/tmp"));
        ctx.db = Some(db);

        let registry = DataRegistry::new();
        assert!(query_rust_loc_stats(&mut ctx, &registry).is_none());
    }
}
