use super::*;
use ops_core::config::{CommandSpec, ExecCommandSpec};
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

/// SEC-31 / TASK-0350: registering two providers under the same name must
/// surface the collision rather than silently swap a trusted built-in for
/// whatever extension happens to load second. In debug builds the
/// `debug_assert!` panics; the production behaviour (warn + keep first) is
/// covered indirectly by every `register_*` site running in release builds
/// without panicking.
#[test]
#[should_panic(expected = "duplicate data provider registration")]
fn data_registry_register_duplicate_panics_in_debug() {
    let mut registry = DataRegistry::new();
    registry.register("stub", Box::new(StubProvider));
    registry.register("stub", Box::new(StubProvider));
}

#[test]
fn data_registry_provide_returns_value() {
    let mut registry = DataRegistry::new();
    registry.register("stub", Box::new(StubProvider));
    let mut ctx = test_context();
    let value = registry.provide("stub", &mut ctx).expect("should succeed");
    assert_eq!(value, serde_json::json!({"key": "value"}));
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
    assert_eq!(ctx.working_directory, PathBuf::from("/tmp/test"));
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
