//! Loading the workspace `Cargo.toml` for the Rust about providers.
//!
//! ARCH-1 / TASK-1791: extracted from the former `query.rs`, which mixed this
//! loader with the cache it consults ([`crate::manifest_cache`]) and the glob
//! expander it calls ([`crate::members`]).

use ops_cargo_toml::{
    find_workspace_root_strict, CargoToml, CargoTomlProvider, FindWorkspaceRootError, Package,
    WorkspacePackage,
};
use ops_extension::{Context, DataProviderError};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use crate::manifest_cache;
use crate::members::resolved_workspace_members;
use crate::workspace_root_cache;

/// ERR-1 / TASK-1076: pairs the cached parsed manifest with its resolved
/// `[workspace].members` list so the original glob spec on the cached
/// `CargoToml` is preserved verbatim.
///
/// Before TASK-1076 `load_workspace_manifest` overwrote
/// `manifest.workspace.members` with the resolved list before caching the
/// `Arc<CargoToml>`. That mutation lost the literal `["crates/*"]` spec for
/// every subsequent consumer (a future linter or doc generator wanting the
/// raw spec would see only the expanded list), and any code re-running glob
/// expansion on the cached manifest no-op'd because the list was already
/// flattened. Storing the resolved view in a sibling field keeps `ws.members`
/// immutable post-parse while preserving the PERF-3 / TASK-0969 contract that
/// resolved members survive across calls without re-walking the filesystem.
///
/// OWN-12 / TASK-1767: this is an aggregate, not a smart pointer, so it does
/// **not** implement `Deref<Target = CargoToml>`. The `Deref` it used to carry
/// put `manifest.resolved_members()` (the expanded list) and
/// `manifest.workspace.members` (the raw `["crates/*"]` spec) on the same
/// receiver, and reaching for the wrong one was a silent wrong answer rather
/// than a compile error. Consumers now go through the named accessors below;
/// the unexpanded spec is reachable only via
/// `unexpanded_workspace_members_spec`, whose name says what it hands back.
///
/// C-DEBUG: derives `Debug` so a provider can `tracing::debug!(?manifest)` and
/// so `Result<LoadedManifest, _>` works with the `expect_err` family in tests.
#[derive(Clone, Debug)]
pub struct LoadedManifest {
    manifest: Arc<CargoToml>,
    /// CL-3 / TASK-1762: the resolved workspace root the manifest was loaded
    /// from. `ctx.working_directory` is the live process cwd, which may sit
    /// anywhere below this; every member join must use this root.
    workspace_root: Arc<PathBuf>,
    resolved_members: Arc<Vec<String>>,
    /// PERF-3 / TASK-1569: lazy map from workspace member (as listed in
    /// `resolved_members`) to its canonical `Cargo.toml` path. Computed
    /// once per `LoadedManifest` instance — itself cached per workspace
    /// — so `RustUnitsProvider::provide` no longer fans out N
    /// `std::fs::canonicalize` syscalls on every invocation. Held behind
    /// `Arc<OnceLock<_>>` because `LoadedManifest` is cloned freely
    /// across providers and the canonicalize work must happen once even
    /// when both `units` and a sibling consumer hit the same cache
    /// entry.
    canonical_member_manifests: Arc<OnceLock<HashMap<String, PathBuf>>>,
}

impl LoadedManifest {
    fn new(manifest: CargoToml, workspace_root: Arc<PathBuf>) -> Self {
        // ERR-1 / TASK-1076: resolve workspace members into a sibling field
        // instead of mutating `manifest.workspace.members` in place. The
        // previous mutation flattened `["crates/*"]` to the expanded list on
        // the cached Arc, hiding the original glob spec from any future
        // consumer (linter, doc generator) and silently no-op'ing any
        // re-expansion attempt.
        let resolved_members = Arc::new(resolved_workspace_members(
            &manifest,
            workspace_root.as_path(),
        ));
        Self {
            manifest: Arc::new(manifest),
            workspace_root,
            resolved_members,
            canonical_member_manifests: Arc::new(OnceLock::new()),
        }
    }

    /// The resolved workspace root this manifest was loaded from.
    ///
    /// CL-3 / TASK-1762: providers must join member paths onto *this*, never
    /// onto `ctx.working_directory` — running `ops about` from a member crate
    /// otherwise resolves every member against the wrong directory and reports
    /// a plausible-looking empty project.
    pub(crate) fn workspace_root(&self) -> &Path {
        self.workspace_root.as_path()
    }

    /// The `[package]` table, if this manifest declares one.
    pub(crate) fn package(&self) -> Option<&Package> {
        self.manifest.package.as_ref()
    }

    /// The `[workspace.package]` inheritance table, if present.
    pub(crate) fn workspace_package(&self) -> Option<&WorkspacePackage> {
        self.manifest
            .workspace
            .as_ref()
            .and_then(|w| w.package.as_ref())
    }

    /// Whether this manifest declares a `[workspace]` table at all.
    pub(crate) fn declares_workspace(&self) -> bool {
        self.manifest.workspace.is_some()
    }

    /// The **unexpanded** `[workspace].members` spec exactly as written in the
    /// manifest — e.g. `["crates/*"]`.
    ///
    /// OWN-12 / TASK-1767: every production consumer wants
    /// [`Self::resolved_members`] instead, and none currently needs the literal
    /// spec — so the accessor is test-gated rather than left standing as an
    /// unused public affordance. Its name is the point: when a
    /// linter/doc-generator consumer does arrive, the spelling at the call site
    /// cannot be mistaken for the expanded list, which is precisely what the
    /// old `Deref` into `CargoToml` allowed.
    #[cfg(test)]
    pub(crate) fn unexpanded_workspace_members_spec(&self) -> &[String] {
        self.manifest
            .workspace
            .as_ref()
            .map_or(&[][..], |w| w.members.as_slice())
    }

    /// Resolved workspace members (post glob expansion, deduped, sorted).
    /// Returns an empty slice when the manifest has no `[workspace]` table.
    ///
    /// # Ordering invariant
    ///
    /// PERF-3 / TASK-1251: this slice is produced by
    /// [`resolved_workspace_members`] which sorts (TASK-0794) and dedups
    /// (TASK-1042) the result before returning. Consumers (about, identity,
    /// units, coverage providers) MUST consume this view directly and MUST
    /// NOT re-sort it — re-sorting allocates a fresh `Vec<&str>` on every
    /// call and adds no semantic value.
    pub(crate) fn resolved_members(&self) -> &[String] {
        self.resolved_members.as_slice()
    }

    /// PERF-3 / TASK-1569: build (or return the cached) member → canonical
    /// `Cargo.toml` path map. Each member is resolved against the workspace
    /// root and then canonicalised; a failed canonicalize falls back to the
    /// unresolved path so the lookup still has a chance on platforms / paths
    /// where canonicalize errors are normal (broken symlinks). The map is
    /// keyed by member string so per-call lookups can use a `&str` borrow
    /// without allocation (TASK-1570).
    pub(crate) fn canonical_member_manifests(&self) -> &HashMap<String, PathBuf> {
        self.canonical_member_manifests.get_or_init(|| {
            self.resolved_members()
                .iter()
                .map(|member| {
                    let crate_toml = self.workspace_root().join(member).join("Cargo.toml");
                    let canonical = std::fs::canonicalize(&crate_toml).unwrap_or(crate_toml);
                    (member.clone(), canonical)
                })
                .collect()
        })
    }

    /// Whether two handles point at the same parsed `Arc<CargoToml>`
    /// allocation — the observable signal that a load was served from the
    /// typed-manifest cache rather than reparsed. Test-facing;
    /// `Arc::ptr_eq` on a private field is not reachable from sibling modules.
    #[cfg(test)]
    pub(crate) fn shares_manifest_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.manifest, &other.manifest)
    }
}

/// Log a `load_workspace_manifest` failure differentiating "no manifest /
/// not a Rust project" (silent debug) from a real read/parse error (warn),
/// mirroring `read_crate_metadata` (TASK-0433).
pub fn log_manifest_load_failure(err: &DataProviderError) {
    if is_manifest_missing(err) {
        tracing::debug!("Cargo.toml not found; Rust providers will produce empty results: {err:#}");
    } else {
        tracing::warn!("failed to load workspace Cargo.toml: {err:#}");
    }
}

fn is_manifest_missing(err: &(dyn std::error::Error + 'static)) -> bool {
    // ARCH-2 / TASK-0871: prefer the typed `FindWorkspaceRootError::NotFound`
    // marker so wrapping context layers added by future callers don't silently
    // mask the "missing manifest" signal. The legacy `io::ErrorKind::NotFound`
    // chain-walk is retained as a fallback for IO errors raised outside the
    // workspace-root walk (e.g. direct `read_to_string` failures).
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(e) = current {
        if let Some(typed) = e.downcast_ref::<FindWorkspaceRootError>() {
            return typed.is_not_found();
        }
        if let Some(io) = e.downcast_ref::<std::io::Error>() {
            return io.kind() == std::io::ErrorKind::NotFound;
        }
        current = e.source();
    }
    false
}

/// Load and parse `Cargo.toml` for the current context, then resolve any
/// `[workspace].members` globs into a sibling field on [`LoadedManifest`].
/// Reuses any value already cached at the `cargo_toml` key; otherwise reads
/// via [`CargoTomlProvider`]. Centralises the parse + glob-resolve step that
/// identity / units / coverage providers all need (TASK-0381).
///
/// FN-1 / TASK-1780: the body is orchestration only — resolve the root, probe
/// the cache, load, insert. The cache probe and insert each own their lock
/// scope in [`crate::manifest_cache`], so the CONC-7 "never hold the lock
/// across IO or parsing" contract is checkable without reading this function.
///
/// # Errors
///
/// [`DataProviderError`] when the workspace root cannot be resolved (no
/// `Cargo.toml` in the ancestor chain, or a symlink-planting rejection) or the
/// manifest cannot be read or parsed.
pub fn load_workspace_manifest(ctx: &mut Context) -> Result<LoadedManifest, DataProviderError> {
    // CL-3 / TASK-1762: resolve the workspace root ONCE and thread it through
    // freshness, cache key, glob expansion and every downstream member join.
    // `ctx.working_directory` is the live process cwd and may sit below the
    // root; the ancestor walk here is the proof the code already knows that.
    let root = resolve_workspace_root(ctx)?;
    let freshness = manifest_cache::cargo_toml_freshness(&root);

    if ctx.is_refreshing() {
        manifest_cache::evict(&root);
    } else if let Some(cached) = manifest_cache::probe(&root, freshness) {
        return Ok(cached);
    }

    let loaded = LoadedManifest::new(parse_manifest(ctx, &root)?, Arc::clone(&root));
    manifest_cache::insert(&root, freshness, &loaded);
    Ok(loaded)
}

/// SEC-25 / TASK-1204: route through the strict workspace-root finder so a
/// hostile `Cargo.toml` planted at the symlink target of an ancestor is
/// rejected before it can redirect the about/units/coverage stack. The lenient
/// `find_workspace_root` walk reaches each ancestor via `Path::parent` on the
/// lexical canonical-start path and never re-canonicalises at each step; the
/// strict variant adds a per-candidate canonicalize so a redirected ancestor
/// surfaces a tracing breadcrumb instead of becoming the discovered root.
///
/// The returned path is canonical, which is also what makes it a stable cache
/// key: two cwds inside one workspace resolve to the same root.
///
/// PERF-1 / TASK-2028: the walk is memoized per cwd in
/// [`crate::workspace_root_cache`], because keying the typed-manifest cache by
/// the resolved root (CL-3 / TASK-1762) put this walk *ahead* of the cache
/// probe — so every provider's cache hit paid for it again against the same
/// cwd. `ctx.refresh` bypasses the memo and replaces the entry, which is the
/// only in-process event that can legitimately move a cwd's root. Failures are
/// never memoized: a missing `Cargo.toml` must stay re-checkable.
fn resolve_workspace_root(ctx: &Context) -> Result<Arc<PathBuf>, DataProviderError> {
    let cwd = ctx.working_directory();
    if !ctx.is_refreshing() {
        if let Some(root) = workspace_root_cache::probe(cwd) {
            return Ok(root);
        }
    }
    let root = find_workspace_root_strict(cwd).map_err(|err| {
        tracing::debug!(
            cwd = ?cwd.display(),
            error = ?err,
            "TASK-1204: strict workspace-root resolution failed; surfacing typed error"
        );
        DataProviderError::from(anyhow::Error::from(err))
    })?;
    let root = Arc::new(root);
    workspace_root_cache::insert(cwd, &root);
    Ok(root)
}

/// PERF-1 / TASK-1195: the cache-miss path goes through
/// `CargoTomlProvider::provide_typed` so the typed `CargoToml` arrives here
/// directly — no `serde_json::Value` round-trip. The pre-existing `ctx.cached`
/// fast path stays for cross-extension consumers that may have populated the
/// JSON cache via `Context::get_or_provide`; that arm still pays one
/// `serde_json::from_value`, but the dominant typed cache miss no longer does.
fn parse_manifest(ctx: &mut Context, root: &Path) -> Result<CargoToml, DataProviderError> {
    if let Some(cached) = ctx.cached(ops_cargo_toml::DATA_PROVIDER_NAME) {
        // PERF-3 / TASK-1201: deserialize against a borrowed `&serde_json::Value`
        // instead of `(**cached).clone()` deep-cloning the entire tree before
        // `from_value` consumes it. The clone allocated one Box per nested
        // map/array node — multi-MB workspaces clone 10k+ allocations only
        // to drop them. `serde::Deserialize::deserialize` takes the value by
        // reference via its `IntoDeserializer` impl, so the cached Arc stays
        // shared and only the typed fields are produced.
        return CargoToml::deserialize(cached.as_ref())
            .map_err(DataProviderError::computation_error);
    }
    // Hand the already-resolved root to the inner provider so it does not redo
    // discovery.
    CargoTomlProvider::with_root(root.to_path_buf()).provide_typed(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_extension::Context;

    fn canonical(path: &Path) -> PathBuf {
        std::fs::canonicalize(path).expect("canonicalize tempdir")
    }

    /// PERF-3 / TASK-0969: the resolved-members list (post glob expansion)
    /// must survive across `load_workspace_manifest` calls without
    /// re-walking the filesystem. ERR-1 / TASK-1076: the resolved view is
    /// now stored in a sibling field on `LoadedManifest` (the cached
    /// `Arc<CargoToml>` keeps the original glob spec verbatim), so
    /// subsequent providers grab the resolved members from the cached
    /// `LoadedManifest::resolved_members` snapshot — verified here by
    /// mutating a member directory between two cached loads and asserting
    /// the cached view does NOT pick up the change (proving no re-walk).
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn resolved_workspace_members_are_amortised_via_typed_manifest_cache() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();
        let crates = root.join("crates");
        std::fs::create_dir(&crates).unwrap();
        let foo = crates.join("foo");
        std::fs::create_dir(&foo).unwrap();
        std::fs::write(
            foo.join("Cargo.toml"),
            "[package]\nname=\"foo\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        manifest_cache::evict(&canonical(root));

        let mut ctx = Context::test_context(root.to_path_buf());
        let first = load_workspace_manifest(&mut ctx).expect("first load");
        let resolved_first = first.resolved_members().to_vec();
        assert_eq!(resolved_first, vec!["crates/foo".to_string()]);

        // Add a sibling crate AFTER the first cache fill. If the second call
        // re-walked the filesystem the resolved list would now include
        // `crates/bar`; the cache must amortise this and keep returning the
        // same Arc with the same resolved members.
        let bar = crates.join("bar");
        std::fs::create_dir(&bar).unwrap();
        std::fs::write(
            bar.join("Cargo.toml"),
            "[package]\nname=\"bar\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();

        let second = load_workspace_manifest(&mut ctx).expect("second load");
        assert!(
            first.shares_manifest_with(&second),
            "second call must serve the cached Arc, proving no re-walk"
        );
        assert_eq!(
            second.resolved_members().to_vec(),
            resolved_first,
            "resolved members must be the cached snapshot, not re-walked"
        );

        manifest_cache::evict(&canonical(root));
    }

    /// ERR-1 / TASK-1076: `load_workspace_manifest` must NOT mutate the
    /// cached `manifest.workspace.members` to the resolved list. The
    /// original spec (e.g. `["crates/*"]`) must survive on the cached Arc
    /// across repeated calls so future consumers (linters, doc generators)
    /// that want the literal spec can read it. The expanded list is exposed
    /// separately via `LoadedManifest::resolved_members()`.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn cached_manifest_preserves_original_glob_spec_across_calls() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        manifest_cache::evict(&canonical(root));

        let mut ctx = Context::test_context(root.to_path_buf());
        let first = load_workspace_manifest(&mut ctx).expect("first load");
        let second = load_workspace_manifest(&mut ctx).expect("second load");

        // Same Arc — proves we are inspecting the cached manifest.
        assert!(
            first.shares_manifest_with(&second),
            "second call must serve the cached Arc"
        );

        // The cached manifest's literal `[workspace].members` must still be
        // the glob spec, NOT the expanded `["crates/foo"]`. Before TASK-1076
        // this would have been the resolved list because the loader
        // overwrote `ws.members` in place before caching.
        assert_eq!(
            first.unexpanded_workspace_members_spec(),
            &["crates/*".to_string()][..],
            "cached manifest must preserve the original glob spec, not the expanded list"
        );

        // Repeated calls yield consistent inputs: same glob spec on the
        // cached manifest AND the same resolved view.
        assert_eq!(
            first.resolved_members(),
            second.resolved_members(),
            "repeated calls must yield the same resolved members"
        );
        assert_eq!(
            first.resolved_members(),
            &["crates/foo".to_string()][..],
            "resolved view must reflect glob expansion"
        );

        manifest_cache::evict(&canonical(root));
    }

    /// CL-3 / TASK-1762: a cwd below the workspace root must still resolve the
    /// root's own glob members. Before the fix the ancestor walk found the
    /// root, parsed its manifest, then expanded `crates/*` by `read_dir`-ing
    /// `<cwd>/crates` — which does not exist — so the member list came back
    /// empty with no error.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn members_resolve_against_the_root_not_the_cwd() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        for name in ["alpha", "beta"] {
            let crate_dir = root.join("crates").join(name);
            std::fs::create_dir_all(crate_dir.join("src")).unwrap();
            std::fs::write(
                crate_dir.join("Cargo.toml"),
                format!("[package]\nname=\"{name}\"\nversion=\"0.1.0\"\n"),
            )
            .unwrap();
        }
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();
        manifest_cache::evict(&canonical(root));

        let mut sub_ctx = Context::test_context(root.join("crates/alpha/src"));
        let loaded = load_workspace_manifest(&mut sub_ctx).expect("load from subdirectory");

        assert_eq!(
            loaded.resolved_members(),
            &["crates/alpha".to_string(), "crates/beta".to_string()][..],
            "globs must expand against the resolved workspace root"
        );
        assert_eq!(
            loaded.workspace_root(),
            canonical(root),
            "the loaded manifest must carry the resolved root"
        );

        manifest_cache::evict(&canonical(root));
    }

    /// CL-3 / TASK-1762: the canonical member-manifest map must be built from
    /// the workspace root, so a subdirectory cwd still finds each member's
    /// `Cargo.toml`.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn canonical_member_manifests_resolve_against_the_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let crate_dir = root.join("crates/alpha");
        std::fs::create_dir_all(crate_dir.join("src")).unwrap();
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname=\"alpha\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();
        manifest_cache::evict(&canonical(root));

        let mut sub_ctx = Context::test_context(crate_dir.join("src"));
        let loaded = load_workspace_manifest(&mut sub_ctx).expect("load from subdirectory");
        let map = loaded.canonical_member_manifests();

        assert_eq!(
            map.get("crates/alpha"),
            Some(&canonical(&crate_dir.join("Cargo.toml"))),
            "member manifests must be joined onto the workspace root"
        );

        manifest_cache::evict(&canonical(root));
    }

    /// PERF-1 / TASK-2028: the canonicalizing ancestor walk runs once per cwd,
    /// not once per `load_workspace_manifest` call. Proved by making a *nearer*
    /// ancestor declare its own `[workspace]` after the first load: a second
    /// walk would stop there, so the loader still reporting the original root
    /// is the observable evidence that no second walk happened.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn workspace_root_resolution_is_memoized_per_cwd() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let member = root.join("crates/alpha");
        std::fs::create_dir_all(member.join("src")).unwrap();
        std::fs::write(
            member.join("Cargo.toml"),
            "[package]\nname=\"alpha\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();
        let cwd = member.join("src");
        manifest_cache::evict(&canonical(root));
        workspace_root_cache::evict(&cwd);

        let mut ctx = Context::test_context(cwd.clone());
        let first = load_workspace_manifest(&mut ctx).expect("first load");
        assert_eq!(first.workspace_root(), canonical(root));

        // A fresh walk from the same cwd would now stop at `crates/alpha`.
        std::fs::write(
            member.join("Cargo.toml"),
            "[workspace]\nmembers = []\n\n[package]\nname=\"alpha\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();

        let second = load_workspace_manifest(&mut ctx).expect("second load");
        assert_eq!(
            second.workspace_root(),
            canonical(root),
            "the memoized root must be reused instead of re-walking the ancestors"
        );

        // The memo is per cwd, so the same directory reached by a fresh
        // resolution (here: `ctx.refresh`) picks up the new nearer root.
        let mut refreshing = Context::test_context(cwd.clone()).with_refresh();
        let refreshed = load_workspace_manifest(&mut refreshing).expect("refreshed load");
        assert_eq!(
            refreshed.workspace_root(),
            canonical(&member),
            "ctx.refresh must bypass the memo and re-resolve the root"
        );

        manifest_cache::evict(&canonical(root));
        manifest_cache::evict(&canonical(&member));
        workspace_root_cache::evict(&cwd);
    }

    /// A missing `Cargo.toml` anywhere in the ancestor chain is an error, not
    /// an empty manifest: `resolve_workspace_root` surfaces the typed
    /// `FindWorkspaceRootError::NotFound` rather than defaulting to the cwd.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn missing_manifest_surfaces_the_typed_not_found_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let err = load_workspace_manifest(&mut ctx).expect_err("no Cargo.toml anywhere");
        assert!(
            format!("{err:#}").contains("no Cargo.toml found"),
            "expected the workspace-root NotFound error, got: {err:#}"
        );
    }

    /// ERR-1 / TASK-2024: the classification built on top of that error is
    /// what was inert. `SharedError::source()` skipped its own inner error and
    /// `From<anyhow::Error>` stored anyhow's wrapper rather than the
    /// originating error, so `is_manifest_missing`'s chain walk never reached
    /// `FindWorkspaceRootError` and returned `false` — every directory that is
    /// simply not a Rust project produced "failed to load workspace
    /// Cargo.toml" at warn. The test above used to say so in prose and
    /// deliberately declined to pin it; this pins the fixed behaviour.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn a_missing_manifest_is_classified_as_not_found_and_logged_at_debug() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cwd = dir.path().to_path_buf();
        workspace_root_cache::evict(&cwd);

        let mut ctx = Context::test_context(cwd.clone());
        let err = load_workspace_manifest(&mut ctx).expect_err("no Cargo.toml anywhere");

        assert!(
            is_manifest_missing(&err),
            "the typed NotFound marker must be reachable through the chain: {err:#}"
        );

        let (logs, ()) = ops_about::test_support::capture_tracing(tracing::Level::DEBUG, || {
            log_manifest_load_failure(&err);
        });

        assert!(
            logs.contains("Cargo.toml not found"),
            "expected the debug classification, got: {logs}"
        );
        assert!(
            !logs.contains("failed to load workspace Cargo.toml"),
            "a directory that is not a Rust project must not warn: {logs}"
        );

        workspace_root_cache::evict(&cwd);
    }

    /// The other half of the classification must still hold: a manifest that
    /// exists but does not parse is a real failure and keeps its warn.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn an_unparseable_manifest_is_not_classified_as_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(root.join("Cargo.toml"), "[workspace\nthis is not toml").unwrap();
        let cwd = root.to_path_buf();
        manifest_cache::evict(&canonical(root));
        workspace_root_cache::evict(&cwd);

        let mut ctx = Context::test_context(cwd.clone());
        let err = load_workspace_manifest(&mut ctx).expect_err("malformed manifest must fail");
        assert!(
            !is_manifest_missing(&err),
            "a parse failure must not be mistaken for an absent manifest: {err:#}"
        );

        manifest_cache::evict(&canonical(root));
        workspace_root_cache::evict(&cwd);
    }
}
