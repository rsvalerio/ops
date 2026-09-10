//! Coverage extension: LLVM code coverage via `cargo llvm-cov`.
//! Collects per-file coverage data and loads into `DuckDB`.
//!
//! ARCH-1 / TASK-1559: the previous monolithic `lib.rs` (412 lines) mixed
//! six concerns. The crate is now split into:
//!
//! - [`subprocess`]: cargo argv + run/check helpers + exit formatter.
//! - [`parse`]: llvm-cov JSON → `CoverageRow` flattening + soft-fail policy.
//! - [`provider`]: `CoverageProvider` impl + `DuckDB` readback.
//! - [`ingestor`]: `CoverageIngestor` (sidecar writer + `DuckDB` loader).
//! - [`views`]: `DuckDB` view DDL.
//!
//! `lib.rs` retains only wiring + `load_coverage` (the crate's public ingest
//! entry point).

// READ-10 / TASK-1946: only `unwrap_used` is load-bearing here. The three
// cast lints that used to sit alongside it were a copied template — the
// crate contains no `as` cast in any configuration, so they suppressed
// nothing and no reader could tell which entries mattered.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        reason = "test code uses unwrap and expect as its failure mechanism (ERR-5 scanning guidance)"
    )
)]

// API-14 / TASK-2198: the five modules below are private, so the `pub`
// items inside them are crate-internal — that spelling (rather than
// `pub(crate)`) is the convention `clippy::redundant_pub_crate` enforces
// workspace-wide; see the note in `ops-cargo-toml`'s lib.rs. The crate's
// exported surface is the const quartet, `CoverageExtension` and
// `load_coverage` below.
mod ingestor;
mod parse;
mod provider;
mod subprocess;
#[cfg(test)]
mod tests;
mod views;

// API-9 / TASK-1601: CoverageIngestor has no external callers; kept
// crate-private (ingestor + provider reference it via crate-internal paths).
// API-9 / TASK-1602: flatten_coverage_json / collect_coverage have no external
// callers either; they stay in the private parse.rs, so nothing they declare
// is reachable outside the crate.

use crate::ingestor::CoverageIngestor;
use ops_duckdb::{init_schema, DataIngestor, DuckDb, IngestDir, LoadResult};
use ops_extension::ExtensionType;

/// Extension identifier used to register this crate in the engine's
/// extension registry.
pub const NAME: &str = "coverage";
/// One-line description shown by `ops about` for this extension.
pub const DESCRIPTION: &str = "LLVM code coverage provider (per-file line, function, region, \
     branch coverage); requires cargo-llvm-cov (cargo install cargo-llvm-cov + \
     rustup component add llvm-tools-preview)";
/// CLI-facing short name (`cov`) used in commands and user-facing output.
pub const SHORTNAME: &str = "cov";
/// Registry key of the `coverage` data provider this crate registers — the
/// key the about coverage subpage looks the per-unit coverage table up by.
pub const DATA_PROVIDER_NAME: &str = "coverage";

/// API-9 / TASK-0922: construct via the registered extension factory only.
#[non_exhaustive]
pub struct CoverageExtension;

ops_extension::impl_extension! {
    CoverageExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        let _ = registry.register(DATA_PROVIDER_NAME, Box::new(provider::CoverageProvider));
    },
    factory: COVERAGE_FACTORY = |_, _| {
        Some((NAME, Box::new(CoverageExtension)))
    },
}

/// Ingest coverage sidecar data into `DuckDB` and return the structured load
/// report.
///
/// READ-5 (TASK-0808): the previous signature returned `()` and silently
/// dropped the [`LoadResult`], leaving callers unable to distinguish a
/// zero-row load from a healthy one. The signature now surfaces the report;
/// a zero-record load is also logged at `warn` so even fire-and-forget
/// callers see the health signal.
///
/// API-5 / TASK-1561: `#[must_use]` carries that contract into the type
/// system so a future caller writing `let _ = load_coverage(...)` lights
/// up a lint.
///
/// # Errors
///
/// If the schema cannot be initialised, or the coverage sidecar staged in
/// `dir` cannot be read or loaded into the database.
///
/// SEC-25 / TASK-2054: takes the verified [`IngestDir`] anchor the ingestor
/// trait now takes, so this public entry point cannot re-introduce a by-name
/// resolution of the staging directory that `provide_via_ingestor` verified.
#[must_use = "load report carries the record_count health signal (TASK-0808)"]
pub fn load_coverage(dir: &IngestDir, db: &DuckDb) -> Result<LoadResult, anyhow::Error> {
    init_schema(db)?;
    let ingestor = CoverageIngestor;
    let load_result = ingestor.load(dir, db)?;
    if load_result.record_count == 0 {
        tracing::warn!(
            source = load_result.source_name,
            data_dir = %dir.path().display(),
            "coverage load completed with zero records"
        );
    }
    Ok(load_result)
}
