use super::*;
use ops_core::config::{CommandId, CommandSpec, ExecCommandSpec};
use std::path::PathBuf;
use std::sync::Arc;

struct StubProvider;
impl DataProvider for StubProvider {
    fn name(&self) -> &'static str {
        "stub"
    }
    fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        Ok(serde_json::json!({"key": "value"}))
    }
}

fn test_context() -> Context {
    Context::test_context(PathBuf::from("."))
}

#[test]
fn data_registry_provide_unknown_returns_error() {
    let registry = DataRegistry::new();
    let mut ctx = test_context();
    let result = registry.provide("nonexistent", &mut ctx);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

/// API-9 / TASK-1179: `DataRegistry::IntoIterator` must yield entries in
/// registration order so audit-trail consumers (and any CLI wiring code
/// that walks the registry directly) see deterministic ordering. Two
/// registries built from the same insertion sequence iterate identically;
/// pre-fix, hashbrown's randomised iteration order broke that pin.
#[test]
fn data_registry_into_iter_yields_insertion_order_and_is_stable() {
    let names = [
        "zeta", "alpha", "delta", "mu", "beta", "lambda", "gamma", "epsilon",
    ];
    let build = || {
        let mut r = DataRegistry::new();
        for n in names {
            let _ = r.register(n.to_string(), Box::new(StubProvider));
        }
        r
    };
    let observed: Vec<String> = build().into_iter().map(|(n, _)| n).collect();
    assert_eq!(observed, names, "iteration must follow insertion order");
    let observed_again: Vec<String> = build().into_iter().map(|(n, _)| n).collect();
    assert_eq!(
        observed, observed_again,
        "two registries built from the same insertion sequence must iterate identically",
    );
}

#[test]
fn data_registry_register_and_get() {
    let mut registry = DataRegistry::new();
    let _ = registry.register("stub", Box::new(StubProvider));
    assert!(registry.get("stub").is_some());
    assert!(registry.get("other").is_none());
}

/// SEC-31 / TASK-0350 + CL-5 / TASK-0756: registering two providers under
/// the same name must (1) be rejected first-write-wins and (2) record the
/// rejected name in the audit trail so the CLI wiring layer can emit a
/// single `tracing::warn` from one place. The earlier
/// `debug_assert!(false, …)` panic was retired because it forced every
/// in-extension duplicate to surface as a test panic instead of letting the
/// wiring layer's per-extension scratch registry aggregate the audit trail.
#[test]
fn data_registry_register_duplicate_records_audit_and_keeps_first() {
    let mut registry = DataRegistry::new();
    let _ = registry.register("stub", Box::new(StubProvider));
    let _ = registry.register("stub", Box::new(StubProvider));
    assert!(
        registry.get("stub").is_some(),
        "first-write-wins must keep the original provider"
    );
    let dups = registry.take_duplicate_inserts();
    assert_eq!(
        dups,
        vec!["stub".to_string()],
        "the rejected name must be recorded for the wiring layer to warn on"
    );
    assert!(
        registry.take_duplicate_inserts().is_empty(),
        "draining the audit trail clears it"
    );
}

#[test]
fn data_registry_provide_returns_value() {
    let mut registry = DataRegistry::new();
    let _ = registry.register("stub", Box::new(StubProvider));
    let mut ctx = test_context();
    let value = registry.provide("stub", &mut ctx).expect("should succeed");
    assert_eq!(value, serde_json::json!({"key": "value"}));
}

/// ERR-1 / TASK-1170: a Context built with `with_refresh()` (or any caller
/// flipping `refresh = true`) must bypass the `data_cache` fast path so the
/// provider is re-invoked. Pre-fix, `get_or_provide` returned the cached
/// value regardless of the refresh flag, making `--refresh` a no-op for any
/// key already populated within the runner's persistent context lifetime.
#[test]
fn context_get_or_provide_refresh_bypasses_cache() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingProvider {
        calls: Arc<AtomicUsize>,
    }
    impl DataProvider for CountingProvider {
        fn name(&self) -> &'static str {
            "counter"
        }
        fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            let n = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
            Ok(serde_json::json!({ "calls": n }))
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = DataRegistry::new();
    let _ = registry.register(
        "counter",
        Box::new(CountingProvider {
            calls: Arc::clone(&calls),
        }),
    );

    let mut ctx = test_context();
    ctx.get_or_provide("counter", &registry).expect("first");
    ctx.get_or_provide("counter", &registry).expect("cached");
    assert_eq!(
        calls.load(Ordering::Relaxed),
        1,
        "second call must be cached"
    );

    let mut refreshing = test_context().with_refresh();
    refreshing
        .get_or_provide("counter", &registry)
        .expect("refresh-first");
    refreshing
        .get_or_provide("counter", &registry)
        .expect("refresh-second");
    assert_eq!(
        calls.load(Ordering::Relaxed),
        3,
        "refresh=true must bypass the data_cache and re-invoke the provider"
    );
    let cached = refreshing.cached("counter").expect("refresh stores result");
    assert_eq!(cached.as_ref(), &serde_json::json!({ "calls": 3 }));
}

#[test]
fn context_get_or_provide_caches() {
    let mut registry = DataRegistry::new();
    let _ = registry.register("stub", Box::new(StubProvider));
    let mut ctx = test_context();

    let v1 = ctx.get_or_provide("stub", &registry).expect("first call");
    let v2 = ctx
        .get_or_provide("stub", &registry)
        .expect("second call (cached)");
    assert_eq!(*v1, *v2);
    assert!(ctx.cached("stub").is_some());
}

/// SEC-38 / TASK-0744: two providers that mutually request each other must
/// surface as `DataProviderError::Cycle` rather than recursing until stack
/// overflow. The `provide` impls below model the documented composition
/// pattern (a provider calling `ctx.get_or_provide(other, registry)`) so the
/// test exercises the real re-entry path through `get_or_provide`.
#[test]
fn context_get_or_provide_detects_provider_cycle() {
    use std::sync::Mutex;

    /// A provider that, when invoked, calls `ctx.get_or_provide(other, ...)`
    /// and surfaces the resulting error verbatim. The companion provider's
    /// name is fetched from a Mutex so we can wire the registry first and
    /// then connect the two providers without a chicken-and-egg construction.
    struct ChainProvider {
        name: &'static str,
        other: &'static str,
        registry: Arc<Mutex<Option<Arc<DataRegistry>>>>,
    }
    impl DataProvider for ChainProvider {
        fn name(&self) -> &'static str {
            self.name
        }
        fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            let reg_handle = self
                .registry
                .lock()
                .unwrap()
                .as_ref()
                .expect("registry wired")
                .clone();
            let _ = ctx.get_or_provide(self.other, &reg_handle)?;
            Ok(serde_json::json!({"unreachable": self.name}))
        }
    }

    let shared: Arc<Mutex<Option<Arc<DataRegistry>>>> = Arc::new(Mutex::new(None));
    let mut registry = DataRegistry::new();
    let _ = registry.register(
        "alpha",
        Box::new(ChainProvider {
            name: "alpha",
            other: "beta",
            registry: Arc::clone(&shared),
        }),
    );
    let _ = registry.register(
        "beta",
        Box::new(ChainProvider {
            name: "beta",
            other: "alpha",
            registry: Arc::clone(&shared),
        }),
    );
    let registry = Arc::new(registry);
    *shared.lock().unwrap() = Some(Arc::clone(&registry));

    let mut ctx = test_context();
    let err = ctx
        .get_or_provide("alpha", &registry)
        .expect_err("cycle must surface as an error");
    match err {
        DataProviderError::Cycle { key } => assert_eq!(key, "alpha"),
        other => panic!("expected Cycle{{alpha}}, got {other:?}"),
    }
    // After the cycle bottom-out, the in-flight set must be drained so a
    // subsequent unrelated call is not poisoned.
    assert!(
        ctx.cached("alpha").is_none(),
        "failed cycle must not poison the cache"
    );
}

#[test]
fn context_get_or_provide_unknown_errors() {
    let registry = DataRegistry::new();
    let mut ctx = test_context();
    let result = ctx.get_or_provide("missing", &registry);
    assert!(result.is_err());
}

/// ERR-2 / TASK-1887: `computation_failed` now produces the message-carrying
/// `ComputationMessage` variant. The rendered string is unchanged, which is
/// the part log readers depend on.
#[test]
fn data_provider_error_computation_failed() {
    let err = DataProviderError::computation_failed("test computation error");
    assert_eq!(
        err.to_string(),
        "data computation failed: test computation error"
    );
    assert!(matches!(err, DataProviderError::ComputationMessage(_)));
}

struct TestExtension;
impl Extension for TestExtension {
    fn name(&self) -> &'static str {
        "test_ext"
    }
    fn register_commands(&self, registry: &mut CommandRegistry) {
        registry.insert(
            "ext_cmd".into(),
            CommandSpec::Exec(ExecCommandSpec::new("echo", ["from_ext"])),
        );
    }
}

#[test]
fn provider_names_returns_sorted() {
    let mut registry = DataRegistry::new();
    let _ = registry.register("zebra", Box::new(StubProvider));
    let _ = registry.register("alpha", Box::new(StubProvider));
    assert_eq!(registry.provider_names(), vec!["alpha", "zebra"]);
}

#[test]
fn provider_names_empty_registry() {
    let registry = DataRegistry::new();
    assert!(registry.provider_names().is_empty());
}

#[test]
fn extension_registers_commands() {
    let ext = TestExtension;
    let mut registry = CommandRegistry::new();
    ext.register_commands(&mut registry);
    assert!(registry.contains_key("ext_cmd"));
}

// --- SharedError tests ---

#[test]
fn shared_error_display_shows_inner_message() {
    let inner = std::io::Error::other("disk full");
    let shared = SharedError::new(inner);
    assert_eq!(shared.to_string(), "disk full");
}

#[test]
fn shared_error_source_chain_preserved() {
    use std::error::Error;
    // A custom error with a source
    #[derive(Debug)]
    struct Outer(std::io::Error);
    impl std::fmt::Display for Outer {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "outer")
        }
    }
    impl std::error::Error for Outer {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }
    let outer = Outer(std::io::Error::other("root cause"));
    let shared = SharedError::new(outer);

    // ERR-1 / TASK-2024: the first link is the *wrapped error itself*. This
    // test previously asserted that `source()` skipped straight to "root
    // cause", which is exactly the missing link the fix restores — the
    // wrapped `Outer` was unreachable by any chain walk or downcast.
    let first = shared
        .source()
        .expect("the wrapped error is the first link");
    assert_eq!(first.to_string(), "outer");
    assert!(
        first.downcast_ref::<Outer>().is_some(),
        "the wrapped error must be downcastable through the chain"
    );

    // …and the rest of the chain still follows from there.
    let root = first.source().expect("the wrapped error's own source");
    assert!(root.to_string().contains("root cause"), "got: {root}");
}

#[test]
fn shared_error_from_anyhow() {
    let anyhow_err = anyhow::anyhow!("anyhow message");
    let shared = SharedError::from(anyhow_err);
    assert!(shared.to_string().contains("anyhow message"));
}

#[test]
fn shared_error_from_serde_json() {
    let bad_json: Result<serde_json::Value, _> = serde_json::from_str("{invalid");
    let json_err = bad_json.unwrap_err();
    let shared = SharedError::from(json_err);
    assert!(!shared.to_string().is_empty());
}

// --- SharedError / DataProviderError alternate-format chain rendering ---

/// Regression guard for the diagnosability bug where
/// `tracing::warn!("…: {e:#}")` on a `DataProviderError` showed only the
/// outermost context (`provide_via_ingestor(coverage_files)`: ingestor
/// collect) and silently dropped the root cause (`cargo llvm-cov exited
/// with status 101: …`). The anyhow alternate flag never propagates
/// through thiserror's nested `{0}` display, so `SharedError` must honor
/// it itself by walking its source chain.
#[test]
fn computation_failed_alternate_display_surfaces_root_cause() {
    let inner = anyhow::anyhow!("cargo llvm-cov exited with status 101: no such command")
        .context("ingestor collect")
        .context("provide_via_ingestor(coverage_files)");
    let e = DataProviderError::from(inner);
    let rendered = format!("{e:#}");
    for layer in [
        "data computation failed",
        "provide_via_ingestor(coverage_files)",
        "ingestor collect",
        "cargo llvm-cov exited with status 101",
    ] {
        assert!(
            rendered.contains(layer),
            "alternate display must surface `{layer}`; got: {rendered}"
        );
    }
}

/// Plain display also flattens the chain: the `{0:#}` in the variant's
/// format string applies the flag unconditionally (parity with
/// `DbError::External`), so `to_string()` keeps the root cause visible on
/// every display path, not just `{:#}`.
#[test]
fn computation_failed_plain_display_flattens_chain() {
    let inner = anyhow::anyhow!("root cause").context("outer context");
    let e = DataProviderError::from(inner);
    assert_eq!(
        e.to_string(),
        "data computation failed: outer context: root cause"
    );
}

/// `SharedError` itself is alternate-aware: `{:#}` walks the source chain,
/// `{}` renders the top-level message only.
#[test]
fn shared_error_alternate_display_walks_source_chain() {
    let inner = anyhow::anyhow!("root").context("middle").context("top");
    let shared = SharedError::from(inner);
    assert_eq!(shared.to_string(), "top");
    assert_eq!(format!("{shared:#}"), "top: middle: root");
}

/// A sourceless error renders identically with and without the alternate
/// flag — the chain walk must not append separators to nothing.
#[test]
fn shared_error_alternate_display_matches_plain_when_no_sources() {
    let shared = SharedError::new(std::io::Error::other("disk full"));
    assert_eq!(shared.to_string(), "disk full");
    assert_eq!(format!("{shared:#}"), "disk full");
}

// --- ExtensionType tests ---

#[test]
fn extension_type_is_datasource() {
    let t = ExtensionType::DATASOURCE;
    assert!(t.is_datasource());
    assert!(!t.is_command());
}

#[test]
fn extension_type_is_command() {
    let t = ExtensionType::COMMAND;
    assert!(t.is_command());
    assert!(!t.is_datasource());
}

#[test]
fn extension_type_combined() {
    let t = ExtensionType::DATASOURCE | ExtensionType::COMMAND;
    assert!(t.is_datasource());
    assert!(t.is_command());
}

#[test]
fn extension_type_empty() {
    let t = ExtensionType::empty();
    assert!(!t.is_datasource());
    assert!(!t.is_command());
}

// --- DataProviderError constructors ---

#[test]
fn data_provider_error_not_found() {
    let err = DataProviderError::not_found("missing_provider");
    assert!(err.to_string().contains("missing_provider"));
    assert!(matches!(err, DataProviderError::NotFound(_)));
}

#[test]
fn data_provider_error_computation_error_from_source() {
    let source = std::io::Error::other("io broke");
    let err = DataProviderError::computation_error(source);
    assert!(err.to_string().contains("io broke"));
    assert!(matches!(err, DataProviderError::ComputationFailed(_)));
}

#[test]
fn data_provider_error_from_anyhow() {
    let anyhow_err = anyhow::anyhow!("anyhow computation error");
    let err = DataProviderError::from(anyhow_err);
    assert!(matches!(err, DataProviderError::ComputationFailed(_)));
    assert!(err.to_string().contains("anyhow computation error"));
}

#[test]
fn data_provider_error_from_serde_json() {
    let json_err: serde_json::Error = serde_json::from_str::<String>("not json").unwrap_err();
    let err = DataProviderError::from(json_err);
    assert!(matches!(err, DataProviderError::Serialization(_)));
}

#[test]
fn data_provider_error_source_chain() {
    use std::error::Error;
    let err = DataProviderError::computation_error(std::io::Error::other("root"));
    assert!(err.source().is_some());
}

// --- DataRegistry::schemas ---

struct SchemaProvider;
impl DataProvider for SchemaProvider {
    fn name(&self) -> &'static str {
        "schematic"
    }
    fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        Ok(serde_json::json!({}))
    }
    fn schema(&self) -> DataProviderSchema {
        DataProviderSchema {
            description: "A test schema",
            fields: vec![
                data_field!("field_a", "str", "First field"),
                data_field!("field_b", "int", "Second field"),
            ],
        }
    }
}

#[test]
fn data_registry_schemas_returns_sorted() {
    let mut registry = DataRegistry::new();
    let _ = registry.register("zzz", Box::new(SchemaProvider));
    let _ = registry.register("aaa", Box::new(StubProvider));
    let schemas = registry.schemas();
    assert_eq!(schemas.len(), 2);
    assert_eq!(schemas[0].0, "aaa");
    assert_eq!(schemas[1].0, "zzz");
    assert_eq!(schemas[1].1.fields.len(), 2);
    assert_eq!(schemas[1].1.fields[0].name, "field_a");
}

#[test]
fn data_registry_schemas_empty() {
    let registry = DataRegistry::new();
    assert!(registry.schemas().is_empty());
}

// --- data_field! macro ---

#[test]
fn data_field_macro_constructs_correctly() {
    let field = data_field!("name", "str", "The name");
    assert_eq!(field.name, "name");
    assert_eq!(field.type_name, "str");
    assert_eq!(field.description, "The name");
}

// --- Context tests ---

#[test]
fn context_with_refresh_sets_flag() {
    let ctx = test_context().with_refresh();
    assert!(ctx.is_refreshing());
}

#[test]
fn context_default_refresh_is_false() {
    let ctx = test_context();
    assert!(!ctx.is_refreshing());
}

#[test]
fn context_working_directory() {
    let ctx = Context::test_context(PathBuf::from("/tmp/test"));
    assert_eq!(ctx.working_directory(), PathBuf::from("/tmp/test"));
}

// --- Extension trait defaults ---
// Verifies all default implementations return expected values and that
// info() correctly aggregates them. Consolidated from per-method tests.

#[test]
fn extension_defaults_and_info_aggregation() {
    let ext = TestExtension;

    // Individual defaults
    assert_eq!(ext.description(), "");
    assert_eq!(ext.shortname(), ext.name());
    assert_eq!(ext.types(), ExtensionType::empty());
    assert!(ext.command_names().is_empty());
    assert!(ext.data_provider_name().is_none());
    assert!(ext.stack().is_none());

    // info() aggregates all defaults
    let info = ext.info();
    assert_eq!(info.name, "test_ext");
    assert_eq!(info.shortname, "test_ext");
    assert_eq!(info.description, "");
    assert_eq!(info.types, ExtensionType::empty());
    assert!(info.command_names.is_empty());
    assert!(info.data_provider_name.is_none());

    // register_data_providers is a no-op
    let mut registry = DataRegistry::new();
    ext.register_data_providers(&mut registry);
    assert!(registry.provider_names().is_empty());
}

// --- impl_extension! macro ---

struct MacroTestExtFull;
impl_extension! {
    MacroTestExtFull,
    name: "macro-full",
    description: "A macro-generated extension",
    shortname: "mf",
    types: ExtensionType::DATASOURCE | ExtensionType::COMMAND,
    command_names: &["cmd1", "cmd2"],
    data_provider_name: Some("macro_data"),
    register_commands: |_self_cmd, registry| {
        registry.insert(
            "cmd1".into(),
            CommandSpec::Exec(ExecCommandSpec::new("echo", ["macro"])),
        );
    },
    register_data_providers: |_self_dp, registry| {
        let _ = registry.register("macro_data", Box::new(StubProvider));
    },
}

#[test]
fn impl_extension_macro_full_form() {
    let ext = MacroTestExtFull;
    assert_eq!(ext.name(), "macro-full");
    assert_eq!(ext.description(), "A macro-generated extension");
    assert_eq!(ext.shortname(), "mf");
    assert!(ext.types().is_datasource());
    assert!(ext.types().is_command());
    assert_eq!(ext.command_names(), &["cmd1", "cmd2"]);
    assert_eq!(ext.data_provider_name(), Some("macro_data"));

    let mut cmd_reg = CommandRegistry::new();
    ext.register_commands(&mut cmd_reg);
    assert!(cmd_reg.contains_key("cmd1"));

    let mut data_reg = DataRegistry::new();
    ext.register_data_providers(&mut data_reg);
    assert!(data_reg.get("macro_data").is_some());
}

struct MacroTestExtShort;
impl_extension! {
    MacroTestExtShort,
    name: "macro-short",
    description: "Short form extension",
    shortname: "ms",
    types: ExtensionType::DATASOURCE,
    data_provider_name: Some("short_data"),
    register_data_providers: |_self_dp, registry| {
        let _ = registry.register("short_data", Box::new(StubProvider));
    },
}

#[test]
fn impl_extension_macro_short_form() {
    let ext = MacroTestExtShort;
    assert_eq!(ext.name(), "macro-short");
    assert_eq!(ext.shortname(), "ms");
    assert!(ext.types().is_datasource());
    assert!(!ext.types().is_command());

    let mut cmd_reg = CommandRegistry::new();
    ext.register_commands(&mut cmd_reg);
    assert!(cmd_reg.is_empty());

    let mut data_reg = DataRegistry::new();
    ext.register_data_providers(&mut data_reg);
    assert!(data_reg.get("short_data").is_some());
}

#[test]
fn impl_extension_macro_info() {
    let ext = MacroTestExtFull;
    let info = ext.info();
    assert_eq!(info.name, "macro-full");
    assert_eq!(info.shortname, "mf");
    assert_eq!(info.description, "A macro-generated extension");
    assert!(info.types.is_datasource());
    assert!(info.types.is_command());
    assert_eq!(info.command_names, &["cmd1", "cmd2"]);
    assert_eq!(info.data_provider_name, Some("macro_data"));
}

// --- DataProviderError is Clone ---

#[test]
fn data_provider_error_is_clone() {
    #[derive(Debug)]
    struct WithSource(std::io::Error);
    impl std::fmt::Display for WithSource {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("outer")
        }
    }
    impl std::error::Error for WithSource {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    let err = DataProviderError::computation_error(WithSource(std::io::Error::other("inner")));
    let cloned = err.clone();

    assert_eq!(err.to_string(), cloned.to_string());
    assert!(matches!(cloned, DataProviderError::ComputationFailed(_)));
    // Source chain survives the clone.
    assert!(std::error::Error::source(&cloned).is_some());

    // EFF-002: Clone reuses the inner Arc rather than rewrapping the error.
    let (DataProviderError::ComputationFailed(orig), DataProviderError::ComputationFailed(copy)) =
        (&err, &cloned)
    else {
        panic!("expected ComputationFailed variants");
    };
    assert!(orig.shares_allocation_with(copy));
}

// --- DataRegistry::about_fields ---

struct AboutFieldProvider;
impl DataProvider for AboutFieldProvider {
    fn name(&self) -> &'static str {
        "identity"
    }
    fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        Ok(serde_json::json!({}))
    }
    fn about_fields(&self) -> Vec<ops_core::project_identity::AboutFieldDef> {
        vec![
            ops_core::project_identity::AboutFieldDef {
                id: "project",
                label: "Project",
                description: "Project name",
            },
            ops_core::project_identity::AboutFieldDef {
                id: "version",
                label: "Version",
                description: "Project version",
            },
        ]
    }
}

#[test]
fn data_registry_about_fields_returns_provider_fields() {
    let mut registry = DataRegistry::new();
    let _ = registry.register("identity", Box::new(AboutFieldProvider));
    let fields = registry.about_fields("identity");
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].id, "project");
    assert_eq!(fields[1].id, "version");
}

#[test]
fn data_registry_about_fields_unknown_provider_returns_empty() {
    let registry = DataRegistry::new();
    let fields = registry.about_fields("nonexistent");
    assert!(fields.is_empty());
}

#[test]
fn data_provider_default_about_fields_is_empty() {
    let provider = StubProvider;
    assert!(provider.about_fields().is_empty());
}

// --- DataRegistry::default ---

#[test]
fn data_registry_default() {
    let registry = DataRegistry::default();
    assert!(registry.provider_names().is_empty());
}

/// DUP-3 / TASK-1225: building a `CommandRegistry` via `collect()` /
/// `from_iter()` must NOT silently drop the duplicate-insert audit
/// trail. The implementation drains `duplicate_inserts` and surfaces
/// each duplicate via `tracing::warn!`, so the audit signal that
/// ERR-2 / TASK-0579 hardened the `.insert()` path to preserve also
/// reaches `collect()` consumers.
#[test]
fn command_registry_from_iter_drains_duplicate_audit_trail() {
    let id = CommandId::new("dup");
    let entries: Vec<(CommandId, CommandSpec)> = vec![
        (
            id.clone(),
            CommandSpec::Exec(ExecCommandSpec::new("a", ["x"])),
        ),
        (
            id.clone(),
            CommandSpec::Exec(ExecCommandSpec::new("b", ["y"])),
        ),
    ];

    let mut reg: CommandRegistry = entries.into_iter().collect();

    // CommandRegistry::insert is last-write-wins (extension overrides
    // are intentional); the audit trail surfaces the override regardless.
    if let Some(CommandSpec::Exec(e)) = reg.get(&id) {
        assert_eq!(e.program, "b");
    } else {
        panic!("unexpected spec variant");
    }

    // FromIterator drained the audit trail itself; subsequent callers
    // observe an empty Vec — exactly the contract the implementation
    // promises (no silent loss; warnings already emitted).
    assert!(
        reg.take_duplicate_inserts().is_empty(),
        "FromIterator must drain the duplicate audit trail in place of the caller"
    );
}

/// SEC-21 / TASK-1226: `DataRegistry::register` formats the runtime-
/// generated `provider_name` field via the `?` (Debug) formatter so an
/// extension that builds a provider name from external data containing
/// newlines or ANSI sequences cannot forge log entries through the
/// duplicate-insert breadcrumb. Pin the value-level escape directly by
/// driving `DataRegistry::register` with a forged provider name and
/// capturing the emitted tracing event — flipping the format specifier
/// from `?name` (Debug) to `%name` (Display) in `data.rs` would let raw
/// newline / ESC bytes through and fail this test, mirroring
/// `program_field_debug_escapes_control_characters` (TASK-1127) and the
/// broader workspace policy.
#[test]
fn provider_name_field_debug_escapes_control_characters() {
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct BufWriter(Arc<Mutex<Vec<u8>>>);
    impl Write for BufWriter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("lock").extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> MakeWriter<'a> for BufWriter {
        type Writer = Self;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    let forged = "stub\nFAKE_LOG\n\u{1b}[31m";
    let buf = BufWriter::default();
    let captured = buf.0.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buf)
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(false)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let mut registry = DataRegistry::new();
        let _ = registry.register(forged, Box::new(StubProvider));
        // Duplicate insert triggers the breadcrumb under test.
        let _ = registry.register(forged, Box::new(StubProvider));
    });

    let text = String::from_utf8(captured.lock().expect("lock").clone()).expect("utf8");
    assert!(
        text.contains("DataRegistry::register rejecting duplicate provider"),
        "expected duplicate-insert breadcrumb in captured tracing output, got: {text}"
    );
    // The raw ESC byte must not appear — Debug formatting must have
    // escaped it. If the format specifier in data.rs is flipped from
    // `?name` (Debug) to `%name` (Display), the raw ESC byte leaks into
    // the breadcrumb and this assertion fails.
    assert!(
        !text.contains('\u{1b}'),
        "captured breadcrumb must not contain raw ESC byte: {text:?}"
    );
    // The escaped forms must appear in the provider_name field. The
    // breadcrumb line itself ends with a real `\n` (record terminator),
    // so we cannot assert the entire `text` is newline-free; instead we
    // pin that Debug produced the escape sequence `\n` and `\u{1b}` in
    // the rendered field.
    assert!(
        text.contains("\\n"),
        "captured breadcrumb must contain escaped newline (\\n): {text:?}"
    );
    assert!(
        text.contains("\\u{1b}"),
        "captured breadcrumb must contain escaped ESC (\\u{{1b}}): {text:?}"
    );
}

// ---------------------------------------------------------------------------
// SEC-38 / TASK-1865 — the cycle guard sits at the dispatch point
// ---------------------------------------------------------------------------

/// A provider that composes a peer through `registry.provide(...)` — the
/// *unguarded* entry point before TASK-1865 — and surfaces the error verbatim.
struct RegistryChainProvider {
    name: &'static str,
    other: &'static str,
    registry: Arc<std::sync::Mutex<Option<Arc<DataRegistry>>>>,
}

impl DataProvider for RegistryChainProvider {
    fn name(&self) -> &'static str {
        self.name
    }
    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let reg = self
            .registry
            .lock()
            .unwrap()
            .as_ref()
            .expect("registry wired")
            .clone();
        let _ = reg.provide(self.other, ctx)?;
        Ok(serde_json::json!({ "unreachable": self.name }))
    }
}

/// Wire an A -> B -> A cycle whose providers compose through
/// `DataRegistry::provide` rather than `Context::get_or_provide`.
fn cyclic_registry() -> Arc<DataRegistry> {
    let shared: Arc<std::sync::Mutex<Option<Arc<DataRegistry>>>> =
        Arc::new(std::sync::Mutex::new(None));
    let mut registry = DataRegistry::new();
    let _ = registry.register(
        "alpha",
        Box::new(RegistryChainProvider {
            name: "alpha",
            other: "beta",
            registry: Arc::clone(&shared),
        }),
    );
    let _ = registry.register(
        "beta",
        Box::new(RegistryChainProvider {
            name: "beta",
            other: "alpha",
            registry: Arc::clone(&shared),
        }),
    );
    let registry = Arc::new(registry);
    *shared.lock().unwrap() = Some(Arc::clone(&registry));
    registry
}

/// SEC-38 / TASK-1865: driving the cycle through `DataRegistry::provide`
/// directly — never touching `Context::get_or_provide` — must terminate with
/// `Cycle`. Before the fix the guard lived only in the caching wrapper, so
/// this path recursed until the stack overflowed (an abort, not an error).
#[test]
fn data_registry_provide_detects_cycle_without_the_cache_wrapper() {
    let registry = cyclic_registry();
    let mut ctx = test_context();
    let err = registry
        .provide("alpha", &mut ctx)
        .expect_err("cycle must surface as an error");
    match err {
        DataProviderError::Cycle { key } => assert_eq!(key, "alpha"),
        other => panic!("expected Cycle{{alpha}}, got {other:?}"),
    }
}

/// SEC-38 / TASK-1865: the other public route to a provider —
/// `DataRegistry::get` handing out a `&dyn DataProvider` — is also bounded.
/// The first hop is unmarked (nothing dispatched it), but every hop it makes
/// crosses `DataRegistry::provide`, so the cycle closes one step later on
/// `beta` instead of recursing forever.
#[test]
fn dyn_data_provider_obtained_from_get_still_bottoms_out_on_cycle() {
    let registry = cyclic_registry();
    let mut ctx = test_context();
    let provider = registry.get("alpha").expect("alpha registered");
    let err = provider
        .provide(&mut ctx)
        .expect_err("cycle must surface as an error");
    match err {
        DataProviderError::Cycle { key } => assert_eq!(
            key, "beta",
            "the unmarked first hop shifts the detection point by one provider"
        ),
        other => panic!("expected Cycle, got {other:?}"),
    }
}

/// The guard must be released on the failure path too, or one failed lookup
/// would permanently poison the key on a long-lived runner context.
#[test]
fn failed_provider_clears_its_in_flight_marker() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FlakyProvider {
        calls: Arc<AtomicUsize>,
    }
    impl DataProvider for FlakyProvider {
        fn name(&self) -> &'static str {
            "flaky"
        }
        fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            if self.calls.fetch_add(1, Ordering::Relaxed) == 0 {
                Err(DataProviderError::computation_failed("first attempt fails"))
            } else {
                Ok(serde_json::json!({ "ok": true }))
            }
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = DataRegistry::new();
    let _ = registry.register(
        "flaky",
        Box::new(FlakyProvider {
            calls: Arc::clone(&calls),
        }),
    );
    let mut ctx = test_context();
    assert!(ctx.get_or_provide("flaky", &registry).is_err());
    let v = ctx
        .get_or_provide("flaky", &registry)
        .expect("retry must not report a phantom cycle");
    assert_eq!(v.as_ref(), &serde_json::json!({ "ok": true }));
}

// ---------------------------------------------------------------------------
// CL-3 / TASK-1872 — register communicates rejection through its return type
// ---------------------------------------------------------------------------

/// A provider whose `name()` identifies which instance came back, so the test
/// can assert the *rejected* value is returned rather than the stored one.
struct NamedProvider(&'static str);
impl DataProvider for NamedProvider {
    fn name(&self) -> &'static str {
        self.0
    }
    fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        Ok(serde_json::json!({ "from": self.0 }))
    }
}

#[test]
fn register_returns_none_on_fresh_insert_and_the_rejected_provider_on_duplicate() {
    let mut registry = DataRegistry::new();
    assert!(
        registry
            .register("thing", Box::new(NamedProvider("first")))
            .is_none(),
        "a fresh key must report the no-op variant"
    );

    let rejected = registry
        .register("thing", Box::new(NamedProvider("second")))
        .expect("a duplicate must hand the rejected provider back");
    assert_eq!(
        rejected.name(),
        "second",
        "the returned value must be the incoming provider, not the stored one"
    );
    assert_eq!(
        registry.get("thing").expect("first-write-wins").name(),
        "first",
        "the first registration must still own the key"
    );
    assert_eq!(registry.take_duplicate_inserts(), vec!["thing".to_string()]);
}

// ---------------------------------------------------------------------------
// ARCH-9 / TASK-1874 — a provider cannot re-point its siblings' context
// ---------------------------------------------------------------------------

/// ARCH-9 / TASK-1874: `refresh` and `working_directory` are private, so a
/// provider has no way to change what a sibling observes. This pins the
/// resulting behaviour: a provider reached transitively through
/// `get_or_provide` sees exactly the values the *caller* configured. Before
/// the fix both were `pub` on the `&mut Context` every provider receives, so
/// `ctx.refresh = false` or `ctx.working_directory = other` inside one
/// provider silently changed caching and path resolution for every provider
/// that ran after it in the same traversal.
#[test]
fn provider_cannot_change_the_context_its_siblings_observe() {
    use std::sync::Mutex;

    type Observed = Arc<Mutex<Vec<(bool, PathBuf)>>>;

    struct Inner {
        observed: Observed,
    }
    impl DataProvider for Inner {
        fn name(&self) -> &'static str {
            "inner"
        }
        fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            self.observed
                .lock()
                .unwrap()
                .push((ctx.is_refreshing(), ctx.working_directory().to_path_buf()));
            Ok(serde_json::Value::Null)
        }
    }

    struct Outer {
        observed: Observed,
        registry: Arc<Mutex<Option<Arc<DataRegistry>>>>,
    }
    impl DataProvider for Outer {
        fn name(&self) -> &'static str {
            "outer"
        }
        fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            self.observed
                .lock()
                .unwrap()
                .push((ctx.is_refreshing(), ctx.working_directory().to_path_buf()));
            // The only mutation surface a provider has is the cache, via
            // `get_or_provide`. There is no `ctx.refresh = …` to write here.
            let reg = self
                .registry
                .lock()
                .unwrap()
                .as_ref()
                .expect("registry wired")
                .clone();
            let _ = ctx.get_or_provide("inner", &reg)?;
            Ok(serde_json::Value::Null)
        }
    }

    let observed: Observed = Arc::new(Mutex::new(Vec::new()));
    let shared: Arc<Mutex<Option<Arc<DataRegistry>>>> = Arc::new(Mutex::new(None));
    let mut registry = DataRegistry::new();
    let _ = registry.register(
        "outer",
        Box::new(Outer {
            observed: Arc::clone(&observed),
            registry: Arc::clone(&shared),
        }),
    );
    let _ = registry.register(
        "inner",
        Box::new(Inner {
            observed: Arc::clone(&observed),
        }),
    );
    let registry = Arc::new(registry);
    *shared.lock().unwrap() = Some(Arc::clone(&registry));

    let cwd = PathBuf::from("/tmp/wave-1874");
    let mut ctx = Context::test_context(cwd.clone()).with_refresh();
    ctx.get_or_provide("outer", &registry).expect("outer");

    let seen = observed.lock().unwrap().clone();
    assert_eq!(seen.len(), 2, "both providers must have run");
    for (refreshing, dir) in seen {
        assert!(
            refreshing,
            "every provider observes the caller's refresh flag"
        );
        assert_eq!(dir, cwd, "every provider observes the caller's cwd");
    }
}

#[test]
fn context_accessors_report_the_constructed_values() {
    let cwd = Arc::new(PathBuf::from("/tmp/accessors"));
    let config = Arc::new(ops_core::config::Config::empty());
    let ctx = Context::from_cwd_arc(Arc::clone(&config), Arc::clone(&cwd));

    assert_eq!(ctx.working_directory(), cwd.as_path());
    assert!(!ctx.is_refreshing());
    // PERF-3 / TASK-0890: `from_cwd_arc` shares the allocation instead of
    // deep-cloning the inner PathBuf.
    assert!(Arc::ptr_eq(ctx.working_directory_arc(), &cwd));
    assert!(Arc::ptr_eq(ctx.config_arc(), &config));
    assert!(std::ptr::eq(ctx.config(), config.as_ref()));
    assert!(ctx.with_refresh().is_refreshing());
}

// ---------------------------------------------------------------------------
// TRAIT-4 / TASK-1879 — Debug impls
// ---------------------------------------------------------------------------

#[test]
fn data_registry_debug_names_its_providers_and_pending_audit_entries() {
    let mut registry = DataRegistry::new();
    let _ = registry.register("zeta", Box::new(StubProvider));
    let _ = registry.register("alpha", Box::new(StubProvider));
    let _ = registry.register("zeta", Box::new(StubProvider));

    let rendered = format!("{registry:?}");
    assert!(rendered.contains("DataRegistry"), "got: {rendered}");
    // Registration order, not sorted order.
    let zeta = rendered.find("\"zeta\"").expect("zeta named");
    let alpha = rendered.find("\"alpha\"").expect("alpha named");
    assert!(
        zeta < alpha,
        "providers print in registration order: {rendered}"
    );
    assert!(
        rendered.contains("duplicate_inserts"),
        "the pending audit trail must be visible: {rendered}"
    );
}

#[test]
fn context_debug_lists_keys_but_never_cached_values() {
    let mut registry = DataRegistry::new();
    let _ = registry.register("stub", Box::new(StubProvider));
    let mut ctx = Context::test_context(PathBuf::from("/tmp/debug-ctx"));
    ctx.get_or_provide("stub", &registry).expect("provide");

    let rendered = format!("{ctx:?}");
    for expected in [
        "Context",
        "/tmp/debug-ctx",
        "refresh",
        "cached_keys",
        "\"stub\"",
        "in_flight",
    ] {
        assert!(
            rendered.contains(expected),
            "Debug output must mention `{expected}`; got: {rendered}"
        );
    }
    assert!(
        !rendered.contains("value"),
        "cached provider *values* must never be rendered: {rendered}"
    );
}

// ---------------------------------------------------------------------------
// ARCH-9 / TASK-1868 — deterministic config_name collision resolution
// ---------------------------------------------------------------------------

struct NamedExtension(&'static str);
impl Extension for NamedExtension {
    fn name(&self) -> &'static str {
        self.0
    }
    fn register_commands(&self, _registry: &mut CommandRegistry) {}
}

/// Unsizing at a return position, so the test bodies need no `as` cast (the
/// workspace denies `clippy::as_conversions`).
fn boxed_named_extension(name: &'static str) -> Box<dyn Extension> {
    Box::new(NamedExtension(name))
}

/// ARCH-9 / TASK-1868: `EXTENSION_REGISTRY` publishes no ordering contract, so
/// the winner of a `config_name` collision used to be decided by whatever
/// order the linker emitted the slots in. `sort_compiled_extensions` imposes a
/// total order on `(config_name, Extension::name())`, so the same pairs
/// resolve to the same winner no matter what order they arrive in.
#[test]
fn sort_compiled_extensions_pins_the_collision_winner_regardless_of_input_order() {
    let build = |order: [&'static str; 2]| {
        let pairs: Vec<(&'static str, Box<dyn Extension>)> = order
            .into_iter()
            .map(|n| ("dup", boxed_named_extension(n)))
            .collect();
        sort_compiled_extensions(pairs)
            .into_iter()
            .map(|(cfg, ext)| (cfg, ext.name()))
            .collect::<Vec<_>>()
    };

    let forwards = build(["b-ext", "a-ext"]);
    let backwards = build(["a-ext", "b-ext"]);
    assert_eq!(forwards, backwards, "link order must not change the result");
    // Consumers collapse the sorted Vec last-write-wins, so the *last* entry
    // is the surviving extension; pinning the whole sequence pins the winner.
    assert_eq!(
        forwards,
        vec![("dup", "a-ext"), ("dup", "b-ext")],
        "ties on config_name break on Extension::name()"
    );
}

#[test]
fn sort_compiled_extensions_orders_distinct_config_names() {
    let pairs: Vec<(&'static str, Box<dyn Extension>)> = vec![
        ("zeta", boxed_named_extension("z")),
        ("alpha", boxed_named_extension("a")),
    ];
    let names: Vec<&str> = sort_compiled_extensions(pairs)
        .into_iter()
        .map(|(cfg, _)| cfg)
        .collect();
    assert_eq!(names, vec!["alpha", "zeta"]);
}

// ---------------------------------------------------------------------------
// TEST-5 / TASK-1877 — the factory arms and the rest of the untested surface
// ---------------------------------------------------------------------------

/// A real factory declines (returns `None`) when its prerequisites are not
/// met — wrong stack, a tool missing from `PATH`. These two model that with a
/// prerequisite a test can control, so both the construct and the decline
/// branch of the `factory:` arms are exercised.
fn factory_full_ext(
    _config: &ops_core::config::Config,
    root: &std::path::Path,
) -> Option<(&'static str, Box<dyn Extension>)> {
    if root.exists() {
        let ext: Box<dyn Extension> = Box::new(FactoryFullExt);
        Some(("factory-full", ext))
    } else {
        None
    }
}

fn factory_short_ext(
    _config: &ops_core::config::Config,
    root: &std::path::Path,
) -> Option<(&'static str, Box<dyn Extension>)> {
    if root.exists() {
        let ext: Box<dyn Extension> = Box::new(FactoryShortExt);
        Some(("factory-short", ext))
    } else {
        None
    }
}

struct FactoryFullExt;
impl_extension! {
    FactoryFullExt,
    name: "factory-full",
    description: "Full form with a factory arm",
    shortname: "ff",
    types: ExtensionType::DATASOURCE | ExtensionType::COMMAND,
    command_names: &["ff_cmd"],
    data_provider_name: Some("factory_full_data"),
    register_commands: |_self_cmd, registry| {
        registry.insert(
            "ff_cmd".into(),
            CommandSpec::Exec(ExecCommandSpec::new("echo", ["ff"])),
        );
    },
    register_data_providers: |_self_dp, registry| {
        let _ = registry.register("factory_full_data", Box::new(StubProvider));
    },
    factory: FACTORY_FULL_EXT = factory_full_ext,
}

struct FactoryShortExt;
impl_extension! {
    FactoryShortExt,
    name: "factory-short",
    description: "Short form with a factory arm",
    shortname: "fs",
    types: ExtensionType::DATASOURCE,
    data_provider_name: Some("factory_short_data"),
    register_data_providers: |_self_dp, registry| {
        let _ = registry.register("factory_short_data", Box::new(StubProvider));
    },
    factory: FACTORY_SHORT_EXT = factory_short_ext,
}

/// TEST-5 / TASK-1877: both `factory:` arms of `impl_extension!` — the crate's
/// entire compiled-in auto-discovery mechanism — are expanded above and walked
/// here. Nothing expanded them before, so a change to `ExtensionFactory`'s
/// signature or to `EXTENSION_REGISTRY`'s type compiled fine in
/// `cargo test -p ops-extension` and broke every downstream extension crate.
#[test]
fn both_factory_arms_register_into_the_extension_registry() {
    let config = ops_core::config::Config::empty();
    let root = std::path::Path::new(".");
    let found: Vec<(&'static str, &'static str)> = EXTENSION_REGISTRY
        .iter()
        .filter_map(|factory| factory(&config, root))
        .map(|(config_name, ext)| (config_name, ext.name()))
        .collect();

    for expected in [
        ("factory-full", "factory-full"),
        ("factory-short", "factory-short"),
    ] {
        assert!(
            found.contains(&expected),
            "EXTENSION_REGISTRY must contain a factory yielding {expected:?}; found: {found:?}"
        );
    }
}

/// A factory whose prerequisites are unmet must decline rather than construct.
/// `collect_compiled_extensions` relies on that to tell "compiled in but
/// inactive" apart from "never linked".
#[test]
fn factory_arms_decline_when_prerequisites_are_unmet() {
    let config = ops_core::config::Config::empty();
    let missing = std::path::Path::new("/nonexistent/wave-1877");
    assert!(factory_full_ext(&config, missing).is_none());
    assert!(factory_short_ext(&config, missing).is_none());
}

/// TEST-5 / TASK-1877 AC#2: `test_datasource_extension!` is the macro every
/// downstream extension crate uses for its registration tests. Invoking it
/// here means a syntax or path regression in it fails at its source instead of
/// in N downstream crates at once.
mod test_datasource_extension_macro {
    use super::{FactoryShortExt, StubProvider};

    crate::test_datasource_extension!(
        FactoryShortExt,
        name: "factory-short",
        data_provider: "factory_short_data"
    );

    // Silence the unused-import warning when the macro body changes shape.
    #[test]
    fn stub_provider_is_reachable() {
        assert_eq!(crate::DataProvider::name(&StubProvider), "stub");
    }
}

/// TRAIT-4 / TASK-0653: `CommandRegistry`'s hand-written `Clone` exists only to
/// differ from `derive(Clone)` — it copies the data but resets the audit trail,
/// so a clone does not replay warnings its original already reported. Nothing
/// pinned that until now (TEST-5 / TASK-1877 AC#3).
#[test]
fn command_registry_clone_copies_data_and_resets_the_audit_trail() {
    let id = CommandId::new("dup");
    let mut reg = CommandRegistry::new();
    assert!(
        reg.insert(
            id.clone(),
            CommandSpec::Exec(ExecCommandSpec::new("a", ["x"]))
        )
        .is_none(),
        "a fresh id has no previous spec"
    );
    let previous = reg
        .insert(
            id.clone(),
            CommandSpec::Exec(ExecCommandSpec::new("b", ["y"])),
        )
        .expect("re-inserting must return the previous spec");
    if let CommandSpec::Exec(e) = previous {
        assert_eq!(e.program, "a");
    } else {
        panic!("expected the Exec spec that was replaced");
    }

    let mut clone = reg.clone();
    assert_eq!(clone.len(), reg.len(), "the data must be copied");
    assert!(clone.contains_key(&id));
    assert!(
        clone.take_duplicate_inserts().is_empty(),
        "the clone must not inherit the original's audit trail"
    );
    assert_eq!(
        reg.take_duplicate_inserts(),
        vec![id],
        "the original keeps its own audit trail"
    );
}

#[test]
fn command_registry_ref_into_iter_yields_insertion_order() {
    let mut reg = CommandRegistry::new();
    for name in ["zeta", "alpha", "mu"] {
        let _ = reg.insert(
            CommandId::new(name),
            CommandSpec::Exec(ExecCommandSpec::new("echo", [name])),
        );
    }
    let observed: Vec<&str> = (&reg).into_iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(observed, vec!["zeta", "alpha", "mu"]);
}

#[test]
fn extension_info_new_defaults_every_optional_field() {
    let info = ExtensionInfo::new("solo");
    assert_eq!(info.name, "solo");
    assert_eq!(info.shortname, "solo", "shortname defaults to name");
    assert_eq!(info.description, "");
    assert_eq!(info.types, ExtensionType::empty());
    assert!(info.command_names.is_empty());
    assert!(info.data_provider_name.is_none());
    assert!(info.stack.is_none());
}

/// ARCH-9 / TASK-1128: the runner calls this when swapping in a new
/// `DataRegistry`, so callers cannot read values produced by the previous
/// registry's providers.
#[test]
fn clear_provider_results_drops_cached_values() {
    let mut registry = DataRegistry::new();
    let _ = registry.register("stub", Box::new(StubProvider));
    let mut ctx = test_context();
    ctx.get_or_provide("stub", &registry).expect("provide");
    assert!(ctx.cached("stub").is_some());

    ctx.clear_provider_results();
    assert!(ctx.cached("stub").is_none(), "the cache must be emptied");

    // The in-flight set is cleared too, so the key is immediately requestable
    // again rather than reporting a phantom cycle.
    ctx.get_or_provide("stub", &registry)
        .expect("the key must be requestable after a clear");
}

// ---------------------------------------------------------------------------
// TEST-5 / TASK-1877 AC#5 — the duckdb-feature surface
// ---------------------------------------------------------------------------

#[cfg(feature = "duckdb")]
mod duckdb_feature {
    use super::{test_context, DuckDbHandle};
    use std::sync::Arc;

    #[derive(Debug, PartialEq, Eq)]
    struct FakeDb(u32);

    /// TRAIT-9 / TASK-1227: the blanket impl supplies `as_any` for every
    /// `'static + Send + Sync` type, so an implementer cannot return a wrong
    /// reference and silently break the documented downcast.
    #[test]
    fn blanket_impl_downcasts_back_to_the_concrete_type() {
        let handle: Arc<dyn DuckDbHandle> = Arc::new(FakeDb(7));
        // The receiver must be reborrowed as `&dyn DuckDbHandle` first. See
        // the test below for why calling `as_any()` on the `Arc` does not
        // reach the inner value.
        let erased: &dyn DuckDbHandle = handle.as_ref();
        let recovered = erased
            .as_any()
            .downcast_ref::<FakeDb>()
            .expect("as_any must return self");
        assert_eq!(recovered, &FakeDb(7));
        assert!(
            erased.as_any().downcast_ref::<String>().is_none(),
            "a downcast to an unrelated type must not succeed"
        );
    }

    /// TEST-5 / TASK-1877: the blanket impl covers **every** `'static + Send +
    /// Sync` type — including `Arc<dyn DuckDbHandle>` itself. Method
    /// resolution on an `Arc` receiver therefore matches the blanket impl for
    /// the smart pointer before it ever derefs to the inner value, and
    /// `as_any()` returns the `Arc` erased rather than the handle. Pinned
    /// because the trait's own "Downcast contract" example reads
    /// `handle.as_any()` and callers copy it verbatim.
    ///
    /// SEC-38 / TASK-2018: this holds *because this module imports
    /// `DuckDbHandle`*. The blanket impl is only a method-resolution candidate
    /// where the trait is in scope, so a module that never names it sees
    /// `handle.as_any()` fall through the deref chain to the trait object's own
    /// method and downcast correctly — which is exactly how the misresolution
    /// stayed invisible in `ops_duckdb::downcast_duckdb`. Do not read this test
    /// as "an `Arc` receiver always fails"; read it as "an `Arc` receiver fails
    /// the moment anyone adds the import". The reborrow is the only shape that
    /// is correct in both modules.
    #[test]
    fn as_any_on_an_arc_receiver_erases_the_arc_not_the_handle() {
        let handle: Arc<dyn DuckDbHandle> = Arc::new(FakeDb(7));
        assert!(
            handle.as_any().downcast_ref::<FakeDb>().is_none(),
            "an Arc receiver does not reach the inner value"
        );
        assert!(
            handle
                .as_any()
                .downcast_ref::<Arc<dyn DuckDbHandle>>()
                .is_some(),
            "it erases the Arc itself instead"
        );
    }

    #[test]
    fn context_db_accessor_reports_the_attached_handle() {
        let mut ctx = test_context();
        assert!(ctx.db().is_none(), "a fresh context has no handle");

        let handle: Arc<dyn DuckDbHandle> = Arc::new(FakeDb(42));
        ctx.attach_db(Arc::clone(&handle));

        let stored = ctx.db().expect("handle attached");
        assert!(Arc::ptr_eq(stored, &handle), "the same Arc must come back");
        let erased: &dyn DuckDbHandle = stored.as_ref();
        assert_eq!(
            erased.as_any().downcast_ref::<FakeDb>(),
            Some(&FakeDb(42)),
            "the downcast contract survives the round trip"
        );
        assert!(
            format!("{ctx:?}").contains("db: true"),
            "Debug reports handle presence, not the handle itself"
        );
    }
}

// ---------------------------------------------------------------------------
// ERR-2 / TASK-1887 + ERR-9 / TASK-1889 — error identity and rendering
// ---------------------------------------------------------------------------

/// ERR-2 / TASK-1887: `computation_failed` used to fabricate a
/// `std::io::Error` as a container for its message, so a caller recovering an
/// I/O cause the normal way got a hit for an error that never touched a file
/// descriptor — and `ErrorKind::Other` gave it no way to tell the difference.
#[test]
fn computation_failed_has_no_fabricated_io_source() {
    use std::error::Error;

    let err = DataProviderError::computation_failed("cargo metadata returned no packages");
    assert!(
        err.source().is_none(),
        "a message-only failure has no cause to expose"
    );

    let mut current: Option<&(dyn Error + 'static)> = Some(&err);
    while let Some(e) = current {
        assert!(
            e.downcast_ref::<std::io::Error>().is_none(),
            "no link in the chain may claim to be a std::io::Error"
        );
        current = e.source();
    }

    // A genuine I/O failure still reaches operators through the chain the
    // crate does expose: `SharedError` renders it and forwards its source.
    let real = DataProviderError::computation_error(std::io::Error::other("disk full"));
    assert!(real.to_string().contains("disk full"));
    assert!(
        real.source().is_some(),
        "a wrapped source error is still exposed as a cause"
    );
}

/// ERR-9 / TASK-1889: the `{0:#}` self-interpolation on `ComputationFailed` and
/// `Serialization` is a deliberate, documented trade-off. Pin the rendering of
/// every display path the workspace actually uses so a future format-string
/// change cannot silently drop a root cause — nor reintroduce duplication on a
/// path that does not have it today.
#[test]
fn computation_failed_rendering_is_pinned_on_every_display_path() {
    let inner = anyhow::anyhow!("root cause").context("outer context");
    let e = DataProviderError::from(inner);

    // Path 1 — `{e}` / `to_string()`: the whole chain, flattened.
    assert_eq!(
        e.to_string(),
        "data computation failed: outer context: root cause"
    );
    // Path 2 — `{e:#}` in `tracing::warn!` (about's warm-up and enrichment
    // sites): identical, because `{0:#}` applies the flag unconditionally.
    assert_eq!(format!("{e:#}"), e.to_string());
    // Path 3 — `{:?}` through anyhow (create-review-tasks' fetch_review_targets,
    // providers::load_or_default). The chain reaches the operator; the cost of
    // keeping paths 1 and 2 whole is that it appears twice.
    let report = format!("{:?}", anyhow::Error::new(e));
    for layer in ["data computation failed", "outer context", "root cause"] {
        assert!(
            report.contains(layer),
            "anyhow's debug report must surface `{layer}`; got: {report}"
        );
    }
    assert!(
        report.contains("Caused by"),
        "anyhow must still walk the #[source] chain: {report}"
    );
}

/// The `Serialization` variant mirrors `ComputationFailed`; pin it too so the
/// two cannot drift apart unnoticed.
#[test]
fn serialization_rendering_flattens_its_chain() {
    let json_err: serde_json::Error = serde_json::from_str::<String>("not json").unwrap_err();
    let expected_root = json_err.to_string();
    let e = DataProviderError::from(json_err);
    let rendered = e.to_string();
    assert!(
        rendered.starts_with("data serialization error: "),
        "got: {rendered}"
    );
    assert!(rendered.contains(&expected_root), "got: {rendered}");
    assert_eq!(format!("{e:#}"), rendered);
}

// ---------------------------------------------------------------------------
// SEC-33 / TASK-2017 — provider dispatch is bounded in wall-clock time
// ---------------------------------------------------------------------------

/// A provider that burns wall-clock time in `steps` chunks, optionally polling
/// the context deadline between them, and records how many chunks it got
/// through so a test can tell "aborted early" from "ran to completion".
struct SlowProvider {
    name: &'static str,
    steps: usize,
    step: std::time::Duration,
    poll_deadline: bool,
    completed_steps: Arc<std::sync::atomic::AtomicUsize>,
}

impl SlowProvider {
    fn new(name: &'static str, poll_deadline: bool) -> Self {
        Self {
            name,
            steps: 20,
            step: std::time::Duration::from_millis(10),
            poll_deadline,
            completed_steps: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
}

impl DataProvider for SlowProvider {
    fn name(&self) -> &'static str {
        self.name
    }
    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        for _ in 0..self.steps {
            if self.poll_deadline {
                ctx.check_deadline()?;
            }
            std::thread::sleep(self.step);
            self.completed_steps
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        Ok(serde_json::json!({ "finished": true }))
    }
}

fn registry_with(name: &str, provider: Box<dyn DataProvider>) -> DataRegistry {
    let mut registry = DataRegistry::new();
    let _ = registry.register(name, provider);
    registry
}

/// The default budget must be an actual bound, not an unset option: a fresh
/// context carries one, which is what makes every provider bounded by
/// construction rather than by each provider remembering to opt in.
#[test]
fn a_fresh_context_carries_a_provider_budget() {
    let registry = registry_with("stub", Box::new(StubProvider));
    let mut ctx = test_context();
    assert!(
        ctx.deadline().is_none(),
        "no dispatch is in flight, so no deadline is installed yet"
    );
    let mut probe = DataRegistry::new();
    let _ = probe.register("probe", Box::new(DeadlineProbe));
    let seen = probe
        .provide("probe", &mut ctx)
        .expect("probe provider must succeed");
    assert_eq!(
        seen,
        serde_json::json!({ "deadline_installed": true }),
        "DataRegistry::provide must install a deadline for the dispatch"
    );
    assert!(
        ctx.deadline().is_none(),
        "the deadline must be cleared once the dispatch returns"
    );
    // The default budget must not be so tight that ordinary providers trip it.
    assert!(registry.provide("stub", &mut ctx).is_ok());
}

/// Reports whether a deadline was visible from inside `provide`.
struct DeadlineProbe;
impl DataProvider for DeadlineProbe {
    fn name(&self) -> &'static str {
        "probe"
    }
    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        Ok(serde_json::json!({ "deadline_installed": ctx.deadline().is_some() }))
    }
}

/// AC #2: a provider that runs past the budget without ever polling must be
/// reported as a failure naming it — never as the late value it produced.
#[test]
fn over_budget_provider_without_polling_is_reported_as_a_named_failure() {
    let slow = SlowProvider::new("slow", false);
    let registry = registry_with("slow", Box::new(slow));
    let mut ctx = test_context().with_provider_budget(Some(std::time::Duration::from_millis(20)));

    match registry.provide("slow", &mut ctx) {
        Err(DataProviderError::TimedOut { provider, budget }) => {
            assert_eq!(provider, "slow");
            assert_eq!(budget, std::time::Duration::from_millis(20));
        }
        other => panic!("expected TimedOut naming the provider, got {other:?}"),
    }
}

/// AC #1/#3: a provider that honours `check_deadline` stops early, so the
/// bound shortens the stall rather than merely labelling it afterwards.
#[test]
fn polling_provider_aborts_at_the_deadline_instead_of_running_to_completion() {
    let slow = SlowProvider::new("polling", true);
    let completed = Arc::clone(&slow.completed_steps);
    let total_steps = slow.steps;
    let registry = registry_with("polling", Box::new(slow));
    let mut ctx = test_context().with_provider_budget(Some(std::time::Duration::from_millis(20)));

    let err = registry
        .provide("polling", &mut ctx)
        .expect_err("the provider must not outlast its budget");
    assert!(
        matches!(err, DataProviderError::TimedOut { ref provider, .. } if provider == "polling"),
        "got {err:?}"
    );
    let done = completed.load(std::sync::atomic::Ordering::Relaxed);
    assert!(
        done < total_steps,
        "the provider ran all {total_steps} steps, so the deadline was not honoured"
    );
}

/// The budget bounds the traversal an operator asked for, not each level of
/// it: a provider composing others must not be able to multiply its budget by
/// nesting, and the failure keeps naming the provider that owns the budget.
#[test]
fn nested_dispatch_inherits_the_outermost_deadline() {
    /// Records the deadline seen on entry and again after sleeping, so the
    /// assertions live in the test body rather than inside a
    /// `Result`-returning `provide` (`clippy::panic_in_result_fn`).
    struct Composing {
        seen: Arc<std::sync::Mutex<Vec<Option<std::time::Instant>>>>,
    }
    impl DataProvider for Composing {
        fn name(&self) -> &'static str {
            "outer"
        }
        fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            self.seen
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(ctx.deadline());
            std::thread::sleep(std::time::Duration::from_millis(40));
            let result = ctx.check_deadline();
            self.seen
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(ctx.deadline());
            result?;
            Ok(serde_json::json!({ "finished": true }))
        }
    }

    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let registry = registry_with(
        "outer",
        Box::new(Composing {
            seen: Arc::clone(&seen),
        }),
    );
    let mut ctx = test_context().with_provider_budget(Some(std::time::Duration::from_millis(20)));
    match registry.provide("outer", &mut ctx) {
        Err(DataProviderError::TimedOut { provider, .. }) => assert_eq!(provider, "outer"),
        other => panic!("expected TimedOut naming the budget owner, got {other:?}"),
    }
    let seen = seen
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(seen.len(), 2, "provide must have run once");
    assert!(seen[0].is_some(), "the dispatch must install a deadline");
    assert_eq!(
        seen[0], seen[1],
        "a nested check must see the same deadline, not a fresh one"
    );
}

/// A dispatch that times out must not poison the context: the deadline is
/// cleared on the failure path, the way the in-flight marker is.
#[test]
fn a_timed_out_dispatch_leaves_the_context_usable() {
    let mut registry = DataRegistry::new();
    let _ = registry.register("slow", Box::new(SlowProvider::new("slow", true)));
    let _ = registry.register("stub", Box::new(StubProvider));
    let mut ctx = test_context().with_provider_budget(Some(std::time::Duration::from_millis(20)));

    assert!(registry.provide("slow", &mut ctx).is_err());
    assert!(
        ctx.deadline().is_none(),
        "a failed dispatch must not leave its deadline installed"
    );
    let v = registry
        .provide("stub", &mut ctx)
        .expect("a later provider must get its own budget, not the expired one");
    assert_eq!(v, serde_json::json!({"key": "value"}));
}

/// Opting out is explicit and honoured: callers that know a provider is
/// legitimately long-running (a full-workspace coverage run) can say so, and
/// nothing is silently converted into a failure.
#[test]
fn an_unbounded_context_does_not_time_a_provider_out() {
    let registry = registry_with("slow", Box::new(SlowProvider::new("slow", true)));
    let mut ctx = test_context().with_provider_budget(None);
    let v = registry
        .provide("slow", &mut ctx)
        .expect("an unbounded dispatch must run to completion");
    assert_eq!(v, serde_json::json!({ "finished": true }));
    assert!(ctx.deadline().is_none());
}

// ---------------------------------------------------------------------------
// ERR-1 / TASK-2024 — typed errors survive the DataProviderError chain
// ---------------------------------------------------------------------------

/// Stands in for `FindWorkspaceRootError`: a typed marker a consumer wants to
/// recover from a `DataProviderError` in order to classify the failure.
#[derive(Debug, PartialEq, Eq)]
enum TypedMarker {
    NotFound,
}

impl std::fmt::Display for TypedMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("no Cargo.toml found")
    }
}

impl std::error::Error for TypedMarker {}

/// Walks the chain the way real consumers do (`extensions-rust/about`'s
/// `is_manifest_missing`).
fn find_typed_marker<'a>(err: &'a (dyn std::error::Error + 'static)) -> Option<&'a TypedMarker> {
    let mut current = Some(err);
    while let Some(e) = current {
        if let Some(found) = e.downcast_ref::<TypedMarker>() {
            return Some(found);
        }
        current = e.source();
    }
    None
}

/// AC #2: the defect this pins is a *false negative* — before the fix the
/// wrapped error was skipped, so the marker was unreachable and every
/// consumer's classification silently degraded to "unknown failure".
#[test]
fn typed_error_is_reachable_through_data_provider_error_source_chain() {
    let err = DataProviderError::from(anyhow::Error::from(TypedMarker::NotFound));
    assert_eq!(
        find_typed_marker(&err),
        Some(&TypedMarker::NotFound),
        "the typed marker must be a link in the chain, not skipped"
    );
}

/// The same must hold for the `computation_error` constructor, which wraps a
/// concrete error directly rather than going through anyhow.
#[test]
fn typed_error_from_computation_error_is_reachable_too() {
    let err = DataProviderError::computation_error(TypedMarker::NotFound);
    assert_eq!(find_typed_marker(&err), Some(&TypedMarker::NotFound));
}

/// AC #4: `SharedError`'s alternate rendering walks `self.0.source()` after
/// printing `self.0`, which is independent of the `Error::source()` impl. The
/// fix must therefore leave `{:#}` byte-identical — no link printed twice and
/// none dropped.
#[test]
fn source_fix_leaves_the_alternate_display_unchanged() {
    #[derive(Debug)]
    struct Layered(std::io::Error);
    impl std::fmt::Display for Layered {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("outer context")
        }
    }
    impl std::error::Error for Layered {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    let shared = SharedError::new(Layered(std::io::Error::other("root cause")));
    assert_eq!(format!("{shared}"), "outer context");
    assert_eq!(format!("{shared:#}"), "outer context: root cause");

    let e = DataProviderError::ComputationFailed(shared);
    assert_eq!(
        e.to_string(),
        "data computation failed: outer context: root cause"
    );
}
