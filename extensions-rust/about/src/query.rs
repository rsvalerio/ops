//! Shared helpers for Rust-specific data providers.
//!
//! ## Why not `MetadataProvider`?
//!
//! `cargo metadata` is the canonical source for resolved workspace members,
//! but invoking it requires running `cargo` (slow, network-touching, and
//! requires a fully-resolvable lockfile). The about/identity/units/coverage
//! providers run on every `ops about` invocation and need to be cheap and
//! offline-tolerant. They therefore parse `Cargo.toml` directly and resolve
//! workspace globs with the static expander below. `MetadataProvider` is used
//! by `deps_provider` where dependency graph data is unavoidable.

use ops_cargo_toml::{CargoToml, CargoTomlProvider, FindWorkspaceRootError};
use ops_extension::{Context, DataProvider, DataProviderError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

/// CONC-2 / TASK-0843: soft upper bound on the typed-manifest cache so a
/// long-running daemon process that visits an unbounded set of workspaces
/// (CI worker, language-server-style host) does not accumulate one entry
/// per cwd indefinitely. When the cap is hit on insert we evict one
/// arbitrary entry rather than clearing the whole cache so steady-state
/// hits remain warm.
const MAX_TYPED_MANIFEST_CACHE_ENTRIES: usize = 64;

/// CONC-2 / TASK-0843: cache entry pairs the parsed manifest with the
/// `Cargo.toml` mtime captured at parse time. On every cache lookup we
/// re-stat the file; if the on-disk mtime differs (or the file disappeared
/// or has become unreadable) the entry is treated as stale and the
/// manifest is reparsed. `None` mtime means we couldn't stat the file at
/// parse time — in that case we keep the legacy "always trust the cache
/// until ctx.refresh" behaviour rather than thrashing on every call.
struct TypedManifestEntry {
    mtime: Option<SystemTime>,
    manifest: Arc<CargoToml>,
}

fn cargo_toml_mtime(workspace_root: &Path) -> Option<SystemTime> {
    std::fs::metadata(workspace_root.join("Cargo.toml"))
        .and_then(|m| m.modified())
        .ok()
}

/// Log a `load_workspace_manifest` failure differentiating "no manifest /
/// not a Rust project" (silent debug) from a real read/parse error (warn),
/// mirroring `read_crate_metadata` (TASK-0433).
pub(crate) fn log_manifest_load_failure(err: &DataProviderError) {
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

// PERF-1 / TASK-0558: identity, units, and coverage providers each call
// `load_workspace_manifest` during a single `ops about` invocation. The
// previous implementation cloned the cached `serde_json::Value` and
// re-deserialized it every time, even though the resolved manifest is
// identical across providers.
//
// ARCH-2 (TASK-0795): the cache lives in a process-global `Mutex<HashMap>`
// keyed by working directory rather than a `thread_local!`. The previous
// thread-local was invisible to providers scheduled on a different worker
// thread (e.g. a future tokio fan-out), silently degrading the cache to
// "off" with no signal. The mutex is held only for the lookup / insert and
// never across provider work, so contention is bounded; readers clone the
// `Arc<CargoToml>` so the typed manifest is shared across threads with no
// reparse. `ctx.refresh` evicts the entry to preserve `--refresh` semantics.
//
// ERR-1 / TASK-0844: the cache lock is acquired exclusively through
// [`lock_typed_manifest_cache`], which recovers from `PoisonError` via
// `into_inner` + `clear_poison` and warns once per process. Without this,
// a panic in a sibling provider would silently degrade the cache to
// "always-miss" — the same invisibility class the thread-local rewrite
// fought against, just routed through a different mechanism.
fn typed_manifest_cache() -> &'static Mutex<HashMap<PathBuf, TypedManifestEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, TypedManifestEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// ERR-1 / TASK-0844: acquire the typed-manifest cache lock, recovering
/// from a `PoisonError` rather than silently falling through. A poisoned
/// mutex (caused by a panic in another provider while it held the lock)
/// would otherwise degrade the cache to "always-miss" with zero diagnostic
/// — exactly the invisibility class CONC-2 / TASK-0795 fought against in
/// the previous thread_local refactor.
///
/// The cache value type is plain data (`SystemTime` + `Arc<CargoToml>`); a
/// panic in a sibling provider cannot leave it in a torn state, so
/// `into_inner()` recovery is safe.
///
/// TASK-0962: every observed poisoning emits a warn carrying a monotonic
/// `recovery_count`. The previous OnceLock-gated log fired only once per
/// process, so a second panic in a different provider was invisible —
/// defeating the "schema drift surfaces" intent. After clearing the sticky
/// poison flag, `clear_poison()` makes subsequent callers see a healthy
/// mutex; only an *actual* re-poisoning by a fresh panic increments the
/// counter.
fn lock_typed_manifest_cache(
    cache: &'static Mutex<HashMap<PathBuf, TypedManifestEntry>>,
) -> std::sync::MutexGuard<'static, HashMap<PathBuf, TypedManifestEntry>> {
    static POISON_RECOVERY_COUNT: AtomicU64 = AtomicU64::new(0);
    match cache.lock() {
        Ok(g) => g,
        Err(poison) => {
            let recovery_count = POISON_RECOVERY_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            tracing::warn!(
                recovery_count,
                "typed_manifest_cache mutex was poisoned by a panic in another provider; \
                 recovering via PoisonError::into_inner — cached entries are plain data \
                 and not torn by the panic"
            );
            let guard = poison.into_inner();
            // Clear the sticky poison flag so subsequent callers don't
            // re-enter the recovery path on every call after a single
            // panic. A fresh panic in another provider re-poisons the
            // mutex and increments `recovery_count` again.
            cache.clear_poison();
            guard
        }
    }
}

/// Load and parse `Cargo.toml` for the current context, then resolve any
/// `[workspace].members` globs in place. Reuses any value already cached at
/// the `cargo_toml` key; otherwise reads via [`CargoTomlProvider`].
/// Centralises the parse + glob-resolve step that identity / units /
/// coverage providers all need (TASK-0381).
pub(crate) fn load_workspace_manifest(
    ctx: &mut Context,
) -> Result<Arc<CargoToml>, DataProviderError> {
    let cwd: PathBuf = PathBuf::clone(&ctx.working_directory);
    let cache = typed_manifest_cache();

    let current_mtime = cargo_toml_mtime(&cwd);

    if ctx.refresh {
        let mut guard = lock_typed_manifest_cache(cache);
        guard.remove(&cwd);
    } else {
        let guard = lock_typed_manifest_cache(cache);
        if let Some(entry) = guard.get(&cwd) {
            // CONC-2 / TASK-0843: if both mtimes are known and they match,
            // serve the cached Arc. If the on-disk mtime advanced, fall
            // through to reparse. If we couldn't stat at all, fall back
            // to the legacy "trust until refresh" behaviour.
            let still_fresh = match (entry.mtime, current_mtime) {
                (Some(cached), Some(now)) => cached == now,
                _ => true,
            };
            if still_fresh {
                return Ok(Arc::clone(&entry.manifest));
            }
        }
    }

    let value = if let Some(cached) = ctx.cached(ops_cargo_toml::DATA_PROVIDER_NAME) {
        (**cached).clone()
    } else {
        CargoTomlProvider::new().provide(ctx)?
    };
    let mut manifest: CargoToml =
        serde_json::from_value(value).map_err(DataProviderError::computation_error)?;

    let resolved = resolved_workspace_members(&manifest, &cwd);
    if let Some(ws) = manifest.workspace.as_mut() {
        ws.members = resolved;
    }

    let arc = Arc::new(manifest);
    {
        let mut guard = lock_typed_manifest_cache(cache);
        // CONC-2 / TASK-0843: bound the cache. When the soft cap is hit
        // and the key is new, evict one arbitrary existing entry so we do
        // not unbounded-grow under a daemon process visiting many cwds.
        if !guard.contains_key(&cwd) && guard.len() >= MAX_TYPED_MANIFEST_CACHE_ENTRIES {
            if let Some(victim) = guard.keys().next().cloned() {
                guard.remove(&victim);
            }
        }
        guard.insert(
            cwd,
            TypedManifestEntry {
                mtime: current_mtime,
                manifest: Arc::clone(&arc),
            },
        );
    }
    Ok(arc)
}

/// Resolve `[workspace].members` globs to concrete member paths, honoring
/// `[workspace].exclude`. Members without a `*` are passed through verbatim.
///
/// Supports the simple `prefix/*` shape Cargo workspaces use in practice.
/// More elaborate patterns (`prefix/*/suffix`, `**`, `?`, character classes)
/// are not expanded — they are passed through unchanged and a `tracing::warn!`
/// is emitted so the unsupported shape is visible in logs rather than silently
/// producing a wrong member list.
pub(crate) fn resolved_workspace_members(
    manifest: &CargoToml,
    workspace_root: &Path,
) -> Vec<String> {
    let Some(ws) = manifest.workspace.as_ref() else {
        return Vec::new();
    };

    let exclude: std::collections::HashSet<&str> = ws.exclude.iter().map(String::as_str).collect();

    let mut resolved = Vec::new();
    for member in &ws.members {
        let star_idx = member.find('*');
        // PATTERN-1 (TASK-0803): detect glob shapes that lack `*` but still
        // contain class/alternation metacharacters (`crates/{core,cli}`,
        // `crates/[abc]`). Without this they would slip through as literal
        // member paths and silently produce wrong member lists.
        if star_idx.is_none() && contains_unsupported_glob_meta(member) {
            tracing::warn!(
                pattern = %member,
                "workspace member glob shape not supported by ops about; passing through unchanged"
            );
            resolved.push(member.clone());
            continue;
        }
        if let Some(idx) = star_idx {
            if is_unsupported_glob(member, idx) {
                tracing::warn!(
                    pattern = %member,
                    "workspace member glob shape not supported by ops about; passing through unchanged"
                );
                resolved.push(member.clone());
                continue;
            }
            let prefix = &member[..idx];
            let parent = workspace_root.join(prefix);
            // ERR-1: log read_dir failures at warn (matching the
            // unsupported-glob warn above) so a permission-denied / EIO on
            // crates/* does not silently produce empty about/units/coverage
            // views. Mirrors the sibling `resolve_member_globs` arm.
            match std::fs::read_dir(&parent) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(entry) => {
                                let path = entry.path();
                                if path.is_dir() && path.join("Cargo.toml").exists() {
                                    if let Ok(rel) = path.strip_prefix(workspace_root) {
                                        // READ-5 (TASK-0946): non-UTF-8
                                        // member paths must not be lossily
                                        // collapsed to U+FFFD (which would
                                        // alias two distinct members into
                                        // the same dedup key). Skip + warn
                                        // matches the `resolve_spec_cwd`
                                        // policy from TASK-0900.
                                        match rel.to_str() {
                                            Some(s) => resolved.push(s.to_string()),
                                            None => {
                                                tracing::warn!(
                                                    parent = ?parent.display(),
                                                    relpath = ?rel,
                                                    "workspace glob member relpath is not valid UTF-8; skipping"
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                // ERR-7 (TASK-0941): Debug-format path/error
                                // so attacker-controlled member paths from a
                                // cloned repo's Cargo.toml cannot forge log
                                // records.
                                tracing::warn!(
                                    parent = ?parent.display(),
                                    error = ?e,
                                    "workspace glob entry unreadable; skipped"
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    // ERR-7 (TASK-0941): Debug-format pattern / parent /
                    // error so embedded newlines / ANSI escapes in
                    // attacker-controlled `[workspace].members` entries
                    // cannot forge log records.
                    tracing::warn!(
                        pattern = ?member,
                        parent = ?parent.display(),
                        error = ?e,
                        "workspace glob prefix unreadable; member skipped"
                    );
                }
            }
        } else {
            resolved.push(member.clone());
        }
    }

    resolved.retain(|m| !exclude.contains(m.as_str()));
    resolved.sort();
    resolved
}

/// Returns true if the glob shape goes beyond a single trailing `*` after
/// the prefix — anything we cannot expand correctly with the simple
/// `read_dir(prefix)` approach.
///
/// PATTERN-1 (TASK-0803): also flag character-class closers (`]`) and brace
/// alternation (`{`, `}`). A pattern like `crates/{core,cli}` lacks `*`,
/// `?`, and `[`, so without these checks it would slip through as
/// "supported" and silently produce an empty member list when `read_dir`
/// failed on the literal-as-directory path.
fn is_unsupported_glob(member: &str, first_star: usize) -> bool {
    let after_star = &member[first_star + 1..];
    if !after_star.is_empty() {
        return true;
    }
    contains_unsupported_glob_meta(member)
}

fn contains_unsupported_glob_meta(member: &str) -> bool {
    member.contains(['?', '[', ']', '{', '}'])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_extension::Context;

    /// ERR-7 (TASK-0941): tracing fields for workspace glob walk paths flow
    /// through the `?` formatter so embedded newlines / ANSI escapes in
    /// attacker-controlled `[workspace].members` cannot forge log records.
    #[test]
    fn workspace_glob_path_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc/crates");
        let rendered = format!("{:?}", p.display());
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

    /// PERF-1 / TASK-0558: load_workspace_manifest caches the typed
    /// CargoToml in a thread-local so identity / units / coverage do not
    /// each pay for a JSON clone + reparse. Verify (a) the second call in
    /// the same context returns an `Arc` that points to the same
    /// allocation as the first, and (b) `ctx.refresh = true` invalidates
    /// the cache and yields a freshly parsed allocation.
    fn evict_cache_for(path: &Path) {
        if let Ok(mut guard) = typed_manifest_cache().lock() {
            guard.remove(path);
        }
    }

    /// PERF-3 / TASK-0969: the resolved-members list (post glob expansion)
    /// must survive across `load_workspace_manifest` calls without
    /// re-walking the filesystem. `load_workspace_manifest` rewrites
    /// `manifest.workspace.members` with the resolved list before caching
    /// the `Arc<CargoToml>`, so subsequent providers grab the resolved
    /// members directly from the cached Arc — verified here by mutating a
    /// member directory between two cached loads and asserting the cached
    /// view does NOT pick up the change (proving no re-walk).
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
        evict_cache_for(root);

        let mut ctx = Context::test_context(root.to_path_buf());
        let first = load_workspace_manifest(&mut ctx).expect("first load");
        let resolved_first = first
            .workspace
            .as_ref()
            .map(|w| w.members.clone())
            .unwrap_or_default();
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
            Arc::ptr_eq(&first, &second),
            "second call must serve the cached Arc, proving no re-walk"
        );
        let resolved_second = second
            .workspace
            .as_ref()
            .map(|w| w.members.clone())
            .unwrap_or_default();
        assert_eq!(
            resolved_second, resolved_first,
            "resolved members must be the cached snapshot, not re-walked"
        );

        evict_cache_for(root);
    }

    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_returns_same_arc_then_invalidates_on_refresh() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        evict_cache_for(dir.path());

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let first = load_workspace_manifest(&mut ctx).expect("load1");
        let second = load_workspace_manifest(&mut ctx).expect("load2");
        assert!(
            Arc::ptr_eq(&first, &second),
            "second call must reuse cached Arc"
        );

        ctx.refresh = true;
        let third = load_workspace_manifest(&mut ctx).expect("load3");
        assert!(
            !Arc::ptr_eq(&first, &third),
            "refresh=true must invalidate cache and reparse"
        );

        evict_cache_for(dir.path());
    }

    /// ARCH-2 (TASK-0795): the cache must be visible to callers on other
    /// threads. The previous `thread_local!` keyed each entry to the
    /// inserting thread, so a parallel-provider refactor would have
    /// silently re-parsed the manifest per worker. Drive a load on one
    /// thread, then assert a sibling `Context` on another thread sees the
    /// same `Arc` allocation.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_is_shared_across_threads() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        evict_cache_for(dir.path());

        let path = dir.path().to_path_buf();
        let path_for_thread = path.clone();
        let primer = std::thread::spawn(move || {
            let mut ctx = Context::test_context(path_for_thread.clone());
            load_workspace_manifest(&mut ctx).expect("primer load")
        });
        let first = primer.join().expect("primer thread");

        let path_for_reader = path.clone();
        let reader = std::thread::spawn(move || {
            let mut ctx = Context::test_context(path_for_reader);
            load_workspace_manifest(&mut ctx).expect("reader load")
        });
        let second = reader.join().expect("reader thread");

        assert!(
            Arc::ptr_eq(&first, &second),
            "cross-thread callers must share the cached Arc"
        );

        evict_cache_for(&path);
    }

    /// ERR-1 / TASK-0844: a poisoned cache mutex (caused by a panic in
    /// another provider while it held the lock) must NOT silently degrade
    /// the cache to "always-miss". load_workspace_manifest must recover via
    /// PoisonError::into_inner and emit a one-shot warn so operators see the
    /// signal instead of paying a silent perf cliff.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_recovers_from_poison_with_warn() {
        use std::sync::{Arc as StdArc, Mutex as StdMutex};
        use tracing_subscriber::fmt::MakeWriter;

        // TASK-0962: poison-recovery now logs every cycle (with a monotonic
        // recovery_count) instead of one-shot via OnceLock, so the warn is
        // always observable here regardless of sibling-test ordering.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"poisoned\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        evict_cache_for(dir.path());

        // Poison the mutex by panicking inside a held lock on another thread.
        let cache = typed_manifest_cache();
        let _ = std::thread::spawn(|| {
            let _guard = typed_manifest_cache().lock().unwrap();
            panic!("intentional poison for test");
        })
        .join();
        assert!(
            cache.lock().is_err(),
            "mutex must be poisoned for the test premise"
        );

        #[derive(Clone, Default)]
        struct BufWriter(StdArc<StdMutex<Vec<u8>>>);
        impl std::io::Write for BufWriter {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> MakeWriter<'a> for BufWriter {
            type Writer = BufWriter;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        let buf = BufWriter::default();
        let captured = buf.0.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf)
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let result =
            tracing::subscriber::with_default(subscriber, || load_workspace_manifest(&mut ctx));
        assert!(result.is_ok(), "poisoned cache must recover, not propagate");

        // TASK-0962: every recovery emits a warn carrying recovery_count, so
        // the buffer is always populated regardless of which other test
        // already poisoned this static.
        let logs = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        assert!(
            logs.contains("poisoned"),
            "poison warn must mention poisoning, got: {logs}"
        );
        assert!(
            logs.contains("recovery_count"),
            "poison warn must include monotonic recovery_count, got: {logs}"
        );

        // After recovery the cache itself is no longer poisoned-blocking
        // (we used into_inner, so the poison flag is cleared).
        assert!(
            cache.lock().is_ok(),
            "cache must be unpoisoned after into_inner recovery"
        );

        evict_cache_for(dir.path());
    }

    /// TASK-0962: a *second* poison cycle (panic in a different provider
    /// after the first recovery) must still produce an observable signal.
    /// The previous OnceLock-gated warn fired only on the first poisoning,
    /// silently swallowing every subsequent one.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_second_poison_still_logs() {
        use std::sync::{Arc as StdArc, Mutex as StdMutex};
        use tracing_subscriber::fmt::MakeWriter;

        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"poisoned-twice\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        evict_cache_for(dir.path());

        // First poison + recovery: triggered via load_workspace_manifest so
        // the OnceLock-or-counter path runs at least once before our
        // observation window opens.
        let _ = std::thread::spawn(|| {
            let _g = typed_manifest_cache().lock().unwrap();
            panic!("first poison");
        })
        .join();
        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let _ = load_workspace_manifest(&mut ctx).expect("first recovery");

        // Re-poison and capture the SECOND warn.
        let _ = std::thread::spawn(|| {
            let _g = typed_manifest_cache().lock().unwrap();
            panic!("second poison");
        })
        .join();

        #[derive(Clone, Default)]
        struct BufWriter(StdArc<StdMutex<Vec<u8>>>);
        impl std::io::Write for BufWriter {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> MakeWriter<'a> for BufWriter {
            type Writer = BufWriter;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }
        let buf = BufWriter::default();
        let captured = buf.0.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf)
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();

        let result =
            tracing::subscriber::with_default(subscriber, || load_workspace_manifest(&mut ctx));
        assert!(result.is_ok(), "second poison must also recover");

        let logs = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        assert!(
            logs.contains("poisoned") && logs.contains("recovery_count"),
            "second poison cycle must still emit a warn with recovery_count, got: {logs}"
        );

        evict_cache_for(dir.path());
    }

    /// CONC-2 / TASK-0843: a stale `Cargo.toml` mtime must invalidate the
    /// cached entry without requiring `ctx.refresh = true`.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_invalidates_on_mtime_change() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest_path = dir.path().join("Cargo.toml");
        std::fs::write(&manifest_path, "[package]\nname=\"x\"\nversion=\"0.1.0\"\n").unwrap();
        evict_cache_for(dir.path());

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let first = load_workspace_manifest(&mut ctx).expect("load1");

        // Bump the mtime by rewriting (with a sleep buffer so filesystems
        // with second-resolution mtimes (HFS+, ext3) advance it).
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(&manifest_path, "[package]\nname=\"y\"\nversion=\"0.2.0\"\n").unwrap();

        let second = load_workspace_manifest(&mut ctx).expect("load2");
        assert!(
            !Arc::ptr_eq(&first, &second),
            "mtime change must invalidate cache and reparse"
        );

        evict_cache_for(dir.path());
    }

    /// CONC-2 / TASK-0843: cache size is soft-capped so a long-running
    /// process visiting many cwds never accumulates more than
    /// MAX_TYPED_MANIFEST_CACHE_ENTRIES entries.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_is_bounded() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Insert MAX + extras directly to exercise the eviction branch.
        // We bypass load_workspace_manifest for speed: the cap is enforced
        // inside the same locked section so the contract holds for the
        // observable behaviour as well.
        let cache = typed_manifest_cache();
        {
            let mut guard = cache.lock().unwrap();
            guard.clear();
        }
        for i in 0..(MAX_TYPED_MANIFEST_CACHE_ENTRIES + 10) {
            let cwd = dir.path().join(format!("ws-{i}"));
            std::fs::create_dir_all(&cwd).unwrap();
            std::fs::write(
                cwd.join("Cargo.toml"),
                format!("[package]\nname=\"w{i}\"\nversion=\"0.1.0\"\n"),
            )
            .unwrap();
            let mut ctx = Context::test_context(cwd);
            let _ = load_workspace_manifest(&mut ctx).expect("load");
        }
        let len = cache.lock().unwrap().len();
        assert!(
            len <= MAX_TYPED_MANIFEST_CACHE_ENTRIES,
            "cache size {len} must stay within MAX_TYPED_MANIFEST_CACHE_ENTRIES = {MAX_TYPED_MANIFEST_CACHE_ENTRIES}"
        );
        // Cleanup so we don't pollute later tests in the same process.
        cache.lock().unwrap().clear();
    }

    fn manifest_with_members(members: &[&str]) -> CargoToml {
        let toml_str = format!(
            "[workspace]\nmembers = [{}]\n",
            members
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        toml::from_str(&toml_str).expect("parse manifest")
    }

    fn manifest_with_members_and_exclude(members: &[&str], exclude: &[&str]) -> CargoToml {
        let toml_str = format!(
            "[workspace]\nmembers = [{}]\nexclude = [{}]\n",
            members
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(", "),
            exclude
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        toml::from_str(&toml_str).expect("parse manifest")
    }

    #[test]
    fn resolves_simple_glob() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/bar")).unwrap();
        std::fs::write(
            root.join("crates/bar/Cargo.toml"),
            "[package]\nname=\"bar\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/not-a-crate")).unwrap();

        let manifest = manifest_with_members(&["crates/*"]);
        let resolved = resolved_workspace_members(&manifest, root);

        assert_eq!(
            resolved,
            vec!["crates/bar".to_string(), "crates/foo".to_string()]
        );
    }

    /// PATTERN-1 (TASK-0803): unsupported glob shapes (brace alternation,
    /// character classes, `?`) must pass through unchanged so downstream
    /// rendering surfaces them as-is rather than producing a silently-empty
    /// member list.
    #[test]
    fn unsupported_glob_shapes_pass_through() {
        let root = std::path::Path::new("/nonexistent");
        for pattern in [
            "crates/{core,cli}",
            "crates/[a-z]*",
            "crates/foo?",
            "crates/foo]",
        ] {
            let manifest = manifest_with_members(&[pattern]);
            let resolved = resolved_workspace_members(&manifest, root);
            assert_eq!(
                resolved,
                vec![pattern.to_string()],
                "expected `{pattern}` to pass through unchanged"
            );
        }
    }

    #[test]
    fn passthrough_non_glob_members() {
        let manifest = manifest_with_members(&["crates/core", "crates/cli"]);
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(
            resolved,
            vec!["crates/cli".to_string(), "crates/core".to_string()]
        );
    }

    #[test]
    fn empty_when_no_workspace() {
        let manifest: CargoToml =
            toml::from_str("[package]\nname=\"x\"\nversion=\"0.1.0\"\n").expect("parse");
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert!(resolved.is_empty());
    }

    #[test]
    fn nonexistent_glob_parent_yields_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest = manifest_with_members(&["crates/*"]);
        let resolved = resolved_workspace_members(&manifest, dir.path());
        assert!(resolved.is_empty());
    }

    #[test]
    fn exclude_filters_explicit_members() {
        let manifest = manifest_with_members_and_exclude(
            &["crates/core", "crates/experimental"],
            &["crates/experimental"],
        );
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(resolved, vec!["crates/core".to_string()]);
    }

    #[test]
    fn exclude_filters_glob_results() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        for name in ["foo", "bar", "experimental"] {
            std::fs::create_dir_all(root.join(format!("crates/{name}"))).unwrap();
            std::fs::write(
                root.join(format!("crates/{name}/Cargo.toml")),
                "[package]\nname=\"x\"\n",
            )
            .unwrap();
        }

        let manifest = manifest_with_members_and_exclude(&["crates/*"], &["crates/experimental"]);
        let resolved = resolved_workspace_members(&manifest, root);
        assert_eq!(
            resolved,
            vec!["crates/bar".to_string(), "crates/foo".to_string()]
        );
    }

    /// Suffix-after-`*` (e.g. `crates/*/sub`) is not supported by the simple
    /// expander. The pattern is passed through unchanged with a warn-log
    /// rather than silently producing a wrong member list (TASK-0410).
    #[test]
    fn unsupported_suffix_after_star_passes_through() {
        let manifest = manifest_with_members(&["crates/*/sub"]);
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(resolved, vec!["crates/*/sub".to_string()]);
    }

    #[test]
    fn unsupported_globstar_passes_through() {
        let manifest = manifest_with_members(&["crates/**"]);
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(resolved, vec!["crates/**".to_string()]);
    }
}
