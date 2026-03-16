//! Extension trait and registries: CommandRegistry, DataRegistry, Context.

use indexmap::IndexMap;
use ops_core::config::{CommandId, CommandSpec, Config};
use ops_core::stack::Stack;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Cloneable wrapper for error sources, preserving the full error chain.
///
/// EFF-002: `Arc` enables `Clone` on `DataProviderError` without discarding the
/// original error's cause chain and Display output.
#[derive(Debug, Clone)]
pub struct SharedError(Arc<dyn std::error::Error + Send + Sync>);

impl std::fmt::Display for SharedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for SharedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl From<anyhow::Error> for SharedError {
    fn from(err: anyhow::Error) -> Self {
        // anyhow::Error doesn't implement std::error::Error, so convert via
        // into_inner() to extract the boxed source, falling back to an io::Error wrapper.
        let boxed: Box<dyn std::error::Error + Send + Sync> = err.into();
        Self(Arc::from(boxed))
    }
}

impl From<serde_json::Error> for SharedError {
    fn from(err: serde_json::Error) -> Self {
        Self(Arc::new(err))
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct ExtensionType: u8 {
        const DATASOURCE = 0b01;
        const COMMAND    = 0b10;
    }
}

impl ExtensionType {
    pub fn is_datasource(self) -> bool {
        self.contains(Self::DATASOURCE)
    }

    pub fn is_command(self) -> bool {
        self.contains(Self::COMMAND)
    }
}

pub struct ExtensionInfo {
    pub name: &'static str,
    pub shortname: &'static str,
    pub description: &'static str,
    pub types: ExtensionType,
    pub command_names: &'static [&'static str],
    pub data_provider_name: Option<&'static str>,
}

/// Registry of command ID → CommandSpec (from config + extensions).
///
/// An `IndexMap` preserving insertion order, mapping command names to their
/// specifications. Commands are registered by:
/// 1. Config file (`[commands.*]` sections)
/// 2. Extensions via [`Extension::register_commands`]
///
/// Config-defined commands take precedence over extension commands when
/// merged into the `CommandRunner`.
pub type CommandRegistry = IndexMap<CommandId, CommandSpec>;

/// Describes a field provided by a data provider.
#[derive(Debug, Clone)]
pub struct DataField {
    pub name: &'static str,
    pub type_name: &'static str,
    pub description: &'static str,
}

/// Schema for a data provider, describing what data it provides.
#[derive(Debug, Clone, Default)]
pub struct DataProviderSchema {
    pub description: &'static str,
    pub fields: Vec<DataField>,
}

/// Error type for data provider operations.
///
/// EFF-002: Uses `SharedError` (Arc-wrapped) for `ComputationFailed` and
/// `Serialization` variants to preserve the full error chain while keeping
/// `Clone`. The `#[source]` attribute enables `Error::source()` traversal.
#[derive(Debug, Clone, thiserror::Error)]
pub enum DataProviderError {
    #[error("data provider not found: {0}")]
    NotFound(String),
    #[error("data computation failed: {0}")]
    ComputationFailed(#[source] SharedError),
    #[error("data serialization error: {0}")]
    Serialization(#[source] SharedError),
}

impl DataProviderError {
    pub fn not_found(name: &str) -> Self {
        Self::NotFound(name.to_string())
    }

    /// Create a computation failure from a string message.
    pub fn computation_failed(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        Self::ComputationFailed(SharedError(Arc::new(std::io::Error::other(msg))))
    }

    /// Create a computation failure from a source error, preserving the error chain.
    pub fn computation_error(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::ComputationFailed(SharedError(Arc::new(err)))
    }
}

impl From<anyhow::Error> for DataProviderError {
    fn from(err: anyhow::Error) -> Self {
        Self::ComputationFailed(SharedError::from(err))
    }
}

impl From<serde_json::Error> for DataProviderError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(SharedError::from(err))
    }
}

/// Trait for data providers that supply JSON data to extensions.
///
/// Data providers are registered by extensions and can be queried by name.
/// The context provides caching to avoid redundant computation.
///
/// # Example
///
/// ```ignore
/// struct MetadataProvider;
///
/// impl DataProvider for MetadataProvider {
///     fn name(&self) -> &'static str { "metadata" }
///     fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
///         // Load or compute data, possibly using ctx.db
///         Ok(serde_json::json!({"version": "1.0"}))
///     }
/// }
/// ```
pub trait DataProvider: Send + Sync {
    /// Returns the unique name of this data provider.
    ///
    /// This name is used to register and query the provider via `DataRegistry`.
    fn name(&self) -> &'static str;

    /// Provides data, potentially using context for caching or configuration.
    ///
    /// Implementations may:
    /// - Use `ctx.db` to query an attached database handle
    /// - Use `ctx.config` to access configuration
    /// - Run external commands or read files
    ///
    /// The result is cached by `Context::get_or_provide` for subsequent calls.
    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError>;

    /// Returns a schema describing what data this provider exposes.
    ///
    /// Used by `cargo ops data info <name>` to show documentation.
    /// Default implementation returns an empty schema.
    fn schema(&self) -> DataProviderSchema {
        DataProviderSchema::default()
    }
}

/// Registry of provider name → DataProvider.
pub struct DataRegistry {
    providers: HashMap<String, Box<dyn DataProvider>>,
}

impl DataRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: impl Into<String>, provider: Box<dyn DataProvider>) {
        self.providers.insert(name.into(), provider);
    }

    pub fn get(&self, name: &str) -> Option<&dyn DataProvider> {
        self.providers.get(name).map(|b| b.as_ref())
    }

    /// Returns sorted list of registered provider names.
    pub fn provider_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.providers.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Returns schemas for all providers that have non-empty descriptions.
    pub fn schemas(&self) -> Vec<(&str, DataProviderSchema)> {
        let mut result: Vec<_> = self
            .providers
            .iter()
            .map(|(name, p)| (name.as_str(), p.schema()))
            .collect();
        result.sort_by_key(|(name, _)| *name);
        result
    }

    pub fn provide(
        &self,
        name: &str,
        ctx: &mut Context,
    ) -> Result<serde_json::Value, DataProviderError> {
        self.providers
            .get(name)
            .ok_or_else(|| DataProviderError::not_found(name))?
            .provide(ctx)
    }
}

impl Default for DataRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Erasure trait for the DuckDb handle so that extension.rs does not depend on duckdb types.
#[cfg(feature = "duckdb")]
pub trait DuckDbHandle: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
}

pub struct Context {
    pub config: Arc<Config>,
    pub data_cache: HashMap<String, Arc<serde_json::Value>>,
    pub working_directory: PathBuf,
    /// When true, data providers should re-collect data instead of using cached/persisted results.
    pub refresh: bool,
    #[cfg(feature = "duckdb")]
    pub db: Option<Arc<dyn DuckDbHandle>>,
}

impl Context {
    pub fn new(config: Arc<Config>, working_directory: PathBuf) -> Self {
        Self {
            config,
            data_cache: HashMap::new(),
            working_directory,
            refresh: false,
            #[cfg(feature = "duckdb")]
            db: None,
        }
    }

    /// Create a context for testing with default config.
    #[cfg(any(test, feature = "test-support"))]
    pub fn test_context(working_directory: PathBuf) -> Self {
        Self::new(Arc::new(Config::default()), working_directory)
    }

    /// Create a context with refresh mode enabled (forces data re-collection).
    pub fn with_refresh(mut self) -> Self {
        self.refresh = true;
        self
    }

    /// Get cached value or compute via provider and cache.
    pub fn get_or_provide(
        &mut self,
        key: &str,
        registry: &DataRegistry,
    ) -> Result<Arc<serde_json::Value>, DataProviderError> {
        if let Some(v) = self.data_cache.get(key) {
            return Ok(Arc::clone(v));
        }
        let v = registry.provide(key, self)?;
        let v = Arc::new(v);
        self.data_cache.insert(key.to_string(), Arc::clone(&v));
        Ok(v)
    }
}

/// Extension: registers commands and/or data providers.
///
/// Extensions are the primary mechanism for adding functionality to ops.
/// They can register:
/// - **Commands**: New named commands available via `cargo ops <name>`
/// - **Data providers**: Named data sources queryable by other extensions
///
/// # Lifecycle
///
/// 1. Extensions are instantiated by `builtin_extensions()` based on config
/// 2. `register_commands()` is called to add commands to the registry
/// 3. `register_data_providers()` is called to add data providers
/// 4. The registries are attached to the `CommandRunner`
///
/// # Example
///
/// ```ignore
/// struct MyExtension;
///
/// impl Extension for MyExtension {
///     fn name(&self) -> &'static str { "my-ext" }
///
///     fn register_commands(&self, registry: &mut CommandRegistry) {
///         registry.insert("my-cmd".into(), CommandSpec::Exec(...));
///     }
///
///     fn register_data_providers(&self, registry: &mut DataRegistry) {
///         registry.register("my-data", Box::new(MyDataProvider));
///     }
/// }
/// ```
pub trait Extension: Send + Sync {
    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str {
        ""
    }

    fn shortname(&self) -> &'static str {
        self.name()
    }

    fn types(&self) -> ExtensionType {
        ExtensionType::empty()
    }

    fn command_names(&self) -> &'static [&'static str] {
        &[]
    }

    fn data_provider_name(&self) -> Option<&'static str> {
        None
    }

    fn stack(&self) -> Option<Stack> {
        None
    }

    fn info(&self) -> ExtensionInfo {
        ExtensionInfo {
            name: self.name(),
            shortname: self.shortname(),
            description: self.description(),
            types: self.types(),
            command_names: self.command_names(),
            data_provider_name: self.data_provider_name(),
        }
    }

    fn register_commands(&self, registry: &mut CommandRegistry);

    fn register_data_providers(&self, _registry: &mut DataRegistry) {}
}

/// Macro to reduce boilerplate when implementing the Extension trait.
///
/// DUP-002: Single variant with optional `command_names` arm.
///
/// Generates the simple accessor methods (name, description, shortname, types,
/// data_provider_name) from the provided constants, and accepts custom blocks
/// for register_commands and register_data_providers.
///
/// # Example
///
/// ```ignore
/// impl_extension! {
///     MyExtension,
///     name: NAME,
///     description: DESCRIPTION,
///     shortname: SHORTNAME,
///     types: ExtensionType::DATASOURCE,
///     data_provider_name: Some(DATA_PROVIDER_NAME),
///     register_commands: |_self, _registry| {},
///     register_data_providers: |_self, registry| {
///         registry.register(DATA_PROVIDER_NAME, Box::new(MyProvider));
///     },
/// }
/// ```
#[macro_export]
macro_rules! impl_extension {
    // Internal rule: shared accessor methods (DUP-036 fix)
    (@accessors $struct:ty, $name:expr, $desc:expr, $short:expr, $types:expr, $dp:expr $(, command_names: $cn:expr)?) => {
        fn name(&self) -> &'static str {
            $name
        }
        fn description(&self) -> &'static str {
            $desc
        }
        fn shortname(&self) -> &'static str {
            $short
        }
        fn types(&self) -> $crate::ExtensionType {
            $types
        }
        $(
            fn command_names(&self) -> &'static [&'static str] {
                $cn
            }
        )?
        fn data_provider_name(&self) -> Option<&'static str> {
            $dp
        }
    };

    // Full form with register_commands
    (
        $struct:ty,
        name: $name:expr,
        description: $desc:expr,
        shortname: $short:expr,
        types: $types:expr,
        $(command_names: $cn:expr,)?
        data_provider_name: $dp:expr,
        register_commands: |$self_cmd:ident, $reg_cmd:ident| $cmd_body:block,
        register_data_providers: |$self_dp:ident, $reg_dp:ident| $dp_body:block $(,)?
    ) => {
        impl $crate::Extension for $struct {
            $crate::impl_extension!(@accessors $struct, $name, $desc, $short, $types, $dp $(, command_names: $cn)?);
            fn register_commands(&self, registry: &mut $crate::CommandRegistry) {
                let $self_cmd = self;
                let $reg_cmd = registry;
                $cmd_body
            }
            fn register_data_providers(&self, registry: &mut $crate::DataRegistry) {
                let $self_dp = self;
                let $reg_dp = registry;
                $dp_body
            }
        }
    };

    // Short form without register_commands (generates no-op)
    (
        $struct:ty,
        name: $name:expr,
        description: $desc:expr,
        shortname: $short:expr,
        types: $types:expr,
        $(command_names: $cn:expr,)?
        data_provider_name: $dp:expr,
        register_data_providers: |$self_dp:ident, $reg_dp:ident| $dp_body:block $(,)?
    ) => {
        impl $crate::Extension for $struct {
            $crate::impl_extension!(@accessors $struct, $name, $desc, $short, $types, $dp $(, command_names: $cn)?);
            fn register_commands(&self, _registry: &mut $crate::CommandRegistry) {}
            fn register_data_providers(&self, registry: &mut $crate::DataRegistry) {
                let $self_dp = self;
                let $reg_dp = registry;
                $dp_body
            }
        }
    };
}

/// Shorthand macro for constructing a [`DataField`].
///
/// Reduces verbose struct initialization from 5 lines to 1.
///
/// # Example
///
/// ```ignore
/// use ops_extension::data_field;
///
/// let fields = vec![
///     data_field!("name", "str", "Package name"),
///     data_field!("version", "str", "Package version string"),
/// ];
/// ```
#[macro_export]
macro_rules! data_field {
    ($name:expr, $type_name:expr, $description:expr) => {
        $crate::DataField {
            name: $name,
            type_name: $type_name,
            description: $description,
        }
    };
}

/// Macro to generate standard extension registration tests for datasource extensions.
///
/// Generates two tests:
/// - `extension_name`: Verifies the extension returns the expected name
/// - `extension_registers_data_provider`: Verifies the extension registers a data provider
///
/// # Example
///
/// ```ignore
/// ops_extension::test_datasource_extension!(
///     MetadataExtension,
///     name: "metadata",
///     data_provider: "metadata"
/// );
/// ```
#[macro_export]
macro_rules! test_datasource_extension {
    ($ext:expr, name: $name:expr, data_provider: $dp:expr) => {
        #[test]
        fn extension_name() {
            assert_eq!($crate::Extension::name(&$ext), $name);
        }

        #[test]
        fn extension_registers_data_provider() {
            let mut registry = $crate::DataRegistry::new();
            $crate::Extension::register_data_providers(&$ext, &mut registry);
            assert!(registry.get($dp).is_some());
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::config::ExecCommandSpec;

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
        assert!(ctx.data_cache.contains_key("stub"));
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
                "ext_cmd".to_string(),
                CommandSpec::Exec(ExecCommandSpec {
                    program: "echo".into(),
                    args: vec!["from_ext".into()],
                    ..Default::default()
                }),
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
        // ComputationFailed has #[source] SharedError, so source() should be Some
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
        // SchemaProvider has fields
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

    #[test]
    fn extension_default_description_is_empty() {
        let ext = TestExtension;
        assert_eq!(ext.description(), "");
    }

    #[test]
    fn extension_default_shortname_equals_name() {
        let ext = TestExtension;
        assert_eq!(ext.shortname(), ext.name());
    }

    #[test]
    fn extension_default_types_is_empty() {
        let ext = TestExtension;
        assert_eq!(ext.types(), ExtensionType::empty());
    }

    #[test]
    fn extension_default_command_names_is_empty() {
        let ext = TestExtension;
        assert!(ext.command_names().is_empty());
    }

    #[test]
    fn extension_default_data_provider_name_is_none() {
        let ext = TestExtension;
        assert!(ext.data_provider_name().is_none());
    }

    #[test]
    fn extension_default_stack_is_none() {
        let ext = TestExtension;
        assert!(ext.stack().is_none());
    }

    #[test]
    fn extension_info_aggregates_defaults() {
        let ext = TestExtension;
        let info = ext.info();
        assert_eq!(info.name, "test_ext");
        assert_eq!(info.shortname, "test_ext");
        assert_eq!(info.description, "");
        assert_eq!(info.types, ExtensionType::empty());
        assert!(info.command_names.is_empty());
        assert!(info.data_provider_name.is_none());
    }

    #[test]
    fn extension_default_register_data_providers_is_noop() {
        let ext = TestExtension;
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
                "cmd1".to_string(),
                CommandSpec::Exec(ExecCommandSpec {
                    program: "echo".into(),
                    args: vec!["macro".into()],
                    ..Default::default()
                }),
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

        // Short form generates no-op register_commands
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
        let err = DataProviderError::computation_failed("clone me");
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }

    // --- DataRegistry::default ---

    #[test]
    fn data_registry_default() {
        let registry = DataRegistry::default();
        assert!(registry.provider_names().is_empty());
    }
}
