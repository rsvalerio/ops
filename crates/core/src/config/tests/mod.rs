//! Tests for configuration loading and merging.
//!
//! # Test Serialization (TQ-003, TQ-004)
//!
//! Some tests in this module are annotated with `#[serial]` because they modify
//! process-global state (environment variables). Without serialization, parallel
//! test execution would cause race conditions where one test's env var changes
//! affect another test.
//!
//! **Trade-off**: Serialization reduces parallelism for these tests, but it's
//! necessary for correctness. Future improvements could use process-isolated
//! tests (e.g., running each test in a subprocess) to restore parallelism.

use super::*;
use crate::test_utils::{exec_spec, TestConfigBuilder};
use indexmap::IndexMap;

mod merge_tests;
mod serde_tests;
mod template_tests;
mod validate_tests;

fn base_config() -> Config {
    TestConfigBuilder::new()
        .theme("classic")
        .columns(80)
        .show_error_detail(true)
        .exec("build", "cargo", &["build"])
        .build()
}

fn make_exec_spec(program: &str, args: &[&str]) -> CommandSpec {
    CommandSpec::Exec(exec_spec(program, args))
}
