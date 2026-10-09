//! Tests for the metadata extension, one sibling module per concern:
//! provider wiring, the subprocess output cap, and the payload cap.
//!
//! Shared fixtures live in `crate::test_support`, where `ingestor.rs`'s own
//! tests can reach them too.

use super::*;

ops_extension::test_datasource_extension!(
    MetadataExtension,
    name: "metadata",
    data_provider: "metadata"
);

mod output_cap;
mod payload_cap;
mod wiring;
