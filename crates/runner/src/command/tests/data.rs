//! TQ-GAP-005: Tests for CommandRunner::query_data().

use super::*;
use ops_extension::{Context, DataProvider, DataProviderError, DataRegistry};

struct FixedProvider {
    value: serde_json::Value,
}

impl DataProvider for FixedProvider {
    fn name(&self) -> &'static str {
        "fixed"
    }
    fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        Ok(self.value.clone())
    }
}

struct FailingProvider;

impl DataProvider for FailingProvider {
    fn name(&self) -> &'static str {
        "failing"
    }
    fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        Err(DataProviderError::computation_failed("provider error"))
    }
}

#[test]
fn query_data_returns_provider_value() {
    let mut registry = DataRegistry::new();
    registry.register(
        "fixed",
        Box::new(FixedProvider {
            value: serde_json::json!({"hello": "world"}),
        }),
    );
    let mut runner = test_runner(HashMap::new());
    runner.register_data_providers(registry);

    let result = runner.query_data("fixed");
    assert!(result.is_ok());
    assert_eq!(*result.unwrap(), serde_json::json!({"hello": "world"}));
}

#[test]
fn query_data_caches_results() {
    let mut registry = DataRegistry::new();
    registry.register(
        "fixed",
        Box::new(FixedProvider {
            value: serde_json::json!(42),
        }),
    );
    let mut runner = test_runner(HashMap::new());
    runner.register_data_providers(registry);

    let v1 = runner.query_data("fixed").expect("first call");
    let v2 = runner.query_data("fixed").expect("second call (cached)");
    assert_eq!(*v1, *v2);
}

#[test]
fn query_data_unknown_provider_errors() {
    let mut runner = test_runner(HashMap::new());
    let result = runner.query_data("nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn query_data_failing_provider_errors() {
    let mut registry = DataRegistry::new();
    registry.register("failing", Box::new(FailingProvider));
    let mut runner = test_runner(HashMap::new());
    runner.register_data_providers(registry);

    let result = runner.query_data("failing");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("provider error"));
}
