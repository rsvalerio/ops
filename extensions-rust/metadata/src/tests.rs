//! Tests for the metadata extension.
//!
//! ARCH-1 / TASK-1545: the previous single-file `tests.rs` had grown past
//! 1280 lines and fused four unrelated concerns (extension wiring, accessor
//! coverage, edge-case JSON probes, `DuckDB` payload-cap behaviour, and
//! duplicate-warning logging). Splitting into focused sibling modules keeps
//! each file under the ARCH-1 guideline and isolates the rebuild blast
//! radius — touching a payload-cap test no longer recompiles the accessor
//! coverage.
//!
//! DUP-4 / TASK-1540: the shared fixtures moved from `tests/fixtures.rs` up to
//! `crate::test_support`, which `ingestor.rs` also reaches — a sibling of
//! `tests/` could not have seen them here.

use super::*;

ops_extension::test_datasource_extension!(
    MetadataExtension,
    name: "metadata",
    data_provider: "metadata"
);

mod accessors;
mod duplicates;
mod edge_cases;
mod payload_cap;
mod wiring;
