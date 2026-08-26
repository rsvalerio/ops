//! Typed accessor wrappers for cargo metadata.
//!
//! Provides ergonomic access to cargo metadata JSON through strongly-typed wrappers.
//!
//! # Dependency- and target-kind helpers (READ-1 / TASK-1552)
//!
//! Boilerplate for "filter dependencies by kind" lives on
//! [`Package::filter_deps_by_kind`], a small inherent method that wraps
//! [`Package::all_dependencies`]. The kind-aware target accessors
//! (`Package::lib_target`, `bin_targets`, `test_targets`, `example_targets`,
//! `bench_targets`) are simple iterator filters over [`Package::targets`] and
//! [`Target::has_kind`]. There are no `filter_deps_by_kind!` /
//! `filter_targets_by_kind!` macros; an earlier doc comment cited macros that
//! never existed in this file.

use ops_extension::{Context, DataProviderError, DataRegistry};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, OnceLock};

trait JsonValueExt {
    /// Returns the value at the given field, if present.
    fn get_field(&self, field: &str) -> Option<&serde_json::Value>;

    /// DUP-006: Generic helper to get a field value with fallback and tracing.
    fn get_or<F, T>(&self, field: &str, extract: F, default: T) -> T
    where
        F: FnOnce(&serde_json::Value) -> Option<T>,
    {
        self.get_field(field).and_then(extract).unwrap_or_else(|| {
            tracing::debug!(field, "metadata field missing, using fallback");
            default
        })
    }

    fn get_str_or<'a>(&'a self, field: &str, default: &'a str) -> &'a str;
    fn get_bool_or(&self, field: &str, default: bool) -> bool;

    /// PATTERN-1 / TASK-1544: iterate the elements of `self[field]` when it is
    /// a JSON array; an absent or non-array field yields an empty iterator.
    /// Centralises the `value[field].as_array().into_iter().flatten()` idiom
    /// so a future change (e.g. logging when an expected array is missing)
    /// lives in one place.
    fn array_iter<'a>(
        &'a self,
        field: &str,
    ) -> std::iter::Flatten<std::option::IntoIter<std::slice::Iter<'a, serde_json::Value>>>;

    /// PATTERN-1 / TASK-1544: iterate the string elements of `self[field]`
    /// (skipping non-string entries). Absent / non-array → empty.
    fn array_str_iter<'a>(&'a self, field: &str) -> ArrayStrIter<'a>;
}

/// PATTERN-1 / TASK-1544: concrete iterator returned by
/// [`JsonValueExt::array_str_iter`]. Carried as a nameable type so call sites
/// can be ascribed in tests and trait helpers if needed.
pub type ArrayStrIter<'a> = std::iter::FilterMap<
    std::iter::Flatten<std::option::IntoIter<std::slice::Iter<'a, serde_json::Value>>>,
    fn(&serde_json::Value) -> Option<&str>,
>;

impl JsonValueExt for serde_json::Value {
    fn get_field(&self, field: &str) -> Option<&serde_json::Value> {
        self.get(field)
    }

    fn get_str_or<'a>(&'a self, field: &str, default: &'a str) -> &'a str {
        self.get_field(field)
            .and_then(Self::as_str)
            .unwrap_or(default)
    }

    fn get_bool_or(&self, field: &str, default: bool) -> bool {
        self.get_or(field, Self::as_bool, default)
    }

    fn array_iter<'a>(
        &'a self,
        field: &str,
    ) -> std::iter::Flatten<std::option::IntoIter<std::slice::Iter<'a, serde_json::Value>>> {
        self.get_field(field)
            .and_then(Self::as_array)
            .map(|a| a.iter())
            .into_iter()
            .flatten()
    }

    fn array_str_iter<'a>(&'a self, field: &str) -> ArrayStrIter<'a> {
        self.array_iter(field).filter_map(Self::as_str)
    }
}

pub fn json_str_with_fallback<'a>(
    value: &'a serde_json::Value,
    field: &str,
    default: &'a str,
) -> &'a str {
    value.get_str_or(field, default)
}

pub fn json_bool_with_fallback(value: &serde_json::Value, field: &str, default: bool) -> bool {
    value.get_bool_or(field, default)
}

/// CQ-002 / TASK-0477: collect member IDs once into an owned `HashSet` so that
/// repeat callers (`members/default_members/is_member/is_default_member`) do
/// not pay the per-call `HashSet` build or O(n) scan.
fn collect_member_ids_owned(metadata: &serde_json::Value, field: &str) -> HashSet<String> {
    metadata.array_str_iter(field).map(str::to_string).collect()
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
/// wrapper with empty `OnceLock`s. The `HashSet` build (one pass over
/// `workspace_members`) is therefore amortized within a single `Metadata`
/// instance — callers that hit `members` / `is_member` / `default_members` /
/// `is_default_member` repeatedly should hold the same `Metadata` value
/// across those calls. Building a new wrapper per call still avoids the deep
/// JSON clone (the dominant cost) but pays the `HashSet` build once. Moving
/// the caches behind the `Arc` would shrink that further but requires
/// interior-mutability gymnastics that the current call sites don't justify.
/// READ-5 / TASK-1548: lazy caches enumerated once in a small substruct so
/// adding a new lazy field (e.g. `targets_by_kind`) does not require touching
/// every `Metadata` constructor.
#[derive(Default)]
struct MetadataCaches {
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
#[non_exhaustive]
pub struct Metadata {
    pub(crate) inner: Arc<serde_json::Value>,
    caches: MetadataCaches,
}

// The omission of `inner` and `caches` is the point of this impl.
#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for Metadata {
    /// TRAIT-1 / TASK-1541: surface coarse summary counts rather than dumping
    /// the entire `Arc<Value>` payload (cargo metadata routinely exceeds 1 MB
    /// and is unreadable in test failure output).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let packages = self
            .inner
            .get("packages")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len);
        let members = self
            .inner
            .get("workspace_members")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len);
        f.debug_struct("Metadata")
            .field("workspace_root", &self.workspace_root())
            .field("packages", &packages)
            .field("workspace_members", &members)
            .finish()
    }
}

#[allow(dead_code)]
impl Metadata {
    /// Parse from cargo metadata JSON. Assumes the JSON is valid cargo metadata output.
    #[must_use]
    pub fn from_value(value: serde_json::Value) -> Self {
        Self {
            inner: Arc::new(value),
            caches: MetadataCaches::default(),
        }
    }

    /// Load metadata from a cached context value, sharing the cached `Arc<Value>`
    /// without deep-cloning the underlying JSON.
    ///
    /// ERR-2 / TASK-1542: returns the framework's typed [`DataProviderError`]
    /// rather than `anyhow::Error` so downstream consumers can match on the
    /// failure variant (`NotFound`, `ComputationFailed`, `Serialization`,
    /// `Cycle`) without string-sniffing the chain.
    ///
    /// # Errors
    ///
    /// Whatever the `"metadata"` provider returns — typically a failing or
    /// unparseable `cargo metadata` invocation.
    pub fn from_context(
        ctx: &mut Context,
        registry: &DataRegistry,
    ) -> Result<Self, DataProviderError> {
        let value = ctx.get_or_provide("metadata", registry)?;
        Ok(Self {
            inner: value,
            caches: MetadataCaches::default(),
        })
    }

    /// DUP-1 / TASK-1539: shared backbone for [`Self::package_index_by_name`]
    /// and [`Self::package_index_by_id`]. The two indexes differ only in the
    /// extracted string field and the warn message wording.
    fn build_package_index_by(&self, field: &str, dup_msg: &str) -> HashMap<String, usize> {
        let mut map: HashMap<String, usize> = HashMap::new();
        for (i, v) in self.inner.array_iter("packages").enumerate() {
            let Some(key) = v.get(field).and_then(serde_json::Value::as_str) else {
                continue;
            };
            if let Some(&existing) = map.get(key) {
                tracing::warn!(
                    duplicate = key,
                    first_index = existing,
                    duplicate_index = i,
                    "{dup_msg}"
                );
                continue;
            }
            map.insert(key.to_string(), i);
        }
        map
    }

    /// Builds a map from package `name` to its index in `inner["packages"]`.
    ///
    /// Cargo metadata names are NOT unique across the resolution graph: a real
    /// workspace can include multiple entries with the same `name` but
    /// different versions/sources (transitive deps resolved at multiple
    /// versions, or a workspace member shadowing a registry crate).
    /// [`Self::package_by_name`] is therefore inherently ambiguous when
    /// duplicates exist. To make the non-determinism observable, this function
    /// emits a single `tracing::warn!` per duplicate name and keeps the
    /// **first-seen** entry (first-write-wins) for predictable lookups.
    /// Consumers that need version disambiguation should use
    /// [`Self::package_by_id`] instead.
    fn package_index_by_name(&self) -> &HashMap<String, usize> {
        self.caches.package_index_by_name.get_or_init(|| {
            self.build_package_index_by(
                "name",
                "duplicate package name in cargo metadata; keeping first-seen entry (use package_by_id for version disambiguation)",
            )
        })
    }

    /// Builds a map from package `id` to its index in `inner["packages"]`.
    ///
    /// Cargo metadata package ids are expected to be unique, but the contract
    /// is not enforced by `cargo metadata` itself. If duplicate ids appear
    /// (e.g. path-dep aliases, vendored crate collisions, future schema
    /// changes), this function emits a single `tracing::warn!` per duplicate
    /// and keeps the **first-seen** entry (first-write-wins) for predictable
    /// lookups via [`Self::package_by_id`].
    fn package_index_by_id(&self) -> &HashMap<String, usize> {
        self.caches.package_index_by_id.get_or_init(|| {
            self.build_package_index_by(
                "id",
                "duplicate package id in cargo metadata; keeping first-seen entry",
            )
        })
    }

    fn package_at(&self, idx: usize) -> Option<Package<'_>> {
        self.inner
            .get("packages")
            .and_then(serde_json::Value::as_array)
            .and_then(|arr| arr.get(idx))
            .map(|v| Package {
                inner: v,
                metadata: self,
            })
    }

    fn member_ids(&self) -> &HashSet<String> {
        self.caches
            .member_ids
            .get_or_init(|| collect_member_ids_owned(&self.inner, "workspace_members"))
    }

    fn default_member_ids(&self) -> &HashSet<String> {
        self.caches
            .default_member_ids
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
        // `get` matches the `Value` index behaviour for a missing key (both
        // yield `None` here) without the panic on a non-object `inner`.
        self.inner
            .get("build_directory")
            .and_then(serde_json::Value::as_str)
    }

    /// Iterator over all packages in the dependency graph.
    pub fn packages(&self) -> impl Iterator<Item = Package<'_>> {
        self.inner.array_iter("packages").map(|v| Package {
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
    ///
    /// **Ambiguity (PATTERN-1 / TASK-1019):** cargo metadata can contain
    /// multiple packages with the same `name` (different versions/sources).
    /// When duplicates exist, this returns the **first-seen** entry and the
    /// index build emits a `tracing::warn!`. For deterministic version-aware
    /// lookup use [`Self::package_by_id`].
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
        let expected = Path::new(self.workspace_root()).join("Cargo.toml");
        self.packages()
            .find(|p| Path::new(p.manifest_path()) == expected)
    }
}

/// A package from cargo metadata.
///
/// TRAIT-1 / TASK-1541: `Debug` is derived so the type is usable in
/// `assert_eq!`, `dbg!`, and `tracing::debug!(?pkg)`.
#[allow(dead_code)]
#[derive(Debug)]
#[non_exhaustive]
pub struct Package<'a> {
    pub(crate) inner: &'a serde_json::Value,
    pub(crate) metadata: &'a Metadata,
}

#[allow(dead_code)]
impl<'a> Package<'a> {
    /// Package name.
    #[must_use]
    pub fn name(&self) -> &'a str {
        json_str_with_fallback(self.inner, "name", "")
    }

    /// Package version string.
    #[must_use]
    pub fn version(&self) -> &'a str {
        json_str_with_fallback(self.inner, "version", "")
    }

    /// Unique package ID (e.g., "<path+file:///path#0.1.0>").
    #[must_use]
    pub fn id(&self) -> &'a str {
        json_str_with_fallback(self.inner, "id", "")
    }

    /// Rust edition.
    #[must_use]
    pub fn edition(&self) -> &'a str {
        json_str_with_fallback(self.inner, "edition", "")
    }

    /// Absolute path to Cargo.toml.
    #[must_use]
    pub fn manifest_path(&self) -> &'a str {
        json_str_with_fallback(self.inner, "manifest_path", "")
    }

    /// License string if specified.
    #[must_use]
    pub fn license(&self) -> Option<&'a str> {
        self.inner["license"].as_str()
    }

    /// Repository URL if specified.
    #[must_use]
    pub fn repository(&self) -> Option<&'a str> {
        self.inner["repository"].as_str()
    }

    /// Description if specified.
    #[must_use]
    pub fn description(&self) -> Option<&'a str> {
        self.inner["description"].as_str()
    }

    /// True if this package is a workspace member.
    #[must_use]
    pub fn is_member(&self) -> bool {
        self.metadata.member_ids().contains(self.id())
    }

    /// True if this package is a default workspace member.
    #[must_use]
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
        self.inner
            .array_iter("dependencies")
            .map(|v| Dependency { inner: v })
    }

    /// All build targets (lib, bins, tests, examples, benches).
    pub fn targets(&self) -> impl Iterator<Item = Target<'a>> {
        self.inner
            .array_iter("targets")
            .map(|v| Target { inner: v })
    }

    /// The library target if present.
    pub fn lib_target(&self) -> Option<Target<'a>> {
        self.targets().find(Target::is_lib)
    }

    /// Binary targets only.
    pub fn bin_targets(&self) -> impl Iterator<Item = Target<'a>> {
        self.targets().filter(Target::is_bin)
    }

    /// Test targets only.
    pub fn test_targets(&self) -> impl Iterator<Item = Target<'a>> {
        self.targets().filter(Target::is_test)
    }

    /// Example targets only.
    pub fn example_targets(&self) -> impl Iterator<Item = Target<'a>> {
        self.targets().filter(Target::is_example)
    }

    /// Benchmark targets only.
    pub fn bench_targets(&self) -> impl Iterator<Item = Target<'a>> {
        self.targets().filter(Target::is_bench)
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
///
/// TRAIT-1 / TASK-1541: `Debug` is derived so dependencies appear cleanly in
/// `tracing::debug!(?dep)` and assertion failure output.
#[allow(dead_code)]
#[derive(Debug)]
#[non_exhaustive]
pub struct Dependency<'a> {
    pub(crate) inner: &'a serde_json::Value,
}

#[allow(dead_code)]
impl<'a> Dependency<'a> {
    /// Dependency name.
    #[must_use]
    pub fn name(&self) -> &'a str {
        json_str_with_fallback(self.inner, "name", "")
    }

    /// Version requirement (e.g., "^1.0", "0.1.0").
    #[must_use]
    pub fn version_req(&self) -> &'a str {
        json_str_with_fallback(self.inner, "req", "")
    }

    /// Dependency kind (normal, dev, or build).
    #[must_use]
    pub fn kind(&self) -> DependencyKind {
        match self.inner["kind"].as_str() {
            Some("dev") => DependencyKind::Dev,
            Some("build") => DependencyKind::Build,
            _ => DependencyKind::Normal,
        }
    }

    /// Whether this is an optional dependency.
    #[must_use]
    pub fn is_optional(&self) -> bool {
        json_bool_with_fallback(self.inner, "optional", false)
    }

    /// Whether default features are enabled.
    #[must_use]
    pub fn uses_default_features(&self) -> bool {
        json_bool_with_fallback(self.inner, "uses_default_features", true)
    }

    /// Features enabled for this dependency.
    pub fn features(&self) -> impl Iterator<Item = &'a str> {
        self.inner.array_str_iter("features")
    }

    /// Renamed name if specified (e.g., `package = "original-name"`).
    #[must_use]
    pub fn rename(&self) -> Option<&'a str> {
        self.inner["rename"].as_str()
    }

    /// Target platform if specified (e.g., "wasm32-unknown-unknown").
    #[must_use]
    pub fn target(&self) -> Option<&'a str> {
        self.inner["target"].as_str()
    }

    /// Source registry or path.
    #[must_use]
    pub fn source(&self) -> Option<&'a str> {
        self.inner["source"].as_str()
    }
}

/// A build target (lib, bin, test, example, bench).
///
/// TRAIT-1 / TASK-1541: `Debug` is derived so targets appear cleanly in
/// `tracing::debug!(?target)` and assertion failure output.
#[allow(dead_code)]
#[derive(Debug)]
#[non_exhaustive]
pub struct Target<'a> {
    pub(crate) inner: &'a serde_json::Value,
}

#[allow(dead_code)]
impl<'a> Target<'a> {
    /// Target name.
    #[must_use]
    pub fn name(&self) -> &'a str {
        json_str_with_fallback(self.inner, "name", "")
    }

    /// Source file path.
    #[must_use]
    pub fn src_path(&self) -> &'a str {
        json_str_with_fallback(self.inner, "src_path", "")
    }

    /// Target kinds (e.g., `["lib"]`, `["bin"]`, `["test"]`).
    pub fn kinds(&self) -> impl Iterator<Item = &'a str> {
        self.inner.array_str_iter("kind")
    }

    fn has_kind(&self, kind: &str) -> bool {
        self.kinds().any(|k| k == kind)
    }

    /// True if this is a library target.
    #[must_use]
    pub fn is_lib(&self) -> bool {
        self.has_kind("lib")
    }

    /// True if this is a binary target.
    #[must_use]
    pub fn is_bin(&self) -> bool {
        self.has_kind("bin")
    }

    /// True if this is a test target.
    #[must_use]
    pub fn is_test(&self) -> bool {
        self.has_kind("test")
    }

    /// True if this is an example target.
    #[must_use]
    pub fn is_example(&self) -> bool {
        self.has_kind("example")
    }

    /// True if this is a benchmark target.
    #[must_use]
    pub fn is_bench(&self) -> bool {
        self.has_kind("bench")
    }

    /// Required features to build this target.
    pub fn required_features(&self) -> impl Iterator<Item = &'a str> {
        self.inner.array_str_iter("required-features")
    }

    /// Edition override if specified.
    #[must_use]
    pub fn edition(&self) -> Option<&'a str> {
        self.inner["edition"].as_str()
    }

    /// Documentation path if specified.
    #[must_use]
    pub fn doc_path(&self) -> Option<&'a str> {
        self.inner["doc_path"].as_str()
    }
}
