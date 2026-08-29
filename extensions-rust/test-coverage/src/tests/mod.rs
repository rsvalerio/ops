//! Tests for the coverage extension.
//!
//! ARCH-1 / TASK-1944: the production code was split by concern under
//! TASK-1559, but the test module was not and had grown to 940 lines behind
//! banner comments that no longer kept related tests together. It now
//! mirrors the production module layout, one file per concern:
//!
//! - [`wiring`]: extension registration + the public `load_coverage` entry.
//! - [`parse`] / [`parse_edge`]: `flatten_coverage_json` behaviour and its
//!   malformed-input edge cases (split only for size; both cover `parse`).
//! - [`collect`]: `collect_coverage`'s soft-fail / hard-fail policy.
//! - [`subprocess`]: argv guards, exit checking, stderr diagnostics.
//! - [`provider`]: `CoverageProvider` schema + `DuckDB` readback.
//! - [`ingest`]: `CoverageIngestor` end-to-end loads.
//! - [`views`]: `coverage_summary` DDL and query behaviour.
//!
//! Test placement rule (TASK-1944): tests live here unless they need
//! module-private items. `ingestor.rs` keeps an inline `#[cfg(test)]` module
//! because its tests bind the private `PIPELINE` constant; nothing else in
//! the crate does, so no other file carries inline tests.

mod collect;
mod ingest;
mod parse;
mod parse_edge;
mod provider;
mod subprocess;
mod views;
mod wiring;

use crate::ingestor::CoverageIngestor;
use crate::parse::flatten_coverage_json;
use ops_duckdb::{DataIngestor, DuckDb, IngestDir};

/// Two-file coverage fixture shared by the flatten, ingest, provider, and
/// view tests. `src/lib.rs` deliberately carries different counts from
/// `src/main.rs` so the `coverage_summary` SUM aggregates are pinned to a
/// value neither file could produce alone.
pub fn sample_coverage_json() -> serde_json::Value {
    serde_json::json!({
        "data": [{
            "files": [
                {
                    "filename": "src/main.rs",
                    "summary": {
                        "lines": { "count": 100, "covered": 80, "percent": 80.0 },
                        "functions": { "count": 10, "covered": 8, "percent": 80.0 },
                        "regions": { "count": 20, "covered": 16, "notcovered": 4, "percent": 80.0 },
                        "branches": { "count": 5, "covered": 3, "notcovered": 2, "percent": 60.0 }
                    }
                },
                {
                    "filename": "src/lib.rs",
                    "summary": {
                        "lines": { "count": 200, "covered": 190, "percent": 95.0 },
                        "functions": { "count": 20, "covered": 19, "percent": 95.0 },
                        "regions": { "count": 40, "covered": 38, "notcovered": 2, "percent": 95.0 },
                        "branches": { "count": 10, "covered": 9, "notcovered": 1, "percent": 90.0 }
                    }
                }
            ]
        }]
    })
}

/// SEC-25 / TASK-2054: the fixture is staged through the verified anchor, the
/// same way `CoverageIngestor::collect` stages it in production.
pub fn write_coverage_fixture(dir: &IngestDir) {
    let raw = sample_coverage_json();
    let flat = flatten_coverage_json(&raw).expect("flatten");
    let json_bytes = serde_json::to_vec_pretty(&flat).expect("serialize");
    dir.write_atomic("coverage_files.json", &json_bytes)
        .expect("write");
    dir.write_atomic("coverage_workspace.txt", b"/test/workspace")
        .expect("write workspace");
}

/// Open a verified ingest anchor inside `tmp`, mirroring what
/// `provide_via_ingestor` builds before it calls an ingestor.
pub fn ingest_anchor(tmp: &tempfile::TempDir) -> IngestDir {
    IngestDir::open(&tmp.path().join("data.duckdb.ingest")).expect("open ingest dir")
}

/// DUP-3 / TASK-1562: collapses the 5-line `tempdir + open_in_memory +
/// write_coverage_fixture + ingest` boilerplate that previously sat in
/// five `DuckDB` integration tests. Returns the tempdir (kept alive so the
/// sidecar paths remain valid for the lifetime of the test) and the
/// loaded `DuckDb` handle.
pub fn setup_loaded_db() -> (tempfile::TempDir, IngestDir, DuckDb) {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let dir = ingest_anchor(&data_dir);
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    write_coverage_fixture(&dir);
    let ingestor = CoverageIngestor;
    let _ = ingestor.load(&dir, &db).expect("load should succeed");
    // The tempdir is returned alongside the anchor so the staging directory
    // outlives the descriptor for the whole test.
    (data_dir, dir, db)
}
