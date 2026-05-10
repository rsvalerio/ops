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

#[test]
fn data_registry_register_and_get() {
    let mut registry = DataRegistry::new();
    registry.register("stub", Box::new(StubProvider));
    assert!(registry.get("stub").is_some());
    assert!(registry.get("other").is_none());
}

/// SEC-31 / TASK-0350 + CL-5 / TASK-0756: registering two providers under
/// the same name must (1) be rejected first-write-wins and (2) record the
/// rejected name in the audit trail so the CLI wiring layer can emit a
/// single tracing::warn from one place. The earlier
/// `debug_assert!(false, …)` panic was retired because it forced every
/// in-extension duplicate to surface as a test panic instead of letting the
/// wiring layer's per-extension scratch registry aggregate the audit trail.
#[test]
fn data_registry_register_duplicate_records_audit_and_keeps_first() {
    let mut registry = DataRegistry::new();
    registry.register("stub", Box::new(StubProvider));
    registry.register("stub", Box::new(StubProvider));
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
    registry.register("stub", Box::new(StubProvider));
    let mut ctx = test_context();
    let value = registry.provide("stub", &mut ctx).expect("should succeed");
    assert_eq!(value, serde_json::json!({"key": "value"}));
}

/// ERR-1 / TASK-1170: a Context built with `with_refresh()` (or any caller
/// flipping `refresh = true`) must bypass the data_cache fast path so the
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
    registry.register(
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
    registry.register("stub", Box::new(StubProvider));
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
    registry.register(
        "alpha",
        Box::new(ChainProvider {
            name: "alpha",
            other: "beta",
            registry: Arc::clone(&shared),
        }),
    );
    registry.register(
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

#[test]
fn data_provider_error_computation_failed() {
    let err = DataProviderError::computation_failed("test computation error");
    assert!(err.to_string().contains("test computation error"));
    assert!(matches!(err, DataProviderError::ComputationFailed(_)));
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
    registry.register("zebra", Box::new(StubProvider));
    registry.register("alpha", Box::new(StubProvider));
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
    let shared = SharedError(Arc::new(inner));
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
    let shared = SharedError(Arc::new(outer));
    assert!(shared.source().is_some());
    assert!(shared.source().unwrap().to_string().contains("root cause"));
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
    registry.register("zzz", Box::new(SchemaProvider));
    registry.register("aaa", Box::new(StubProvider));
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
    assert!(ctx.refresh);
}

#[test]
fn context_default_refresh_is_false() {
    let ctx = test_context();
    assert!(!ctx.refresh);
}

#[test]
fn context_working_directory() {
    let ctx = Context::test_context(PathBuf::from("/tmp/test"));
    assert_eq!(*ctx.working_directory, PathBuf::from("/tmp/test"));
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
        registry.register("macro_data", Box::new(StubProvider));
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
        registry.register("short_data", Box::new(StubProvider));
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
    assert!(std::sync::Arc::ptr_eq(&orig.0, &copy.0));
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
    registry.register("identity", Box::new(AboutFieldProvider));
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

/// SEC-21 / TASK-1226: `DataRegistry::register` formats the runtime-
/// generated `provider_name` field via the `?` (Debug) formatter so an
/// extension that builds a provider name from external data containing
/// newlines or ANSI sequences cannot forge log entries through the
/// duplicate-insert breadcrumb. Pin the value-level escape directly,
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

/// mirroring `program_field_debug_escapes_control_characters`
/// (TASK-1127) and the broader workspace policy.
#[test]
fn provider_name_field_debug_escapes_control_characters() {
    let name = "stub\nFAKE_LOG\n\u{1b}[31m";
    let rendered = format!("{name:?}");
    assert!(!rendered.contains('\n'));
    assert!(!rendered.contains('\u{1b}'));
    assert!(rendered.contains("\\n"));
}
