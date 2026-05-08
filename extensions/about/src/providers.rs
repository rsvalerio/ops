//! Shared `about` subpage scaffolding: warm-up + load-with-default helpers.
//!
//! DUP-1 (TASK-0464): every subpage repeats the same `for provider in [...]
//! { match get_or_provide ... }` warm-up loop and the same triadic
//! `match get_or_provide(<provider>, registry)` deserialise-or-default
//! sequence. Centralising both here keeps the four subpages aligned and
//! makes drift between their warm-up lists visible at the call site.

use serde::de::DeserializeOwned;

use ops_extension::{Context, DataProviderError, DataRegistry};

/// Warm a sequence of provider names, swallowing only `NotFound` (which is
/// expected when a provider is not registered for the active stack). Real
/// provider failures are surfaced at `tracing::warn!` so a misbehaving
/// provider doesn't silently zero the rendered subpage.
///
/// `subpage` labels the warning so a reader can tell which subpage triggered
/// the warm-up failure.
pub fn warm_providers(
    ctx: &mut Context,
    registry: &DataRegistry,
    providers: &[&str],
    subpage: &str,
) {
    for provider in providers {
        match ctx.get_or_provide(provider, registry) {
            Ok(_) | Err(DataProviderError::NotFound(_)) => {}
            Err(e) => tracing::warn!("about/{subpage}: warm-up {provider} failed: {e:#}"),
        }
    }
}

/// Fetch a typed payload from the provider registry, returning a fresh
/// `Default` if the provider is not registered (`NotFound`). Other errors
/// are propagated so the subpage doesn't render zeros over a real failure.
pub fn load_or_default<T>(
    ctx: &mut Context,
    registry: &DataRegistry,
    provider: &str,
) -> anyhow::Result<T>
where
    T: DeserializeOwned + Default,
{
    match ctx.get_or_provide(provider, registry) {
        // PERF-3 (TASK-1117): borrow the Arc payload via `Deserialize::deserialize`
        // on `&Value` instead of deep-cloning the entire JSON tree just to feed
        // `from_value`, which takes `Value` by value.
        Ok(value) => Ok(T::deserialize(value.as_ref())?),
        Err(DataProviderError::NotFound(_)) => Ok(T::default()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_extension::DataProvider;
    use std::sync::Arc;

    struct FailingProvider(&'static str);
    impl DataProvider for FailingProvider {
        fn name(&self) -> &'static str {
            self.0
        }
        fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            Err(DataProviderError::computation_failed("boom"))
        }
    }

    fn test_ctx() -> Context {
        let config = Arc::new(ops_core::config::Config::empty());
        Context::new(config, std::path::PathBuf::from("/tmp"))
    }

    /// ERR-1 (TASK-0516): a non-NotFound provider error during warm-up
    /// must not propagate (warm-up is best-effort) and must not panic. The
    /// warn fires through tracing; pinning the value-level contract here
    /// avoids the tracing-subscriber dev-dep cost (matches the pattern in
    /// `code::tests::query_language_stats_returns_none_when_db_lock_poisoned`).
    #[test]
    fn warm_providers_swallows_real_failures_without_panic() {
        let mut registry = DataRegistry::new();
        registry.register("flaky", Box::new(FailingProvider("flaky")));

        let mut ctx = test_ctx();
        warm_providers(&mut ctx, &registry, &["flaky", "absent"], "test");
        // Reaching this line means warm-up returned cleanly for both a
        // failing-provider error and an unregistered (NotFound) provider.
    }

    /// ERR-1 (TASK-0516): load_or_default surfaces non-NotFound errors so a
    /// failing provider doesn't render zeros over a real failure.
    #[test]
    fn load_or_default_propagates_real_failures() {
        let mut registry = DataRegistry::new();
        registry.register("flaky", Box::new(FailingProvider("flaky")));

        let mut ctx = test_ctx();
        let result: anyhow::Result<Vec<u8>> = load_or_default(&mut ctx, &registry, "flaky");
        assert!(result.is_err(), "real failure should propagate");
    }

    /// load_or_default returns Default for unregistered providers (NotFound).
    #[test]
    fn load_or_default_returns_default_for_unregistered_provider() {
        let registry = DataRegistry::new();
        let mut ctx = test_ctx();
        let result: Vec<u8> = load_or_default(&mut ctx, &registry, "absent").expect("ok");
        assert!(result.is_empty());
    }
}
