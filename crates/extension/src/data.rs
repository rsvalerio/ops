//! Data provider system: DataProvider trait, DataRegistry, Context, DuckDbHandle.

use crate::error::DataProviderError;
use ops_core::config::Config;
use ops_core::project_identity::AboutFieldDef;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

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

    /// Returns the about-card fields this provider supports.
    ///
    /// Stack-specific `project_identity` providers override this to declare
    /// which fields appear in `ops about setup`. Default: empty (no fields).
    fn about_fields(&self) -> Vec<AboutFieldDef> {
        vec![]
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

    /// Returns about-card field declarations from the named provider.
    pub fn about_fields(&self, provider_name: &str) -> Vec<AboutFieldDef> {
        self.get(provider_name)
            .map(|p| p.about_fields())
            .unwrap_or_default()
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
