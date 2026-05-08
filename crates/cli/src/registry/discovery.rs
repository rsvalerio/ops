//! Compiled-in extension discovery and stack/config filtering.
//!
//! Split out from the parent `registry` module per ARCH-1 / TASK-0842 so
//! the discovery surface (resolve stack, enumerate compiled-in factories,
//! filter by stack + `extensions.enabled`) lives in one place separately
//! from the registration audit pipeline in [`super::registration`].

use ops_core::config::Config;
use ops_core::stack::Stack;
use ops_extension::Extension;
#[cfg(test)]
use ops_extension::ExtensionInfo;
use std::collections::BTreeMap;
use std::path::Path;
use tracing::{debug, warn};

/// Resolves the active stack from config override or auto-detection.
/// DUP-001: Delegates to `Stack::resolve()` to avoid duplicating the chain.
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
    // ERR-4 (TASK-0584): factories that return None (prerequisites not met,
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
/// # Architecture (CQ-020)
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
/// # Ordering (PATTERN-1 / TASK-1087)
///
/// The `enabled = None` branch must return extensions in a stable order
/// because [`super::registration::register_extension_commands`] is
/// **last-write-wins** on duplicate command ids (CL-5 / TASK-0904). A
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
    let stack = resolve_stack(config, workspace_root);
    let compiled = collect_compiled_extensions(config, workspace_root);

    let stack_filtered: Vec<(&'static str, Box<dyn Extension>)> = compiled
        .into_iter()
        .filter(|(_, ext)| match ext.stack() {
            None => true,
            Some(ext_stack) => stack == Some(ext_stack),
        })
        .collect();
    let mut available = dedup_compiled_extensions(stack_filtered);

    if let Some(s) = stack {
        debug!(stack = ?s, "stack resolved");
    } else {
        debug!("no stack detected, loading generic extensions only");
    }

    let Some(enabled) = &config.extensions.enabled else {
        let exts: Vec<Box<dyn Extension>> = available.into_values().collect();
        debug!(count = exts.len(), "stack-filtered extensions loaded");
        return Ok(exts);
    };

    for name in enabled {
        if !available.contains_key(name.as_str()) {
            // PATTERN-1 / TASK-0990: HashMap iteration order is randomised
            // per process; render a sorted list so the message is
            // deterministic, snapshot-friendly, and skim-able by operators
            // copy-pasting into bug reports.
            let mut available_names: Vec<&'static str> = available.keys().copied().collect();
            available_names.sort_unstable();
            anyhow::bail!(
                "extension '{}' enabled in config but not compiled in; available: {}",
                name,
                available_names.join(", ")
            );
        }
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
/// PATTERN-1 / TASK-1088: the symmetric command and data-provider audit
/// pipelines (CL-5 / TASK-0756, DUP-1 / TASK-0876) explicitly surface
/// duplicate registrations via `take_duplicate_inserts` + `tracing::warn!`;
/// the discovery layer regressed by collapsing the `Vec<(name, ext)>` it
/// gets back from `collect_compiled_extensions` straight into a map,
/// dropping the earlier `Box` for any colliding `config_name` with no
/// breadcrumb.
///
/// **Resolution policy**: last-write-wins on duplicate `config_name`,
/// matching `register_extension_commands` (CL-5 / TASK-0904).
///
/// **Iteration order** (PATTERN-1 / TASK-1087): `BTreeMap` yields entries
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
