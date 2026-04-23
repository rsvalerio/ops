//! Cargo.toml extension: parses manifest files and provides structured data to other extensions.
//!
//! This extension serves as the canonical example of a **data-source-only extension**:
//! it registers no commands, only a data provider that other extensions can consume.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐    register_data_providers()    ┌──────────────────┐
//! │ CargoTomlExt    │ ─────────────────────────────▶  │ DataRegistry     │
//! │ (Extension)     │                                 │ ("cargo_toml")   │
//! └─────────────────┘                                 └──────────────────┘
//!                                                              │
//!                                                              │ provide()
//!                                                              ▼
//! ┌─────────────────┐    Context::get_or_provide()    ┌──────────────────┐
//! │ Other Extension │ ◀─────────────────────────────  │ CargoTomlProvider│
//! │ (Consumer)      │                                 │ (DataProvider)   │
//! └─────────────────┘                                 └──────────────────┘
//! ```
//!
//! # Usage
//!
//! ## As a Data Provider (from other extensions)
//!
//! ```ignore
//! use ops_extension::{Context, DataRegistry};
//! use ops_cargo_toml::CargoToml;
//!
//! fn my_extension_logic(ctx: &mut Context, registry: &DataRegistry) -> Result<(), anyhow::Error> {
//!     // Get cached Cargo.toml data
//!     let value = ctx.get_or_provide("cargo_toml", registry)?;
//!     let manifest: CargoToml = serde_json::from_value((*value).clone())?;
//!
//!     if let Some(pkg) = &manifest.package {
//!         println!("Package: {} v{}", pkg.name, pkg.version.as_str().unwrap_or(""));
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Workspace Inheritance
//!
//! Dependencies with `workspace = true` are automatically resolved from
//! `[workspace.dependencies]`. Enable this by calling [`CargoToml::resolve_inheritance`]
//! after parsing, or use the resolved data from the provider.
//!
//! # Implementation Notes for New Data Providers
//!
//! When creating a new data-source-only extension, follow this pattern:
//!
//! 1. **Create types** in `types.rs` with `#[derive(Deserialize)]`
//! 2. **Implement `DataProvider`** with a unique name (used as registry key)
//! 3. **Implement `Extension`** that registers only data providers (no commands)
//! 4. **Support configuration** via builder pattern if needed (e.g., custom paths)
//! 5. **Document the data contract** clearly so consumers know what to expect

mod inheritance;
#[cfg(test)]
mod tests;
mod types;

#[allow(unused_imports)]
pub use inheritance::InheritanceError;
#[allow(unused_imports)]
pub use types::{
    CargoToml, DepSpec, DetailedDepSpec, Package, PublishSpec, ReadmeSpec, Workspace,
    WorkspacePackage,
};

use anyhow::Context as _;
use ops_extension::{Context, DataProvider, DataProviderError, DataProviderSchema, ExtensionType};
use std::fs;
use std::path::{Path, PathBuf};

pub const NAME: &str = "cargo-toml";
#[allow(dead_code)]
pub const DESCRIPTION: &str = "Cargo.toml manifest parser and workspace data provider";
#[allow(dead_code)]
pub const SHORTNAME: &str = "cargo";
pub const DATA_PROVIDER_NAME: &str = "cargo_toml";

/// Extension that provides Cargo.toml parsing capabilities.
///
/// This extension registers no commands—it only provides data.
/// Use [`CargoTomlExtension::new`] for default behavior (auto-discover workspace root)
/// or [`CargoTomlExtension::with_root`] to specify a custom path.
///
/// # Example
///
/// ```
/// use ops_cargo_toml::CargoTomlExtension;
/// use std::path::PathBuf;
///
/// // Auto-discover from current directory
/// let ext = CargoTomlExtension::new();
///
/// // Or specify explicit root
/// let ext = CargoTomlExtension::with_root(PathBuf::from("/path/to/workspace"));
/// ```
pub struct CargoTomlExtension {
    root: Option<PathBuf>,
}

impl CargoTomlExtension {
    /// Create extension that auto-discovers workspace root from working directory.
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Create extension with an explicit workspace root path.
    #[allow(dead_code)]
    pub fn with_root(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }
}

impl Default for CargoTomlExtension {
    fn default() -> Self {
        Self::new()
    }
}

ops_extension::impl_extension! {
    CargoTomlExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Rust),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |this, registry| {
        let provider = match &this.root {
            Some(p) => CargoTomlProvider::with_root(p.clone()),
            None => CargoTomlProvider::new(),
        };
        registry.register(DATA_PROVIDER_NAME, Box::new(provider));
    },
    factory: CARGO_TOML_FACTORY = |_, _| {
        Some((NAME, Box::new(CargoTomlExtension::new())))
    },
}

/// Data provider that parses Cargo.toml and returns structured JSON.
///
/// This provider:
/// - Discovers workspace root by walking up from the working directory
/// - Parses Cargo.toml into [`CargoToml`] types
/// - Resolves workspace inheritance (`workspace = true`)
/// - Returns fresh data on each call (no internal caching)
///
/// Consumers should use `Context::get_or_provide()` for caching.
pub struct CargoTomlProvider {
    root: Option<PathBuf>,
}

impl CargoTomlProvider {
    /// Create provider that auto-discovers workspace root.
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Create provider with an explicit workspace root path.
    pub fn with_root(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }

    fn resolve_root(&self, working_dir: &Path) -> Result<PathBuf, anyhow::Error> {
        if let Some(root) = &self.root {
            return Ok(root.clone());
        }
        find_workspace_root(working_dir)
    }
}

impl Default for CargoTomlProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DataProvider for CargoTomlProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let root = self.resolve_root(&ctx.working_directory)?;
        let cargo_toml = root.join("Cargo.toml");

        let content = fs::read_to_string(&cargo_toml)
            .with_context(|| format!("reading {}", cargo_toml.display()))?;

        let mut manifest: CargoToml = toml::from_str(&content)
            .with_context(|| format!("parsing {}", cargo_toml.display()))?;

        manifest
            .resolve_inheritance()
            .context("resolving workspace inheritance")?;

        manifest.resolve_package_inheritance();

        Ok(serde_json::to_value(&manifest)?)
    }

    fn schema(&self) -> DataProviderSchema {
        use ops_extension::data_field;
        DataProviderSchema::new(
            "Cargo.toml manifest data (parsed from workspace root)",
            vec![
                data_field!(
                    "package",
                    "Option<Package>",
                    "Root package definition (None for virtual workspaces)"
                ),
                data_field!("workspace", "Option<Workspace>", "Workspace configuration"),
                data_field!(
                    "dependencies",
                    "Map<String, DepSpec>",
                    "Package dependencies"
                ),
                data_field!(
                    "dev-dependencies",
                    "Map<String, DepSpec>",
                    "Development dependencies"
                ),
                data_field!(
                    "build-dependencies",
                    "Map<String, DepSpec>",
                    "Build dependencies"
                ),
                data_field!("Package.name", "String", "Package name"),
                data_field!("Package.version", "String", "Package version"),
                data_field!("Package.edition", "String", "Rust edition (e.g., 2021)"),
                data_field!("Package.license", "Option<String>", "License identifier"),
                data_field!(
                    "Package.description",
                    "Option<String>",
                    "Package description"
                ),
                data_field!("Package.repository", "Option<String>", "Repository URL"),
                data_field!("Package.authors", "Vec<String>", "Package authors"),
                data_field!("Workspace.members", "Vec<String>", "Workspace member paths"),
                data_field!(
                    "Workspace.dependencies",
                    "Map<String, DepSpec>",
                    "Shared workspace dependencies"
                ),
                data_field!(
                    "DepSpec",
                    "String | DetailedDepSpec",
                    "Simple version string or detailed spec with features"
                ),
            ],
        )
    }
}

/// Find the workspace root by walking up from `start` looking for Cargo.toml.
///
/// Stops at the first directory containing Cargo.toml.
pub fn find_workspace_root(start: &Path) -> Result<PathBuf, anyhow::Error> {
    let mut current = start;

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            return Ok(current.to_path_buf());
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => {
                anyhow::bail!(
                    "no Cargo.toml found in {} or any parent directory",
                    start.display()
                );
            }
        }
    }
}
