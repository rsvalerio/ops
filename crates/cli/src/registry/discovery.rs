//! Compiled-in extension discovery and stack/config filtering.
//!
//! Split out from the parent `registry` module so the discovery surface
//! (resolve stack, enumerate compiled-in factories, filter by stack +
//! `extensions.enabled`) lives in one place separately from the
//! registration audit pipeline in [`super::registration`].

use ops_core::config::Config;
use ops_core::stack::Stack;
use ops_extension::Extension;
#[cfg(test)]
use ops_extension::ExtensionInfo;
use std::collections::BTreeMap;
use std::path::Path;
use tracing::{debug, warn};

/// Resolves the active stack from config override or auto-detection.
/// Delegates to `Stack::resolve()` to avoid duplicating the chain.
pub fn resolve_stack(config: &Config, workspace_root: &Path) -> Option<Stack> {
    Stack::resolve(config.stack.as_deref(), workspace_root)
}

/// Returns all compiled-in extensions as (config_name, extension) pairs.
/// Does not filter by config or stack — caller decides what to do with disabled extensions.
///
/// Extensions self-register via `impl_extension!` with a `factory:` arm,
/// which contributes to the `EXTENSION_REGISTRY` distributed slice at link time.
/// No manual registration needed — if the crate is linked, it's discovered.
pub fn collect_compiled_extensions(
    config: &Config,
    workspace_root: &Path,
) -> Vec<(&'static str, Box<dyn Extension>)> {
    // Factories that return None (prerequisites not met,
    // e.g. wrong stack, missing tool on PATH) used to be dropped silently —
    // an extension that compiled in but quietly opts out was
    // indistinguishable from one that never linked. Emit a one-shot debug
    // event per slot so `RUST_LOG=ops=debug` answers "the X extension is not
    // running for me" without changing behaviour for the success path.
    ops_extension::EXTENSION_REGISTRY
        .iter()
        .enumerate()
        .filter_map(|(slot, factory)| match factory(config, workspace_root) {
            Some(pair) => Some(pair),
            None => {
                debug!(
                    slot,
                    "extension factory declined to construct (returned None); compiled in but inactive"
                );
                None
            }
        })
        .collect()
}

/// Collect all built-in extensions (feature-gated), filtered by config and stack.
/// Returns an error if any enabled extension is not compiled in.
///
/// # Filtering Logic
///
/// Extensions are filtered in two stages:
/// 1. **By stack**: Only extensions where `stack()` returns `None` (generic) or
///    matches the detected/configured stack are included
/// 2. **By config**: If `extensions.enabled` is set, only those named extensions are loaded
///
/// # Architecture
///
/// This function uses a two-phase approach:
/// 1. **Collection**: Build a `BTreeMap` of all compiled-in extensions
/// 2. **Filtering**: Return only enabled extensions, or all if none specified
///
/// The `BTreeMap` serves three purposes:
/// - Enables O(log n) lookup for the "not compiled in" error message
/// - Allows efficient filtering by key removal
/// - Yields deterministic, sorted-by-`config_name` iteration order
///
/// # Ordering
///
/// The `enabled = None` branch must return extensions in a stable order
/// because [`super::registration::register_extension_commands`] is
/// **last-write-wins** on duplicate command ids. A
/// `HashMap`-random iteration order would let the surviving extension for a
/// command-id collision flip between processes — genuine functional
/// non-determinism, not just log noise. Sorting by `config_name` (via
/// `BTreeMap`) pins the winner across builds and matches the deterministic
/// `enabled = Some(..)` branch (which iterates the user's config order).
///
/// Alternative designs considered:
/// - Vec + iterator filter: Simpler but O(n) for each lookup
/// - Registry pattern: More complex for the current 3-4 extensions
pub fn builtin_extensions(
    config: &Config,
    workspace_root: &Path,
) -> anyhow::Result<Vec<Box<dyn Extension>>> {
    let compiled = collect_compiled_extensions(config, workspace_root);
    builtin_extensions_from(compiled, config, workspace_root)
}

/// Same as [`builtin_extensions`] but consumes a
/// pre-collected `compiled` vector so callers that already walked
/// `EXTENSION_REGISTRY` (e.g. `run_extension_show_to`) do not pay the
/// factory probes twice. Public surface is unchanged: [`builtin_extensions`]
/// still owns the collection step for callers that don't already hold the
/// pairs.
pub fn builtin_extensions_from(
    compiled: Vec<(&'static str, Box<dyn Extension>)>,
    config: &Config,
    workspace_root: &Path,
) -> anyhow::Result<Vec<Box<dyn Extension>>> {
    let stack = resolve_stack(config, workspace_root);
    let compiled_names: std::collections::BTreeSet<&'static str> =
        compiled.iter().map(|(n, _)| *n).collect();
    let stack_filtered = filter_by_stack(compiled, stack);
    let available = dedup_compiled_extensions(stack_filtered);

    if let Some(s) = stack {
        debug!(stack = ?s, "stack resolved");
    } else {
        debug!("no stack detected, loading generic extensions only");
    }

    select_enabled(
        available,
        &compiled_names,
        stack,
        config.extensions.enabled.as_deref(),
    )
}

/// Stack-filter pass extracted from `builtin_extensions` so the policy
/// is independently testable. Generic extensions
/// (`stack() == None`) always pass; stack-specific extensions only when
/// `ext.stack() == detected`.
fn filter_by_stack(
    compiled: Vec<(&'static str, Box<dyn Extension>)>,
    detected: Option<Stack>,
) -> Vec<(&'static str, Box<dyn Extension>)> {
    compiled
        .into_iter()
        .filter(|(_, ext)| match ext.stack() {
            None => true,
            Some(ext_stack) => detected == Some(ext_stack),
        })
        .collect()
}

/// Enabled-validation + ordering pass extracted from `builtin_extensions`.
/// Aggregates every missing entry into one error and distinguishes
/// "compiled in but stack-filtered" from "not compiled in" using the
/// unfiltered `compiled_names` set.
fn select_enabled(
    mut available: std::collections::BTreeMap<&'static str, Box<dyn Extension>>,
    compiled_names: &std::collections::BTreeSet<&'static str>,
    detected_stack: Option<Stack>,
    enabled: Option<&[String]>,
) -> anyhow::Result<Vec<Box<dyn Extension>>> {
    let Some(enabled) = enabled else {
        let exts: Vec<Box<dyn Extension>> = available.into_values().collect();
        debug!(count = exts.len(), "stack-filtered extensions loaded");
        return Ok(exts);
    };

    let mut missing_not_compiled: Vec<&str> = Vec::new();
    let mut missing_stack_filtered: Vec<&str> = Vec::new();
    for name in enabled {
        if !available.contains_key(name.as_str()) {
            if compiled_names.contains(name.as_str()) {
                missing_stack_filtered.push(name.as_str());
            } else {
                missing_not_compiled.push(name.as_str());
            }
        }
    }
    if !missing_not_compiled.is_empty() || !missing_stack_filtered.is_empty() {
        // Sort `available` so the rendered list is
        // deterministic across processes and snapshot-friendly.
        let mut available_names: Vec<&'static str> = available.keys().copied().collect();
        available_names.sort_unstable();
        let stack_label = detected_stack.map_or_else(
            || "no stack detected".to_string(),
            |s| s.as_str().to_string(),
        );
        let mut parts: Vec<String> = Vec::new();
        if !missing_not_compiled.is_empty() {
            parts.push(format!(
                "not compiled in: {}",
                missing_not_compiled.join(", ")
            ));
        }
        if !missing_stack_filtered.is_empty() {
            parts.push(format!(
                "compiled in but disabled for the current stack ({stack_label}): {}",
                missing_stack_filtered.join(", ")
            ));
        }
        anyhow::bail!(
            "extensions enabled in config but unavailable — {}; available: {}",
            parts.join("; "),
            available_names.join(", ")
        );
    }
    let exts: Vec<Box<dyn Extension>> = enabled
        .iter()
        .filter_map(|name| available.remove(name.as_str()))
        .collect();
    debug!(count = exts.len(), "extensions loaded from config");
    Ok(exts)
}

/// Collapse `(config_name, extension)` pairs into a `BTreeMap`, emitting a
/// `tracing::warn!` audit breadcrumb for each duplicate `config_name` that
/// would otherwise be silently dropped by `BTreeMap::insert`.
///
/// The symmetric command and data-provider audit pipelines explicitly
/// surface duplicate registrations via `take_duplicate_inserts` +
/// `tracing::warn!`; the discovery layer used to regress by collapsing the
/// `Vec<(name, ext)>` it gets back from `collect_compiled_extensions`
/// straight into a map, dropping the earlier `Box` for any colliding
/// `config_name` with no breadcrumb.
///
/// **Resolution policy**: last-write-wins on duplicate `config_name`,
/// matching `register_extension_commands`.
///
/// **Iteration order**: `BTreeMap` yields entries
/// sorted by `config_name`. The `enabled = None` branch of
/// [`builtin_extensions`] forwards that order to
/// `register_extension_commands`, so the last-write-wins winner of a
/// command-id collision between two compiled-in extensions is stable
/// across processes. Using a `HashMap` here would re-randomise that order
/// per process and silently flip the winner.
pub(super) fn dedup_compiled_extensions(
    pairs: Vec<(&'static str, Box<dyn Extension>)>,
) -> BTreeMap<&'static str, Box<dyn Extension>> {
    let mut available: BTreeMap<&'static str, Box<dyn Extension>> = BTreeMap::new();
    for (name, ext) in pairs {
        let new_ext_name = ext.name();
        if let Some(prev) = available.insert(name, ext) {
            warn!(
                config_name = name,
                first = prev.name(),
                second = new_ext_name,
                "duplicate compiled-in extension config_name; last-write-wins, the earlier extension is dropped"
            );
        }
    }
    available
}

/// Convert boxed extensions to trait-object references.
pub fn as_ext_refs(exts: &[Box<dyn Extension>]) -> Vec<&dyn Extension> {
    exts.iter().map(|b| b.as_ref()).collect()
}

/// Collect metadata/info for all extensions.
#[cfg(test)]
pub fn collect_extension_info(extensions: &[&dyn Extension]) -> Vec<ExtensionInfo> {
    extensions.iter().map(|e| e.info()).collect()
}
