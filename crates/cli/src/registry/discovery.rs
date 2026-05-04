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
use std::collections::HashMap;
use std::path::Path;
use tracing::debug;

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
/// 1. **Collection**: Build a HashMap of all compiled-in extensions
/// 2. **Filtering**: Return only enabled extensions, or all if none specified
///
/// The HashMap serves dual purposes:
/// - Enables O(1) lookup for the "not compiled in" error message
/// - Allows efficient filtering by key removal
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

    let mut available: HashMap<&'static str, Box<dyn Extension>> = compiled
        .into_iter()
        .filter(|(_, ext)| match ext.stack() {
            None => true,
            Some(ext_stack) => stack == Some(ext_stack),
        })
        .collect();

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

/// Convert boxed extensions to trait-object references.
pub fn as_ext_refs(exts: &[Box<dyn Extension>]) -> Vec<&dyn Extension> {
    exts.iter().map(|b| b.as_ref()).collect()
}

/// Collect metadata/info for all extensions.
#[cfg(test)]
pub fn collect_extension_info(extensions: &[&dyn Extension]) -> Vec<ExtensionInfo> {
    extensions.iter().map(|e| e.info()).collect()
}
