//! Typed accessor wrappers for cargo metadata.
//!
//! Provides ergonomic access to cargo metadata JSON through strongly-typed wrappers.
//!
//! # Code Generation (DUP-STR-001, DUP-STR-002)
//!
//! The `filter_deps_by_kind!` and `filter_targets_by_kind!` macros reduce boilerplate
//! for dependency and target accessor methods. Each macro generates multiple methods
//! that differ only by the filter predicate (enum variant or target kind string).

use ops_extension::{Context, DataRegistry};
use std::sync::Arc;

trait JsonValueExt {
    /// Returns the value at the given field, if present.
    fn get_field(&self, field: &str) -> Option<&serde_json::Value>;

    /// DUP-006: Generic helper to get a field value with fallback and tracing.
    fn get_or<F, T>(&self, field: &str, extract: F, default: T) -> T
    where
        F: FnOnce(&serde_json::Value) -> Option<T>,
    {
        match self.get_field(field).and_then(extract) {
            Some(v) => v,
            None => {
                tracing::debug!(field, "metadata field missing, using fallback");
                default
            }
        }
    }

    fn get_str_or<'a>(&'a self, field: &str, default: &'a str) -> &'a str;
    fn get_bool_or(&self, field: &str, default: bool) -> bool;
}

impl JsonValueExt for serde_json::Value {
    fn get_field(&self, field: &str) -> Option<&serde_json::Value> {
        self.get(field)
    }

    fn get_str_or<'a>(&'a self, field: &str, default: &'a str) -> &'a str {
        self.get_field(field)
            .and_then(serde_json::Value::as_str)
            .unwrap_or(default)
    }

    fn get_bool_or(&self, field: &str, default: bool) -> bool {
        self.get_or(field, |v| v.as_bool(), default)
    }
}

pub(crate) fn json_str_with_fallback<'a>(
    value: &'a serde_json::Value,
    field: &str,
    default: &'a str,
) -> &'a str {
    value.get_str_or(field, default)
}

pub(crate) fn json_bool_with_fallback(
    value: &serde_json::Value,
    field: &str,
    default: bool,
) -> bool {
    value.get_bool_or(field, default)
}

/// CQ-002: Helper to collect member IDs from a workspace field.
fn collect_member_ids<'a>(
    metadata: &'a serde_json::Value,
    field: &str,
) -> std::collections::HashSet<&'a str> {
    metadata[field]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .collect()
}

/// CQ-002: Helper to check if an ID is in a workspace field.
fn id_in_field(metadata: &serde_json::Value, field: &str, id: &str) -> bool {
    metadata[field]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .any(|member_id| member_id == id)
}

/// Parsed cargo metadata with convenient accessor methods.
///
/// `inner` is held as `Arc<Value>` so `from_context` can clone the cached
/// pointer instead of deep-cloning the whole metadata blob — cargo metadata
/// for a workspace with hundreds of dependencies routinely exceeds 1 MB and
/// the cache exists precisely so that repeat consumers (about, deps, units,
/// coverage providers) share one allocation.
#[allow(dead_code)]
pub struct Metadata {
    pub(crate) inner: Arc<serde_json::Value>,
}

#[allow(dead_code)]
impl Metadata {
    /// Parse from cargo metadata JSON. Assumes the JSON is valid cargo metadata output.
    pub fn from_value(value: serde_json::Value) -> Self {
        Self {
            inner: Arc::new(value),
        }
    }

    /// Load metadata from a cached context value, sharing the cached `Arc<Value>`
    /// without deep-cloning the underlying JSON.
    pub fn from_context(ctx: &mut Context, registry: &DataRegistry) -> Result<Self, anyhow::Error> {
        let value = ctx.get_or_provide("metadata", registry)?;
        Ok(Self { inner: value })
    }

    /// Absolute path to the workspace root directory.
    pub fn workspace_root(&self) -> &str {
        json_str_with_fallback(&self.inner, "workspace_root", "")
    }

    /// Absolute path to the target directory.
    pub fn target_directory(&self) -> &str {
        json_str_with_fallback(&self.inner, "target_directory", "")
    }

    /// Build directory if present.
    pub fn build_directory(&self) -> Option<&str> {
        self.inner["build_directory"].as_str()
    }

    /// Iterator over all packages in the dependency graph.
    pub fn packages(&self) -> impl Iterator<Item = Package<'_>> {
        self.inner["packages"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|v| Package {
                inner: v,
                metadata: &self.inner,
            })
    }

    /// Iterator over workspace member packages only.
    pub fn members(&self) -> impl Iterator<Item = Package<'_>> {
        let member_ids = collect_member_ids(&self.inner, "workspace_members");
        self.packages().filter(move |p| member_ids.contains(p.id()))
    }

    /// Iterator over default workspace member packages.
    pub fn default_members(&self) -> impl Iterator<Item = Package<'_>> {
        let default_ids = collect_member_ids(&self.inner, "workspace_default_members");
        self.packages()
            .filter(move |p| default_ids.contains(p.id()))
    }

    /// Find a package by name.
    pub fn package_by_name(&self, name: &str) -> Option<Package<'_>> {
        self.packages().find(|p| p.name() == name)
    }

    /// Find a package by its package ID string.
    pub fn package_by_id(&self, id: &str) -> Option<Package<'_>> {
        self.packages().find(|p| p.id() == id)
    }

    /// Find the root package (workspace root Cargo.toml), if present.
    /// Returns None for virtual workspaces (no root package).
    pub fn root_package(&self) -> Option<Package<'_>> {
        let ws_root = self.workspace_root();
        let expected = format!("{}/Cargo.toml", ws_root);
        self.packages().find(|p| p.manifest_path() == expected)
    }
}

/// A package from cargo metadata.
#[allow(dead_code)]
pub struct Package<'a> {
    pub(crate) inner: &'a serde_json::Value,
    pub(crate) metadata: &'a serde_json::Value,
}

#[allow(dead_code)]
impl<'a> Package<'a> {
    /// Package name.
    pub fn name(&self) -> &'a str {
        json_str_with_fallback(self.inner, "name", "")
    }

    /// Package version string.
    pub fn version(&self) -> &'a str {
        json_str_with_fallback(self.inner, "version", "")
    }

    /// Unique package ID (e.g., "path+file:///path#0.1.0").
    pub fn id(&self) -> &'a str {
        json_str_with_fallback(self.inner, "id", "")
    }

    /// Rust edition.
    pub fn edition(&self) -> &'a str {
        json_str_with_fallback(self.inner, "edition", "")
    }

    /// Absolute path to Cargo.toml.
    pub fn manifest_path(&self) -> &'a str {
        json_str_with_fallback(self.inner, "manifest_path", "")
    }

    /// License string if specified.
    pub fn license(&self) -> Option<&'a str> {
        self.inner["license"].as_str()
    }

    /// Repository URL if specified.
    pub fn repository(&self) -> Option<&'a str> {
        self.inner["repository"].as_str()
    }

    /// Description if specified.
    pub fn description(&self) -> Option<&'a str> {
        self.inner["description"].as_str()
    }

    /// True if this package is a workspace member.
    pub fn is_member(&self) -> bool {
        id_in_field(self.metadata, "workspace_members", self.id())
    }

    /// True if this package is a default workspace member.
    pub fn is_default_member(&self) -> bool {
        id_in_field(self.metadata, "workspace_default_members", self.id())
    }

    fn filter_deps_by_kind(&self, kind: DependencyKind) -> impl Iterator<Item = Dependency<'a>> {
        self.all_dependencies().filter(move |d| d.kind() == kind)
    }

    /// Normal dependencies (kind == null).
    pub fn dependencies(&self) -> impl Iterator<Item = Dependency<'a>> {
        self.filter_deps_by_kind(DependencyKind::Normal)
    }

    /// Dev dependencies (kind == "dev").
    pub fn dev_dependencies(&self) -> impl Iterator<Item = Dependency<'a>> {
        self.filter_deps_by_kind(DependencyKind::Dev)
    }

    /// Build dependencies (kind == "build").
    pub fn build_dependencies(&self) -> impl Iterator<Item = Dependency<'a>> {
        self.filter_deps_by_kind(DependencyKind::Build)
    }

    /// All dependencies regardless of kind.
    pub fn all_dependencies(&self) -> impl Iterator<Item = Dependency<'a>> {
        self.inner["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|v| Dependency { inner: v })
    }

    /// All build targets (lib, bins, tests, examples, benches).
    pub fn targets(&self) -> impl Iterator<Item = Target<'a>> {
        self.inner["targets"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|v| Target { inner: v })
    }

    /// The library target if present.
    pub fn lib_target(&self) -> Option<Target<'a>> {
        self.targets().find(|t| t.is_lib())
    }

    /// Binary targets only.
    pub fn bin_targets(&self) -> impl Iterator<Item = Target<'a>> {
        self.targets().filter(|t| t.is_bin())
    }

    /// Test targets only.
    pub fn test_targets(&self) -> impl Iterator<Item = Target<'a>> {
        self.targets().filter(|t| t.is_test())
    }

    /// Example targets only.
    pub fn example_targets(&self) -> impl Iterator<Item = Target<'a>> {
        self.targets().filter(|t| t.is_example())
    }

    /// Benchmark targets only.
    pub fn bench_targets(&self) -> impl Iterator<Item = Target<'a>> {
        self.targets().filter(|t| t.is_bench())
    }
}

/// Dependency kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DependencyKind {
    Normal,
    Dev,
    Build,
}

/// A dependency from a package.
#[allow(dead_code)]
pub struct Dependency<'a> {
    pub(crate) inner: &'a serde_json::Value,
}

#[allow(dead_code)]
impl<'a> Dependency<'a> {
    /// Dependency name.
    pub fn name(&self) -> &'a str {
        json_str_with_fallback(self.inner, "name", "")
    }

    /// Version requirement (e.g., "^1.0", "0.1.0").
    pub fn version_req(&self) -> &'a str {
        json_str_with_fallback(self.inner, "req", "")
    }

    /// Dependency kind (normal, dev, or build).
    pub fn kind(&self) -> DependencyKind {
        match self.inner["kind"].as_str() {
            Some("dev") => DependencyKind::Dev,
            Some("build") => DependencyKind::Build,
            _ => DependencyKind::Normal,
        }
    }

    /// Whether this is an optional dependency.
    pub fn is_optional(&self) -> bool {
        json_bool_with_fallback(self.inner, "optional", false)
    }

    /// Whether default features are enabled.
    pub fn uses_default_features(&self) -> bool {
        json_bool_with_fallback(self.inner, "uses_default_features", true)
    }

    /// Features enabled for this dependency.
    pub fn features(&self) -> impl Iterator<Item = &'a str> {
        self.inner["features"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
    }

    /// Renamed name if specified (e.g., `package = "original-name"`).
    pub fn rename(&self) -> Option<&'a str> {
        self.inner["rename"].as_str()
    }

    /// Target platform if specified (e.g., "wasm32-unknown-unknown").
    pub fn target(&self) -> Option<&'a str> {
        self.inner["target"].as_str()
    }

    /// Source registry or path.
    pub fn source(&self) -> Option<&'a str> {
        self.inner["source"].as_str()
    }
}

/// A build target (lib, bin, test, example, bench).
#[allow(dead_code)]
pub struct Target<'a> {
    pub(crate) inner: &'a serde_json::Value,
}

#[allow(dead_code)]
impl<'a> Target<'a> {
    /// Target name.
    pub fn name(&self) -> &'a str {
        json_str_with_fallback(self.inner, "name", "")
    }

    /// Source file path.
    pub fn src_path(&self) -> &'a str {
        json_str_with_fallback(self.inner, "src_path", "")
    }

    /// Target kinds (e.g., ["lib"], ["bin"], ["test"]).
    pub fn kinds(&self) -> impl Iterator<Item = &'a str> {
        self.inner["kind"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
    }

    fn has_kind(&self, kind: &str) -> bool {
        self.kinds().any(|k| k == kind)
    }

    /// True if this is a library target.
    pub fn is_lib(&self) -> bool {
        self.has_kind("lib")
    }

    /// True if this is a binary target.
    pub fn is_bin(&self) -> bool {
        self.has_kind("bin")
    }

    /// True if this is a test target.
    pub fn is_test(&self) -> bool {
        self.has_kind("test")
    }

    /// True if this is an example target.
    pub fn is_example(&self) -> bool {
        self.has_kind("example")
    }

    /// True if this is a benchmark target.
    pub fn is_bench(&self) -> bool {
        self.has_kind("bench")
    }

    /// Required features to build this target.
    pub fn required_features(&self) -> impl Iterator<Item = &'a str> {
        self.inner["required-features"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
    }

    /// Edition override if specified.
    pub fn edition(&self) -> Option<&'a str> {
        self.inner["edition"].as_str()
    }

    /// Documentation path if specified.
    pub fn doc_path(&self) -> Option<&'a str> {
        self.inner["doc_path"].as_str()
    }
}
