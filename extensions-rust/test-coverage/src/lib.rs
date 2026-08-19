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

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

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
// callers either; demoted to pub(crate) in parse.rs.

use crate::ingestor::CoverageIngestor;
use ops_duckdb::{init_schema, DataIngestor, DuckDb, LoadResult};
use ops_extension::ExtensionType;
use std::path::Path;

pub const NAME: &str = "coverage";
pub const DESCRIPTION: &str = "LLVM code coverage provider (per-file line, function, region, \
     branch coverage); requires cargo-llvm-cov (cargo install cargo-llvm-cov + \
     rustup component add llvm-tools-preview)";
pub const SHORTNAME: &str = "cov";
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
        registry.register(DATA_PROVIDER_NAME, Box::new(provider::CoverageProvider));
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
/// If the schema cannot be initialised, or the coverage sidecar in
/// `data_dir` cannot be read or loaded into the database.
#[must_use = "load report carries the record_count health signal (TASK-0808)"]
pub fn load_coverage(data_dir: &Path, db: &DuckDb) -> Result<LoadResult, anyhow::Error> {
    init_schema(db)?;
    let ingestor = CoverageIngestor;
    let load_result = ingestor.load(data_dir, db)?;
    if load_result.record_count == 0 {
        tracing::warn!(
            source = load_result.source_name,
            data_dir = %data_dir.display(),
            "coverage load completed with zero records"
        );
    }
    Ok(load_result)
}
