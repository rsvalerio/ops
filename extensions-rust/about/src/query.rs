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
use std::sync::{Arc, Mutex, OnceLock};

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
fn typed_manifest_cache() -> &'static Mutex<HashMap<PathBuf, Arc<CargoToml>>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<CargoToml>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Load and parse `Cargo.toml` for the current context, then resolve any
/// `[workspace].members` globs in place. Reuses any value already cached at
/// the `cargo_toml` key; otherwise reads via [`CargoTomlProvider`].
/// Centralises the parse + glob-resolve step that identity / units /
/// coverage providers all need (TASK-0381).
pub(crate) fn load_workspace_manifest(
    ctx: &mut Context,
) -> Result<Arc<CargoToml>, DataProviderError> {
    let cwd = ctx.working_directory.clone();
    let cache = typed_manifest_cache();

    if ctx.refresh {
        if let Ok(mut guard) = cache.lock() {
            guard.remove(&cwd);
        }
    } else if let Ok(guard) = cache.lock() {
        if let Some(arc) = guard.get(&cwd) {
            return Ok(Arc::clone(arc));
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
    if let Ok(mut guard) = cache.lock() {
        guard.insert(cwd, Arc::clone(&arc));
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
                                        resolved.push(rel.to_string_lossy().to_string());
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    parent = %parent.display(),
                                    error = %e,
                                    "workspace glob entry unreadable; skipped"
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        pattern = %member,
                        parent = %parent.display(),
                        error = %e,
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
