//! Extension trait and registries: CommandRegistry, DataRegistry, Context.

use crate::config::{CommandId, CommandSpec, Config};
use crate::stack::Stack;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct ExtensionType: u8 {
        const DATASOURCE = 0b01;
        const COMMAND    = 0b10;
    }
}

#[allow(dead_code)]
impl ExtensionType {
    pub fn is_datasource(self) -> bool {
        self.contains(Self::DATASOURCE)
    }

    pub fn is_command(self) -> bool {
        self.contains(Self::COMMAND)
    }
}

#[allow(dead_code)]
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
#[derive(Debug, Clone, thiserror::Error)]
#[allow(dead_code)]
pub enum DataProviderError {
    #[error("data provider not found: {0}")]
    NotFound(String),
    #[error("data computation failed: {0}")]
    ComputationFailed(String),
    #[error("data serialization error: {0}")]
    Serialization(String),
}

impl DataProviderError {
    pub fn not_found(name: &str) -> Self {
        Self::NotFound(name.to_string())
    }

    #[allow(dead_code)]
    pub fn computation_failed(msg: impl Into<String>) -> Self {
        Self::ComputationFailed(msg.into())
    }
}

impl From<anyhow::Error> for DataProviderError {
    fn from(err: anyhow::Error) -> Self {
        Self::ComputationFailed(err.to_string())
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
#[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn register(&mut self, name: impl Into<String>, provider: Box<dyn DataProvider>) {
        self.providers.insert(name.into(), provider);
    }

    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<&dyn DataProvider> {
        self.providers.get(name).map(|b| b.as_ref())
    }

    /// Returns sorted list of registered provider names.
    #[allow(dead_code)]
    pub fn provider_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.providers.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Returns schemas for all providers that have non-empty descriptions.
    #[allow(dead_code)]
    pub fn schemas(&self) -> Vec<(&str, DataProviderSchema)> {
        let mut result: Vec<_> = self
            .providers
            .iter()
            .map(|(name, p)| (name.as_str(), p.schema()))
            .collect();
        result.sort_by_key(|(name, _)| *name);
        result
    }

    #[allow(dead_code)]
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

/// Erasure trait for the OpsDb handle so that extension.rs does not depend on ops_db types.
#[cfg(feature = "stack-rust")]
pub trait OpsDbHandle: Send + Sync {
    /// For downcast to concrete `OpsDb` in extensions that need it.
    fn as_any(&self) -> &dyn std::any::Any;
}

#[allow(dead_code)]
pub struct Context {
    pub config: Arc<Config>,
    pub data_cache: HashMap<String, Arc<serde_json::Value>>,
    pub working_directory: PathBuf,
    #[cfg(feature = "stack-rust")]
    pub db: Option<Arc<dyn OpsDbHandle>>,
}

#[allow(dead_code)]
impl Context {
    pub fn new(config: Arc<Config>, working_directory: PathBuf) -> Self {
        Self {
            config,
            data_cache: HashMap::new(),
            working_directory,
            #[cfg(feature = "stack-rust")]
            db: None,
        }
    }

    /// Create a context for testing with default config.
    #[cfg(test)]
    pub fn test_context(working_directory: PathBuf) -> Self {
        Self::new(Arc::new(Config::default()), working_directory)
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
/// Extensions are the primary mechanism for adding functionality to cargo-ops.
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
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

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
                CommandSpec::Exec(crate::config::ExecCommandSpec {
                    program: "echo".into(),
                    args: vec!["from_ext".into()],
                    env: HashMap::new(),
                    cwd: None,
                    timeout_secs: None,
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
}
