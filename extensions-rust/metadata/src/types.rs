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
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

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

/// CQ-002 / TASK-0477: collect member IDs once into an owned HashSet so that
/// repeat callers (members/default_members/is_member/is_default_member) do
/// not pay the per-call HashSet build or O(n) scan.
fn collect_member_ids_owned(metadata: &serde_json::Value, field: &str) -> HashSet<String> {
    metadata[field]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .map(str::to_string)
        .collect()
}

/// Parsed cargo metadata with convenient accessor methods.
///
/// `inner` is held as `Arc<Value>` so `from_context` can clone the cached
/// pointer instead of deep-cloning the whole metadata blob — cargo metadata
/// for a workspace with hundreds of dependencies routinely exceeds 1 MB and
/// the cache exists precisely so that repeat consumers (about, deps, units,
/// coverage providers) share one allocation.
///
/// **Cache lifetime (PATTERN-1 / TASK-0603):** `member_ids` and
/// `default_member_ids` live on this wrapper, not behind the `Arc`. Each call
/// to [`Metadata::from_context`] / [`Metadata::from_value`] returns a fresh
/// wrapper with empty `OnceLock`s. The HashSet build (one pass over
/// `workspace_members`) is therefore amortized within a single `Metadata`
/// instance — callers that hit `members` / `is_member` / `default_members` /
/// `is_default_member` repeatedly should hold the same `Metadata` value
/// across those calls. Building a new wrapper per call still avoids the deep
/// JSON clone (the dominant cost) but pays the HashSet build once. Moving
/// the caches behind the `Arc` would shrink that further but requires
/// interior-mutability gymnastics that the current call sites don't justify.
#[allow(dead_code)]
#[non_exhaustive]
pub struct Metadata {
    pub(crate) inner: Arc<serde_json::Value>,
    /// TASK-0477: cached `workspace_members` id set, lazily computed once.
    member_ids: OnceLock<HashSet<String>>,
    /// TASK-0477: cached `workspace_default_members` id set, lazily computed once.
    default_member_ids: OnceLock<HashSet<String>>,
    /// PERF-1 / TASK-0883: lazy package indexes keyed by name and id, each
    /// pointing at the offset in `inner["packages"][]`. Built on first
    /// `package_by_name`/`package_by_id` call so a one-shot consumer pays
    /// nothing and a multi-lookup consumer (about/units/coverage/deps in
    /// the same `Metadata`) gets O(1) average-case lookups instead of an
    /// O(n) array scan per call.
    package_index_by_name: OnceLock<HashMap<String, usize>>,
    package_index_by_id: OnceLock<HashMap<String, usize>>,
}

#[allow(dead_code)]
impl Metadata {
    /// Parse from cargo metadata JSON. Assumes the JSON is valid cargo metadata output.
    pub fn from_value(value: serde_json::Value) -> Self {
        Self {
            inner: Arc::new(value),
            member_ids: OnceLock::new(),
            default_member_ids: OnceLock::new(),
            package_index_by_name: OnceLock::new(),
            package_index_by_id: OnceLock::new(),
        }
    }

    /// Load metadata from a cached context value, sharing the cached `Arc<Value>`
    /// without deep-cloning the underlying JSON.
    pub fn from_context(ctx: &mut Context, registry: &DataRegistry) -> Result<Self, anyhow::Error> {
        let value = ctx.get_or_provide("metadata", registry)?;
        Ok(Self {
            inner: value,
            member_ids: OnceLock::new(),
            default_member_ids: OnceLock::new(),
            package_index_by_name: OnceLock::new(),
            package_index_by_id: OnceLock::new(),
        })
    }

    fn package_index_by_name(&self) -> &HashMap<String, usize> {
        self.package_index_by_name.get_or_init(|| {
            self.inner["packages"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
                .filter_map(|(i, v)| {
                    v.get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| (n.to_string(), i))
                })
                .collect()
        })
    }

    fn package_index_by_id(&self) -> &HashMap<String, usize> {
        self.package_index_by_id.get_or_init(|| {
            self.inner["packages"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
                .filter_map(|(i, v)| {
                    v.get("id")
                        .and_then(|n| n.as_str())
                        .map(|n| (n.to_string(), i))
                })
                .collect()
        })
    }

    fn package_at(&self, idx: usize) -> Option<Package<'_>> {
        self.inner["packages"]
            .as_array()
            .and_then(|arr| arr.get(idx))
            .map(|v| Package {
                inner: v,
                metadata: self,
            })
    }

    fn member_ids(&self) -> &HashSet<String> {
        self.member_ids
            .get_or_init(|| collect_member_ids_owned(&self.inner, "workspace_members"))
    }

    fn default_member_ids(&self) -> &HashSet<String> {
        self.default_member_ids
            .get_or_init(|| collect_member_ids_owned(&self.inner, "workspace_default_members"))
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
                metadata: self,
            })
    }

    /// Iterator over workspace member packages only.
    pub fn members(&self) -> impl Iterator<Item = Package<'_>> {
        let member_ids = self.member_ids();
        self.packages().filter(move |p| member_ids.contains(p.id()))
    }

    /// Iterator over default workspace member packages.
    pub fn default_members(&self) -> impl Iterator<Item = Package<'_>> {
        let default_ids = self.default_member_ids();
        self.packages()
            .filter(move |p| default_ids.contains(p.id()))
    }

    /// Find a package by name. PERF-1 / TASK-0883: O(1) average-case after
    /// first call via the lazy [`Self::package_index_by_name`] index.
    pub fn package_by_name(&self, name: &str) -> Option<Package<'_>> {
        let idx = *self.package_index_by_name().get(name)?;
        self.package_at(idx)
    }

    /// Find a package by its package ID string. PERF-1 / TASK-0883: O(1)
    /// average-case after first call via the lazy index.
    pub fn package_by_id(&self, id: &str) -> Option<Package<'_>> {
        let idx = *self.package_index_by_id().get(id)?;
        self.package_at(idx)
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
#[non_exhaustive]
pub struct Package<'a> {
    pub(crate) inner: &'a serde_json::Value,
    pub(crate) metadata: &'a Metadata,
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
        self.metadata.member_ids().contains(self.id())
    }

    /// True if this package is a default workspace member.
    pub fn is_default_member(&self) -> bool {
        self.metadata.default_member_ids().contains(self.id())
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
#[non_exhaustive]
pub enum DependencyKind {
    Normal,
    Dev,
    Build,
}

/// A dependency from a package.
#[allow(dead_code)]
#[non_exhaustive]
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
#[non_exhaustive]
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
