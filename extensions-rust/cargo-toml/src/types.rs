//! Pure type definitions for Cargo.toml parsing.
//!
//! This module provides strongly-typed structures that map directly to Cargo.toml sections.
//! All types derive `Deserialize` for parsing and implement `Clone` for caching.
//! Workspace inheritance resolution lives in [`crate::inheritance`].

use ops_core::serde_defaults;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Root Cargo.toml structure.
///
/// Represents the complete manifest with all sections. Use [`CargoToml::parse`]
/// to parse from TOML source, or access via the `cargo_toml` data provider.
///
/// # Example
///
/// ```
/// use ops_cargo_toml::CargoToml;
///
/// let toml_content = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n";
/// let manifest = CargoToml::parse(toml_content).unwrap();
/// if let Some(pkg) = &manifest.package {
///     println!("Package: {} v{}", pkg.name, pkg.version.as_str().unwrap_or(""));
/// }
/// ```
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct CargoToml {
    /// The `[package]` section. Present for package manifests, absent for virtual workspaces.
    pub package: Option<Package>,

    /// The `[workspace]` section. Present for workspace roots.
    pub workspace: Option<Workspace>,

    /// Normal dependencies from `[dependencies]`.
    #[serde(default)]
    pub dependencies: BTreeMap<String, DepSpec>,

    /// Dev dependencies from `[dev-dependencies]`.
    #[serde(default, alias = "dev-dependencies")]
    pub dev_dependencies: BTreeMap<String, DepSpec>,

    /// Build dependencies from `[build-dependencies]`.
    #[serde(default, alias = "build-dependencies")]
    pub build_dependencies: BTreeMap<String, DepSpec>,

    /// Feature definitions from `[features]`.
    #[serde(default)]
    pub features: BTreeMap<String, Vec<String>>,
}

#[allow(dead_code)]
impl CargoToml {
    /// Parse Cargo.toml content from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML is malformed or contains unexpected types.
    pub fn parse(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// Returns `true` if this is a virtual workspace (no root package).
    pub fn is_virtual_workspace(&self) -> bool {
        self.package.is_none() && self.workspace.is_some()
    }

    /// Returns `true` if this manifest defines a workspace.
    pub fn is_workspace(&self) -> bool {
        self.workspace.is_some()
    }

    /// Returns the package name if present.
    pub fn package_name(&self) -> Option<&str> {
        self.package.as_ref().map(|p| p.name.as_str())
    }

    /// Returns the package version if present and resolved.
    pub fn package_version(&self) -> Option<&str> {
        self.package.as_ref().and_then(|p| p.version.as_str())
    }

    /// Returns workspace members if defined.
    pub fn workspace_members(&self) -> Option<&[String]> {
        self.workspace.as_ref().map(|w| w.members.as_slice())
    }
}

/// The `[package]` section of Cargo.toml.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Package {
    /// The crate name.
    pub name: String,

    /// The crate version (semver, can be inherited from workspace).
    #[serde(default)]
    pub version: InheritableString,

    /// The Rust edition (e.g., "2021", can be inherited from workspace).
    #[serde(default)]
    pub edition: InheritableString,

    /// Minimum supported Rust version (can be inherited from workspace).
    #[serde(default, alias = "rust-version")]
    pub rust_version: InheritableString,

    /// List of authors (can be inherited from workspace).
    #[serde(default)]
    pub authors: InheritableVec,

    /// Crate description (can be inherited from workspace).
    #[serde(default)]
    pub description: InheritableString,

    /// Documentation URL (can be inherited from workspace).
    #[serde(default)]
    pub documentation: InheritableString,

    /// README file path (can be inherited from workspace).
    pub readme: Option<ReadmeSpec>,

    /// Homepage URL (can be inherited from workspace).
    #[serde(default)]
    pub homepage: InheritableString,

    /// Repository URL (can be inherited from workspace).
    #[serde(default)]
    pub repository: InheritableString,

    /// License identifier (e.g., "MIT OR Apache-2.0", can be inherited from workspace).
    #[serde(default)]
    pub license: InheritableString,

    /// Path to license file (can be inherited from workspace).
    #[serde(alias = "license-file")]
    pub license_file: Option<InheritableString>,

    /// Keywords for crates.io (can be inherited from workspace).
    #[serde(default)]
    pub keywords: InheritableVec,

    /// Categories for crates.io (can be inherited from workspace).
    #[serde(default)]
    pub categories: InheritableVec,

    /// Path to the main source file.
    #[serde(alias = "default-run")]
    pub default_run: Option<String>,

    /// Whether to publish to crates.io.
    #[serde(default)]
    pub publish: PublishSpec,
}

/// A field that can be inherited from workspace.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum InheritableField<T> {
    /// Direct value.
    Value(T),
    /// Inherited from workspace: `field = { workspace = true }`
    Inherited { workspace: bool },
}

impl<T> InheritableField<T> {
    /// Returns the value if this is a direct value, otherwise None.
    pub fn value(&self) -> Option<&T> {
        match self {
            InheritableField::Value(v) => Some(v),
            InheritableField::Inherited { .. } => None,
        }
    }
}

impl InheritableField<String> {
    /// Returns the string value as &str if present.
    pub fn as_str(&self) -> Option<&str> {
        self.value().map(|s| s.as_str())
    }
}

impl<T: Default> Default for InheritableField<T> {
    fn default() -> Self {
        InheritableField::Value(T::default())
    }
}

/// A vec field that can be inherited from workspace.
pub type InheritableVec = InheritableField<Vec<String>>;

/// A string field that can be inherited from workspace.
pub type InheritableString = InheritableField<String>;

/// README specification: can be a boolean, string, table, or workspace-inherited.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ReadmeSpec {
    /// `readme = true` or `readme = false`
    Bool(bool),
    /// `readme = "README.md"`
    Path(String),
    /// `readme = { workspace = true }`
    Inherited { workspace: bool },
    /// `readme = { file = "...", ... }`
    Table { file: String },
}

/// Publish specification: can be a boolean, list of registries, or workspace-inherited.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(untagged)]
pub enum PublishSpec {
    /// `publish = false`
    Bool(bool),
    /// `publish = ["my-registry"]`
    Registries(Vec<String>),
    /// `publish = { workspace = true }`
    Inherited { workspace: bool },
    /// No publish field (defaults to true).
    #[default]
    None,
}

#[allow(dead_code)]
impl PublishSpec {
    /// Returns `true` if publishing is allowed (to any registry).
    pub fn is_publishable(&self) -> bool {
        match self {
            PublishSpec::Bool(b) => *b,
            PublishSpec::Registries(v) => !v.is_empty(),
            // Conservatively report unresolved-inherited as publishable; the
            // resolved value (after `resolve_package_inheritance`) is what
            // callers should rely on.
            PublishSpec::Inherited { .. } | PublishSpec::None => true,
        }
    }
}

/// The `[workspace]` section of Cargo.toml.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Workspace {
    /// Workspace member paths (globs supported by Cargo).
    #[serde(default)]
    pub members: Vec<String>,

    /// Dependency resolver version ("1" or "2").
    pub resolver: Option<String>,

    /// Shared workspace dependencies from `[workspace.dependencies]`.
    #[serde(default)]
    pub dependencies: BTreeMap<String, DepSpec>,

    /// Default members for `cargo build/test` without `-p`.
    #[serde(default, alias = "default-members")]
    pub default_members: Vec<String>,

    /// Path to workspace root (excluding this crate).
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Shared package metadata from `[workspace.package]`.
    #[serde(default)]
    pub package: Option<WorkspacePackage>,
}

/// The `[workspace.package]` section - shared metadata for workspace members.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct WorkspacePackage {
    /// Shared authors list.
    #[serde(default)]
    pub authors: Vec<String>,

    /// Shared edition.
    pub edition: Option<String>,

    /// Shared version.
    pub version: Option<String>,

    /// Shared description.
    pub description: Option<String>,

    /// Shared homepage.
    pub homepage: Option<String>,

    /// Shared documentation URL.
    pub documentation: Option<String>,

    /// Shared license.
    pub license: Option<String>,

    /// Shared repository.
    pub repository: Option<String>,

    /// Shared rust-version.
    #[serde(alias = "rust-version")]
    pub rust_version: Option<String>,

    /// Shared README.
    pub readme: Option<ReadmeSpec>,

    /// Shared keywords.
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Shared categories.
    #[serde(default)]
    pub categories: Vec<String>,

    /// Shared license file path.
    #[serde(alias = "license-file")]
    pub license_file: Option<String>,

    /// Shared publish setting.
    #[serde(default)]
    pub publish: PublishSpec,
}

/// Dependency specification from `[dependencies]`, `[dev-dependencies]`, etc.
///
/// Handles all dependency forms:
/// - Simple: `serde = "1.0"`
/// - Table: `serde = { version = "1.0", features = ["derive"] }`
/// - Path: `my-crate = { path = "../my-crate" }`
/// - Git: `my-crate = { git = "https://...", branch = "main" }`
/// - Workspace: `my-crate = { workspace = true }`
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DepSpec {
    /// Simple version string: `"1.0"`.
    Simple(String),
    /// Full table specification.
    Detailed(DetailedDepSpec),
}

#[allow(dead_code)]
impl DepSpec {
    /// Returns the inner `DetailedDepSpec` if this is a `Detailed` variant.
    pub fn detail(&self) -> Option<&DetailedDepSpec> {
        match self {
            DepSpec::Simple(_) => None,
            DepSpec::Detailed(d) => Some(d),
        }
    }

    /// Returns `true` if this dependency inherits from workspace.
    pub fn is_workspace_inherited(&self) -> bool {
        self.detail().is_some_and(|d| d.workspace == Some(true))
    }

    /// Returns the version requirement if specified.
    pub fn version(&self) -> Option<&str> {
        match self {
            DepSpec::Simple(v) => Some(v),
            DepSpec::Detailed(d) => d.version.as_deref(),
        }
    }

    /// Returns the path if specified.
    pub fn path(&self) -> Option<&str> {
        self.detail().and_then(|d| d.path.as_deref())
    }

    /// Returns the git URL if specified.
    pub fn git(&self) -> Option<&str> {
        self.detail().and_then(|d| d.git.as_deref())
    }

    /// Returns enabled features.
    pub fn features(&self) -> &[String] {
        match self.detail() {
            Some(d) => &d.features,
            None => &[],
        }
    }

    /// Returns `true` if this is an optional dependency.
    pub fn is_optional(&self) -> bool {
        self.detail().is_some_and(|d| d.optional)
    }

    /// Returns `true` if default features are enabled (default is true).
    pub fn uses_default_features(&self) -> bool {
        self.detail().is_none_or(|d| d.default_features)
    }

    /// Returns the renamed package name if using `package = "original-name"`.
    pub fn package(&self) -> Option<&str> {
        self.detail().and_then(|d| d.package.as_deref())
    }
}

/// Detailed dependency specification (table form).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct DetailedDepSpec {
    /// Version requirement.
    pub version: Option<String>,

    /// Path to local crate.
    pub path: Option<String>,

    /// Git repository URL.
    pub git: Option<String>,

    /// Git branch.
    pub branch: Option<String>,

    /// Git tag.
    pub tag: Option<String>,

    /// Git revision.
    pub rev: Option<String>,

    /// Enabled features.
    #[serde(default)]
    pub features: Vec<String>,

    /// Whether this is optional.
    #[serde(default)]
    pub optional: bool,

    /// Whether default features are enabled.
    #[serde(default = "serde_defaults::default_true", alias = "default-features")]
    pub default_features: bool,

    /// Inherit from workspace dependencies.
    pub workspace: Option<bool>,

    /// Renamed package name.
    pub package: Option<String>,

    /// Target platform (e.g., "cfg(target_os = \"linux\")").
    pub target: Option<String>,
}

impl Default for DetailedDepSpec {
    fn default() -> Self {
        Self {
            version: None,
            path: None,
            git: None,
            branch: None,
            tag: None,
            rev: None,
            features: Vec::new(),
            optional: false,
            default_features: true,
            workspace: None,
            package: None,
            target: None,
        }
    }
}
