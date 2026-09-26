//! `about loc` subpage: Rust production / test / example line counts.
//!
//! Reads the `rust_loc_summary` `SQLite` view (populated by the `rust-loc`
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
use ops_extension::{Context, DataRegistry};
use ops_sqlite::sql::RustLocStat;

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
        .enumerate()
        .find(|(_, (key, _))| *key == region)
        .map_or((REGION_ORDER.len(), region), |(i, (_, display))| {
            (i, *display)
        })
}

/// Everything the page renders, read from one `SQLite` snapshot.
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
/// `SQLite` support, or a failed query — because the page renders the same
/// message for all of them. Query failures are warn-logged so a real
/// error is still diagnosable.
pub fn query_rust_loc_stats(
    ctx: &mut Context,
    data_registry: &DataRegistry,
) -> Option<RustLocPage> {
    warm_providers(ctx, data_registry, &["sqlite", "rust-loc"], "loc");

    let db = ops_sqlite::get_db(ctx)?;
    let regions = match ops_sqlite::sql::query_rust_loc_summary(db) {
        Ok(regions) if regions.is_empty() => return None,
        Ok(regions) => regions,
        Err(e) => {
            tracing::warn!(error = ?e, "about/loc: query_rust_loc_summary failed");
            return None;
        }
    };

    // The file count is a second query against the same connection, so a
    // concurrent re-ingest between the two could pair regions with a file
    // count from a different snapshot. Accepted for the same reason as the
    // about card's five-query enrich (see `lib::enrich_from_db`): the page
    // re-renders on every invocation, so a stale frame self-corrects.
    let files = ops_sqlite::sql::query_rust_loc_file_count(db).unwrap_or_else(|e| {
        tracing::warn!(error = ?e, "about/loc: query_rust_loc_file_count failed");
        0
    });

    Some(RustLocPage { regions, files })
}

/// Format the region breakdown as displayable lines.
///
/// Returns `None` when there is nothing to show, signalling the caller to
/// emit a user-facing message instead of an empty table.
#[must_use = "render the returned section; a `None` means emit the fallback message"]
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
            OpsTable::text_cell(&capitalize(name)),
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
                // `i64 -> f64` has no `From` impl, so these two casts stay.
                // Line counts never approach f64's 2^53 exact-integer range
                // (~9e15 lines of Rust), so both conversions are exact, and
                // the result is rendered to one decimal place anyway.
                #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
                let pct = s.code as f64 * 100.0 / total_code as f64;
                format!("{pct:.1}% {name}")
            })
            .collect();
        use std::fmt::Write as _;
        let _ = write!(line, " \u{2014} {}", shares.join(", "));
    }

    line
}

/// # Errors
///
/// If the current directory cannot be determined, a required data provider
/// fails, or writing the rendered output fails.
pub fn run_about_loc(data_registry: &DataRegistry) -> anyhow::Result<()> {
    run_about_loc_with(data_registry, &mut std::io::stdout())
}

/// # Errors
///
/// If the current directory cannot be determined, a required data provider
/// fails, or writing the rendered output fails.
pub fn run_about_loc_with(
    data_registry: &DataRegistry,
    writer: &mut dyn Write,
) -> anyhow::Result<()> {
    let mut ctx = crate::providers::subpage_context("loc")?;

    let page = query_rust_loc_stats(&mut ctx, data_registry);
    match format_rust_loc_section(page.as_ref()) {
        Some(lines) => writeln!(writer, "{}", lines.join("\n"))?,
        // Named as a Rust-only page so a Go or Node user reads this as
        // "not applicable here" rather than "collection failed".
        None => writeln!(writer, "No Rust LOC data available.")?,
    }
    Ok(())
}

/// Version of the `ops about loc --json` document shape.
pub const LOC_JSON_SCHEMA_VERSION: u32 = 1;

/// One region's counts in the `ops about loc --json` document. `region` is
/// the display name (`production`, `test`, `example`, or an unknown raw
/// key), matching the table.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocRegionRecord {
    pub region: String,
    pub files: i64,
    pub code: i64,
    pub docs: i64,
    pub comments: i64,
    pub blanks: i64,
    pub lines: i64,
}

/// One crate's split in the `ops about loc --json` document.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocCrateRecord {
    pub name: String,
    /// Repo-relative manifest dir, as `ops about crates --json` reports it.
    pub manifest_dir: String,
    /// Distinct `.rs` files owned by the crate.
    pub files: i64,
    pub regions: Vec<LocRegionRecord>,
}

/// The `ops about loc --json` document: workspace totals plus the
/// per-crate split, both computed from one read of `rust_loc_files`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocDocument {
    pub schema_version: u32,
    pub kind: &'static str,
    /// Distinct `.rs` files counted across the workspace.
    pub files: i64,
    pub regions: Vec<LocRegionRecord>,
    pub crates: Vec<LocCrateRecord>,
}

/// Accumulates region stats and the distinct files behind them.
#[derive(Default)]
struct RegionTally {
    regions: std::collections::BTreeMap<String, RustLocStat>,
    files: std::collections::BTreeSet<String>,
}

impl RegionTally {
    fn add(&mut self, file: &str, stat: &RustLocStat) {
        self.files.insert(file.to_string());
        let entry = self
            .regions
            .entry(stat.region.clone())
            .or_insert_with(|| RustLocStat {
                region: stat.region.clone(),
                files: 0,
                code: 0,
                docs: 0,
                comments: 0,
                blanks: 0,
                lines: 0,
            });
        entry.files = entry.files.saturating_add(stat.files);
        entry.code = entry.code.saturating_add(stat.code);
        entry.docs = entry.docs.saturating_add(stat.docs);
        entry.comments = entry.comments.saturating_add(stat.comments);
        entry.blanks = entry.blanks.saturating_add(stat.blanks);
        entry.lines = entry.lines.saturating_add(stat.lines);
    }

    fn file_count(&self) -> i64 {
        i64::try_from(self.files.len()).unwrap_or(i64::MAX)
    }

    /// Region records in display order (production, test, example, then
    /// unknown regions by raw key).
    fn records(&self) -> Vec<LocRegionRecord> {
        let mut stats: Vec<&RustLocStat> = self.regions.values().collect();
        stats.sort_by(|a, b| {
            region_display(&a.region)
                .0
                .cmp(&region_display(&b.region).0)
                .then_with(|| a.region.cmp(&b.region))
        });
        stats
            .into_iter()
            .map(|s| LocRegionRecord {
                region: region_display(&s.region).1.to_string(),
                files: s.files,
                code: s.code,
                docs: s.docs,
                comments: s.comments,
                blanks: s.blanks,
                lines: s.lines,
            })
            .collect()
    }
}

/// Assign each `(file, stat)` row to the in-tree crate whose manifest dir
/// is the longest prefix of the file (a `"."` root crate owns whatever no
/// nested member claims) and total the workspace alongside.
#[must_use]
pub fn build_loc_document(
    rows: &[(String, RustLocStat)],
    crates: &[crate::units::UnitRecord],
) -> LocDocument {
    let members: Vec<&crate::units::UnitRecord> = crates.iter().filter(|c| c.in_tree).collect();
    let mut total = RegionTally::default();
    let mut per_crate: Vec<RegionTally> = members.iter().map(|_| RegionTally::default()).collect();
    for (file, stat) in rows {
        total.add(file, stat);
        let owner = members
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.manifest_dir == "."
                    || file
                        .strip_prefix(c.manifest_dir.as_str())
                        .is_some_and(|rest| rest.starts_with('/'))
            })
            .max_by_key(|(_, c)| {
                if c.manifest_dir == "." {
                    0
                } else {
                    c.manifest_dir.len()
                }
            })
            .map(|(i, _)| i);
        if let Some(tally) = owner.and_then(|i| per_crate.get_mut(i)) {
            tally.add(file, stat);
        }
    }
    let mut crate_records: Vec<LocCrateRecord> = members
        .iter()
        .zip(&per_crate)
        .map(|(c, tally)| LocCrateRecord {
            name: c.name.clone(),
            manifest_dir: c.manifest_dir.clone(),
            files: tally.file_count(),
            regions: tally.records(),
        })
        .collect();
    crate_records.sort_by(|a, b| a.manifest_dir.cmp(&b.manifest_dir));
    LocDocument {
        schema_version: LOC_JSON_SCHEMA_VERSION,
        kind: "about-loc",
        files: total.file_count(),
        regions: total.records(),
        crates: crate_records,
    }
}

/// `ops about loc --json`.
///
/// # Errors
///
/// If the current directory cannot be determined, a provider fails, or
/// writing fails.
pub fn run_about_loc_json(data_registry: &DataRegistry) -> anyhow::Result<()> {
    run_about_loc_json_with(data_registry, &mut std::io::stdout())
}

/// [`run_about_loc_json`] against an explicit writer. A workspace with no
/// Rust LOC data yields the document with empty lists, not an error, so a
/// consumer can parse every answer the same way.
///
/// # Errors
///
/// If the current directory cannot be determined, a provider fails, or
/// writing fails.
pub fn run_about_loc_json_with(
    data_registry: &DataRegistry,
    writer: &mut dyn Write,
) -> anyhow::Result<()> {
    let mut ctx = crate::providers::subpage_context("loc")?;
    warm_providers(&mut ctx, data_registry, &["sqlite", "rust-loc"], "loc");
    let rows = ops_sqlite::get_db(&ctx).map_or_else(Vec::new, |db| {
        ops_sqlite::sql::query_rust_loc_files(db).unwrap_or_else(|e| {
            tracing::warn!(error = ?e, "about/loc: query_rust_loc_files failed");
            Vec::new()
        })
    });
    let units: Vec<ops_core::project_identity::ProjectUnit> = crate::providers::load_or_default(
        &mut ctx,
        data_registry,
        crate::units::PROJECT_UNITS_PROVIDER,
    )?;
    let records: Vec<crate::units::UnitRecord> = units
        .iter()
        .map(crate::units::UnitRecord::from_unit)
        .collect();
    crate::write_json_document(writer, &build_loc_document(&rows, &records))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &str, dir: &str) -> crate::units::UnitRecord {
        crate::units::UnitRecord {
            name: name.to_string(),
            version: None,
            manifest_dir: dir.to_string(),
            in_tree: true,
        }
    }

    fn row(file: &str, region: &str, code: i64) -> (String, RustLocStat) {
        (
            file.to_string(),
            RustLocStat {
                region: region.to_string(),
                files: 1,
                code,
                docs: 1,
                comments: 1,
                blanks: 1,
                lines: code.saturating_add(3),
            },
        )
    }

    /// TASK-2282: pins the `ops about loc --json` shape and the
    /// longest-prefix crate assignment (a nested member wins over the root
    /// crate; the root owns the rest).
    #[test]
    fn build_loc_document_pins_shape_and_assignment() {
        let rows = vec![
            row("src/main.rs", "main", 10),
            row("crates/a/src/lib.rs", "main", 20),
            row("crates/a/src/lib.rs", "test", 5),
            row("crates/ab/src/lib.rs", "main", 7),
        ];
        let crates = vec![
            record("root", "."),
            record("a", "crates/a"),
            record("ab", "crates/ab"),
        ];
        let doc = build_loc_document(&rows, &crates);
        let value = serde_json::to_value(&doc).expect("serialize");
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["kind"], "about-loc");
        assert_eq!(value["files"], 3);
        assert_eq!(
            value["regions"],
            serde_json::json!([
                {"region": "production", "files": 3, "code": 37, "docs": 3, "comments": 3, "blanks": 3, "lines": 46},
                {"region": "test", "files": 1, "code": 5, "docs": 1, "comments": 1, "blanks": 1, "lines": 8},
            ])
        );
        let crates = value["crates"].as_array().expect("crates");
        let dirs: Vec<&str> = crates
            .iter()
            .map(|c| c["manifestDir"].as_str().unwrap())
            .collect();
        assert_eq!(dirs, [".", "crates/a", "crates/ab"]);
        assert_eq!(crates[0]["files"], 1);
        assert_eq!(crates[0]["regions"][0]["code"], 10);
        assert_eq!(crates[1]["files"], 1);
        assert_eq!(crates[1]["regions"].as_array().unwrap().len(), 2);
        assert_eq!(crates[2]["regions"][0]["code"], 7);
        let text = serde_json::to_string(&doc).expect("serialize");
        assert!(
            text.starts_with(
                "{\"schemaVersion\":1,\"kind\":\"about-loc\",\"files\":3,\"regions\":"
            ),
            "field order is part of the contract: {text}"
        );
    }

    #[test]
    fn run_about_loc_json_with_empty_registry_emits_empty_lists() {
        let registry = DataRegistry::new();
        let mut out: Vec<u8> = Vec::new();
        run_about_loc_json_with(&registry, &mut out).expect("runner must succeed");
        let value: serde_json::Value = serde_json::from_slice(&out).expect("valid json");
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["regions"], serde_json::json!([]));
        assert_eq!(value["crates"], serde_json::json!([]));
    }

    /// TEST-5 / TASK-1739: with no `SQLite` handle on the context,
    /// `query_rust_loc_stats` yields `None` and the runner takes its
    /// Rust-only "not applicable here" branch. That string had no assertion
    /// behind it.
    #[test]
    fn run_about_loc_with_reports_no_data_for_an_empty_registry() {
        let registry = DataRegistry::new();
        let mut out: Vec<u8> = Vec::new();
        run_about_loc_with(&registry, &mut out).expect("runner must succeed");
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "No Rust LOC data available.\n"
        );
    }

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
    /// a poisoned `Sqlite` mutex degrades to `None` (warn-logged), not a panic.
    #[test]
    fn query_rust_loc_stats_returns_none_when_db_lock_poisoned() {
        use ops_core::config::Config;
        use std::sync::Arc;

        let db = Arc::new(ops_sqlite::Sqlite::open_in_memory().expect("db"));
        ops_sqlite::init_schema(&db).expect("init_schema");

        let poisoner = Arc::clone(&db);
        let _ = std::thread::spawn(move || {
            let _guard = poisoner.lock().expect("lock");
            panic!("intentional poison");
        })
        .join();

        let config = Arc::new(Config::empty());
        let mut ctx = Context::new(config, std::path::PathBuf::from("/tmp"));
        ctx.attach_db(db);

        let registry = DataRegistry::new();
        assert!(query_rust_loc_stats(&mut ctx, &registry).is_none());
    }
}
