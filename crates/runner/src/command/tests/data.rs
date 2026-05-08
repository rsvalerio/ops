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

/// ARCH-9 / TASK-0993: a provider that fans out to a peer via
/// `ctx.get_or_provide(other, registry)` should compute the peer exactly
/// once across two outer `query_data` calls on the same runner. Prior to
/// TASK-0993, `query_data` constructed a fresh `Context` per call and
/// promoted only the *outer* key into `self.data_cache`; the inner
/// provider's cached result lived only in the throw-away context, so a
/// later `runner.query_data("inner")` recomputed it.
#[test]
fn query_data_shares_inner_cache_across_outer_calls() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc as StdArc;

    struct CountingInner {
        calls: StdArc<AtomicUsize>,
    }
    impl DataProvider for CountingInner {
        fn name(&self) -> &'static str {
            "inner"
        }
        fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(serde_json::json!("inner-value"))
        }
    }

    struct ComposingOuter {
        sub_registry: StdArc<DataRegistry>,
    }
    impl DataProvider for ComposingOuter {
        fn name(&self) -> &'static str {
            "outer"
        }
        fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            let _ = ctx.get_or_provide("inner", &self.sub_registry)?;
            Ok(serde_json::json!("outer-value"))
        }
    }

    let calls = StdArc::new(AtomicUsize::new(0));

    let mut sub_registry = DataRegistry::new();
    sub_registry.register(
        "inner",
        Box::new(CountingInner {
            calls: StdArc::clone(&calls),
        }),
    );
    let sub_registry = StdArc::new(sub_registry);

    let mut main_registry = DataRegistry::new();
    main_registry.register(
        "inner",
        Box::new(CountingInner {
            calls: StdArc::clone(&calls),
        }),
    );
    main_registry.register(
        "outer",
        Box::new(ComposingOuter {
            sub_registry: StdArc::clone(&sub_registry),
        }),
    );

    let mut runner = test_runner(HashMap::new());
    runner.register_data_providers(main_registry);

    // Outer composes "inner" transitively via ctx.get_or_provide.
    runner.query_data("outer").expect("outer");
    // Now query "inner" directly. With a single shared Context cache,
    // this hits the cached inner-value populated during the outer call;
    // no extra invocation of either CountingInner instance occurs.
    runner.query_data("inner").expect("inner");
    runner.query_data("inner").expect("inner again");

    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "inner provider should be computed exactly once across outer calls"
    );
}

/// PERF-3 / TASK-0890: capture the cwd `Arc<PathBuf>` inside a provider
/// and assert its strong_count climbs above 1 — proof that `query_data`
/// hands out shared `Arc::clone`s instead of deep-cloning the inner path.
#[test]
fn query_data_shares_cwd_arc_with_provider() {
    use std::sync::Mutex;
    struct ArcCapturingProvider {
        captured: std::sync::Arc<Mutex<Option<std::sync::Arc<std::path::PathBuf>>>>,
    }
    impl DataProvider for ArcCapturingProvider {
        fn name(&self) -> &'static str {
            "arc_capture"
        }
        fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            let arc = std::sync::Arc::clone(&ctx.working_directory);
            *self.captured.lock().unwrap() = Some(arc);
            Ok(serde_json::Value::Null)
        }
    }

    let captured = std::sync::Arc::new(Mutex::new(None));
    let mut registry = DataRegistry::new();
    registry.register(
        "arc_capture",
        Box::new(ArcCapturingProvider {
            captured: std::sync::Arc::clone(&captured),
        }),
    );
    let mut runner = test_runner(HashMap::new());
    runner.register_data_providers(registry);

    let _ = runner.query_data("arc_capture").expect("query_data");
    let inner = captured.lock().unwrap();
    let cwd_arc = inner.as_ref().expect("provider captured cwd");
    assert!(
        std::sync::Arc::strong_count(cwd_arc) >= 2,
        "expected shared cwd Arc, got strong_count = {}",
        std::sync::Arc::strong_count(cwd_arc)
    );
}
