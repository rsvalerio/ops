//! Shared `about` subpage scaffolding: warm-up + load-with-default helpers.
//!
//! DUP-1 (TASK-0464): every subpage repeats the same `for provider in [...]
//! { match get_or_provide ... }` warm-up loop and the same triadic
//! `match get_or_provide(<provider>, registry)` deserialise-or-default
//! sequence. Centralising both here keeps the four subpages aligned and
//! makes drift between their warm-up lists visible at the call site.

use serde::de::DeserializeOwned;

use ops_extension::{Context, DataProviderError, DataRegistry};

/// Build the [`Context`] an `about` subpage runs its providers against.
///
/// DUP-3 / TASK-1745: all five `run_about_*_with` runners opened with the
/// same three lines, byte for byte —
///
/// ```text
/// let cwd = std::env::current_dir()?;
/// let config = std::sync::Arc::new(ops_core::config::Config::empty());
/// let mut ctx = Context::new(config, cwd);
/// ```
///
/// This module exists to collapse exactly that class of repetition (DUP-1 /
/// TASK-0464), and it had stopped one step short of the context construction
/// preceding its own helpers. Five copies is past DUP-3's threshold and left
/// two drift surfaces open: the `Config::empty()` decision was made in five
/// places with no comment in any of them, and `current_dir()?` was propagated
/// bare from five sites.
///
/// **Why `Config::empty()` and not the loaded project config**: the about
/// subpages only ever read *data providers*, never configured commands, and
/// they must render for a directory that has no `.ops.toml` at all. An empty
/// config keeps them independent of project configuration and of whether it
/// parses. `lib.rs::run_about` is deliberately not routed through here: it
/// takes `cwd` as a parameter (the better shape — it never touches the
/// process-global current directory) and sets `refresh` on the result.
///
/// `subpage` names the caller so the `current_dir` failure is attributable
/// (ERR-4) instead of surfacing the same bare OS error from five different
/// subcommands.
///
/// # Errors
///
/// If the process's current directory cannot be determined — it was deleted,
/// or a parent component is unreadable.
pub fn subpage_context(subpage: &str) -> anyhow::Result<Context> {
    use anyhow::Context as _;

    let cwd = std::env::current_dir()
        .with_context(|| format!("about/{subpage}: could not determine the current directory"))?;
    let config = std::sync::Arc::new(ops_core::config::Config::empty());
    Ok(Context::new(config, cwd))
}

/// Warm a sequence of provider names, swallowing only `NotFound` (which is
/// expected when a provider is not registered for the active stack).
///
/// Real provider failures are surfaced at `tracing::warn!` so a
/// misbehaving provider doesn't silently zero the rendered subpage.
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
/// `Default` if the provider is not registered (`NotFound`).
///
/// Other errors are propagated so the subpage doesn't render zeros over a
/// real failure.
///
/// # Errors
///
/// If the provider fails with anything other than `NotFound` (which yields
/// `T::default()`), or if its payload does not deserialize into `T`.
pub fn load_or_default<T>(
    ctx: &mut Context,
    registry: &DataRegistry,
    provider: &str,
) -> anyhow::Result<T>
where
    T: DeserializeOwned + Default,
{
    match ctx.get_or_provide(provider, registry) {
        Ok(value) => deserialize_payload(provider, value.as_ref()),
        Err(DataProviderError::NotFound(_)) => Ok(T::default()),
        Err(e) => Err(e.into()),
    }
}

/// Deserialize a provider payload into `T`, naming the provider, the target
/// type and the failing field path when it does not fit.
///
/// ERR-4 / TASK-1734: both deserialization call sites in this crate —
/// [`load_or_default`], the single funnel for `project_coverage`,
/// `project_dependencies` and `project_units`, and `lib.rs::resolve_identity`
/// for `project_identity` — used to propagate the raw `serde_json` error with
/// a bare `?`. A stack-extension author whose payload shape had drifted saw
/// only `invalid type: string, expected i64`, with no indication of which
/// provider produced it or which type it was being read into — in a crate
/// that is otherwise meticulous about attaching `path` / `kind` / `subpage`
/// to every warn. Both facts are in scope here, so both are attached.
///
/// ERR-14: these payloads are nested (`ProjectIdentity` carries
/// `languages: Vec<LanguageStat>`, `ProjectCoverage` carries
/// `units: Vec<UnitCoverage>`), so the bare message could refer to any of a
/// hundred fields. `serde_path_to_error` reports the concrete location —
/// `units[3].lines_percent` — which is the difference between a fixable bug
/// report and a bisect.
///
/// PERF-3 (TASK-1117): still borrows the payload. `serde_path_to_error`
/// wraps the `&Value` deserializer, so the JSON tree is not deep-cloned to
/// feed `from_value` (which takes `Value` by value).
///
/// # Errors
///
/// If `value` does not deserialize into `T`.
pub fn deserialize_payload<T>(provider: &str, value: &serde_json::Value) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    serde_path_to_error::deserialize(value).map_err(|e| {
        let path = e.path().to_string();
        let target = std::any::type_name::<T>();
        // `Path`'s Display renders the root as "."; spell that out rather
        // than emitting a message that trails off in a bare dot.
        let at = if path == "." { "<root>" } else { path.as_str() };
        anyhow::Error::new(e.into_inner()).context(format!(
            "provider `{provider}` payload did not match {target} (at `{at}`)"
        ))
    })
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

    /// DUP-3 / TASK-1745: the shared subpage context is built against the
    /// process cwd and an empty `Config` — the decision the five runners each
    /// used to make silently.
    #[test]
    fn subpage_context_uses_the_cwd_and_an_empty_config() {
        let ctx = subpage_context("units").expect("cwd is readable in the test harness");
        assert_eq!(
            ctx.working_directory(),
            std::env::current_dir().unwrap().as_path()
        );
        let empty = ops_core::config::Config::empty();
        assert_eq!(
            ctx.config().commands.len(),
            empty.commands.len(),
            "subpages run against an empty config, not the loaded project config"
        );
    }

    /// ERR-1 (TASK-0516): a non-NotFound provider error during warm-up
    /// must not propagate (warm-up is best-effort) and must not panic. The
    /// warn fires through tracing; pinning the value-level contract here
    /// avoids the tracing-subscriber dev-dep cost (matches the pattern in
    /// `code::tests::query_language_stats_returns_none_when_db_lock_poisoned`).
    #[test]
    fn warm_providers_swallows_real_failures_without_panic() {
        let mut registry = DataRegistry::new();
        let _ = registry.register("flaky", Box::new(FailingProvider("flaky")));

        let mut ctx = test_ctx();
        warm_providers(&mut ctx, &registry, &["flaky", "absent"], "test");
        // Reaching this line means warm-up returned cleanly for both a
        // failing-provider error and an unregistered (NotFound) provider.
    }

    /// ERR-1 (TASK-0516): `load_or_default` surfaces non-NotFound errors so a
    /// failing provider doesn't render zeros over a real failure.
    #[test]
    fn load_or_default_propagates_real_failures() {
        let mut registry = DataRegistry::new();
        let _ = registry.register("flaky", Box::new(FailingProvider("flaky")));

        let mut ctx = test_ctx();
        let result: anyhow::Result<Vec<u8>> = load_or_default(&mut ctx, &registry, "flaky");
        assert!(result.is_err(), "real failure should propagate");
    }

    /// A provider whose payload has drifted from the type the subpage expects.
    /// Mirrors the real failure mode: a nested field of the wrong JSON type.
    struct DriftedProvider(&'static str);
    impl DataProvider for DriftedProvider {
        fn name(&self) -> &'static str {
            self.0
        }
        fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            Ok(serde_json::json!({
                "units": [
                    { "name": "alpha", "lines_percent": 91.5 },
                    { "name": "beta", "lines_percent": "not-a-number" },
                ]
            }))
        }
    }

    #[derive(Debug, Default, serde::Deserialize)]
    struct UnitCoverageStub {
        #[allow(dead_code)]
        name: String,
        #[allow(dead_code)]
        lines_percent: f64,
    }

    #[derive(Debug, Default, serde::Deserialize)]
    struct CoverageStub {
        #[allow(dead_code)]
        units: Vec<UnitCoverageStub>,
    }

    /// ERR-4 + ERR-14 (TASK-1734): a payload-shape failure must name the
    /// provider that produced it, the type it was being read into, and the
    /// concrete field path. Previously the whole message was
    /// `invalid type: string, expected f64` — unattributable across four
    /// subpages and a hundred fields.
    #[test]
    fn load_or_default_error_names_provider_and_field_path() {
        let mut registry = DataRegistry::new();
        let _ = registry.register(
            "project_coverage",
            Box::new(DriftedProvider("project_coverage")),
        );

        let mut ctx = test_ctx();
        let err = load_or_default::<CoverageStub>(&mut ctx, &registry, "project_coverage")
            .expect_err("a drifted payload must not deserialize");
        let rendered = format!("{err:#}");

        assert!(
            rendered.contains("project_coverage"),
            "error must name the provider; got {rendered}"
        );
        assert!(
            rendered.contains("CoverageStub"),
            "error must name the target type; got {rendered}"
        );
        assert!(
            rendered.contains("units[1].lines_percent"),
            "error must report the failing field path; got {rendered}"
        );
        // The underlying serde message is preserved, not replaced.
        assert!(
            rendered.contains("invalid type"),
            "error must keep the serde cause; got {rendered}"
        );
    }

    /// A root-level type mismatch has no field path; the message must still
    /// be attributable rather than trailing off in a bare dot.
    #[test]
    fn deserialize_payload_labels_a_root_level_mismatch() {
        let err = deserialize_payload::<CoverageStub>("project_coverage", &serde_json::json!(7))
            .expect_err("an integer is not a CoverageStub");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("`<root>`"),
            "root mismatch must be labelled; got {rendered}"
        );
        assert!(rendered.contains("project_coverage"), "got {rendered}");
    }

    /// `load_or_default` returns Default for unregistered providers (`NotFound`).
    #[test]
    fn load_or_default_returns_default_for_unregistered_provider() {
        let registry = DataRegistry::new();
        let mut ctx = test_ctx();
        let result: Vec<u8> = load_or_default(&mut ctx, &registry, "absent").expect("ok");
        assert!(result.is_empty());
    }
}
