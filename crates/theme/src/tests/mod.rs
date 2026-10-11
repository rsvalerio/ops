//! Tests that need private access to the crate.
//!
//! Everything else — every test that only exercises the public API — lives
//! in `tests/public_api/` as integration tests. These two modules stay
//! inline because they call `render_error_block_gated`, which lives in the
//! private `render` module and is unreachable from an integration test.

use super::*;
use ops_core::output::ErrorDetail;

mod error_block_color;
mod error_block_sanitise;
