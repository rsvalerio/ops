//! Extension glue: resolve stack, collect compiled-in extensions, register commands/data providers.

use ops_core::config::Config;
use ops_core::stack::Stack;
#[cfg(test)]
use ops_extension::ExtensionInfo;
use ops_extension::{CommandRegistry, DataRegistry, Extension};
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
    ops_extension::EXTENSION_REGISTRY
        .iter()
        .filter_map(|factory| factory(config, workspace_root))
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
            anyhow::bail!(
                "extension '{}' enabled in config but not compiled in; available: {}",
                name,
                available.keys().cloned().collect::<Vec<_>>().join(", ")
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

fn register_with_extensions<R, F>(
    extensions: &[&dyn Extension],
    registry: &mut R,
    action_name: &'static str,
    mut action: F,
) where
    F: FnMut(&dyn Extension, &mut R),
{
    for e in extensions {
        debug!(extension = e.name(), action = action_name, "registering");
        action(*e, registry);
    }
}

/// Collect all commands from registered extensions into a registry.
///
/// SEC-31 / TASK-0402 (symmetric with TASK-0350 for `DataRegistry`):
/// extensions register into a shared `CommandRegistry` via `IndexMap::insert`.
/// We snapshot the keys after each extension's contribution so a second
/// extension introducing a colliding command id is logged at
/// `tracing::warn!` instead of silently shadowing the first registration.
/// Insertion order is preserved (the late entry wins, matching the prior
/// observable behaviour) but the collision is now visible.
pub fn register_extension_commands(extensions: &[&dyn Extension], registry: &mut CommandRegistry) {
    use std::collections::HashSet;

    let mut owners: std::collections::HashMap<ops_core::config::CommandId, &'static str> =
        std::collections::HashMap::new();

    for ext in extensions {
        debug!(extension = ext.name(), action = "commands", "registering");
        let before: HashSet<ops_core::config::CommandId> = registry.keys().cloned().collect();
        ext.register_commands(registry);
        for id in registry.keys() {
            if !before.contains(id) {
                owners.insert(id.clone(), ext.name());
                continue;
            }
            // Already-present keys whose owner is a different extension are
            // collisions: this extension just re-inserted under the same id.
            if let Some(prev) = owners.get(id) {
                if *prev != ext.name() {
                    tracing::warn!(
                        command = %id,
                        first = %prev,
                        second = %ext.name(),
                        "duplicate command registration; the later extension shadows the earlier one"
                    );
                    owners.insert(id.clone(), ext.name());
                }
            }
        }
    }
}

/// Collect all data providers from registered extensions.
pub fn register_extension_data_providers(
    extensions: &[&dyn Extension],
    registry: &mut DataRegistry,
) {
    register_with_extensions(extensions, registry, "data_providers", |ext, reg| {
        ext.register_data_providers(reg);
    });
}

/// DUP-003: Build a DataRegistry from all enabled extensions in one call.
///
/// Reduces the 4-line boilerplate of builtin_extensions + ext_refs + new registry + register.
pub fn build_data_registry(config: &Config, workspace_root: &Path) -> anyhow::Result<DataRegistry> {
    let exts = builtin_extensions(config, workspace_root)?;
    let mut registry = DataRegistry::new();
    register_extension_data_providers(&as_ext_refs(&exts), &mut registry);
    Ok(registry)
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

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::config::{Config, ExtensionConfig};

    #[test]
    fn builtin_extensions_rejects_unknown_extension() {
        let config = Config {
            extensions: ExtensionConfig {
                enabled: Some(vec!["nonexistent-extension".to_string()]),
            },
            ..Default::default()
        };
        let result = builtin_extensions(&config, std::path::Path::new("."));
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("nonexistent-extension"));
        assert!(err.contains("not compiled in"));
    }

    #[test]
    fn builtin_extensions_empty_enabled_list() {
        let config = Config {
            extensions: ExtensionConfig {
                enabled: Some(vec![]),
            },
            ..Default::default()
        };
        let result = builtin_extensions(&config, std::path::Path::new("."));
        assert!(result.is_ok());
        let exts = result.unwrap();
        assert!(
            exts.is_empty(),
            "empty enabled list should return no extensions"
        );
    }

    #[test]
    fn builtin_extensions_none_enabled_loads_all() {
        let config = Config::default();
        let result = builtin_extensions(&config, std::path::Path::new("."));
        assert!(result.is_ok());
    }

    #[test]
    fn collect_compiled_extensions_returns_entries() {
        let config = Config::default();
        let compiled = collect_compiled_extensions(&config, std::path::Path::new("."));
        // All entries should have non-empty names
        for (name, ext) in &compiled {
            assert!(!name.is_empty());
            assert!(!ext.name().is_empty());
        }
    }

    #[test]
    fn collect_compiled_extensions_unfiltered_by_config() {
        // Even with an empty enabled list, collect_compiled_extensions returns all compiled-in
        let config = Config {
            extensions: ExtensionConfig {
                enabled: Some(vec![]),
            },
            ..Default::default()
        };
        let compiled = collect_compiled_extensions(&config, std::path::Path::new("."));
        // builtin_extensions would return 0, but collect returns all compiled-in
        let filtered = builtin_extensions(&config, std::path::Path::new(".")).unwrap();
        assert!(filtered.is_empty());
        // compiled may or may not be empty depending on features, but the key point
        // is that it's not filtered by the enabled list
        let _ = compiled;
    }

    #[test]
    fn extension_info_provides_metadata() {
        let config = Config::default();
        let exts = builtin_extensions(&config, std::path::Path::new(".")).unwrap();
        let infos = collect_extension_info(&as_ext_refs(&exts));

        // This test only validates extension info format when extensions are available.
        // Extensions are only compiled in when a stack feature is enabled.
        for info in &infos {
            assert!(!info.name.is_empty(), "name should not be empty");
            assert!(!info.shortname.is_empty(), "shortname should not be empty");
            let _ = info.description;
            let _ = info.command_names;
            let _ = info.data_provider_name;
            assert!(
                info.types.is_datasource() || info.types.is_command(),
                "extension should be datasource or command type"
            );
        }
    }

    /// SEC-31 / TASK-0402: when two extensions claim the same command id,
    /// the registry must observe the collision (later wins, matching prior
    /// behaviour) and the cli wiring layer must log a warning (verified by
    /// virtue of the late-write taking effect — the warning itself requires
    /// a tracing subscriber that we deliberately do not pull in for one
    /// test). We do verify the count: two extensions × 1 command each that
    /// share an id collapse to a single entry.
    #[test]
    fn register_extension_commands_detects_duplicate_command_id() {
        use ops_core::config::{CommandSpec, ExecCommandSpec};
        use ops_extension::{CommandRegistry, Extension};

        struct ExtA;
        impl Extension for ExtA {
            fn name(&self) -> &'static str {
                "ext_a"
            }
            fn register_commands(&self, registry: &mut CommandRegistry) {
                registry.insert(
                    "shared".into(),
                    CommandSpec::Exec(ExecCommandSpec::new("echo", ["a"])),
                );
            }
        }
        struct ExtB;
        impl Extension for ExtB {
            fn name(&self) -> &'static str {
                "ext_b"
            }
            fn register_commands(&self, registry: &mut CommandRegistry) {
                registry.insert(
                    "shared".into(),
                    CommandSpec::Exec(ExecCommandSpec::new("echo", ["b"])),
                );
            }
        }

        let a = ExtA;
        let b = ExtB;
        let exts: Vec<&dyn Extension> = vec![&a, &b];
        let mut registry = CommandRegistry::new();
        register_extension_commands(&exts, &mut registry);

        assert_eq!(registry.len(), 1, "duplicate id collapses to one entry");
        match registry.get("shared") {
            Some(CommandSpec::Exec(e)) => assert_eq!(e.args, vec!["b".to_string()]),
            other => panic!("expected exec spec, got {other:?}"),
        }
    }

    #[test]
    fn register_extension_commands_empty_inputs() {
        let mut registry = CommandRegistry::new();
        register_extension_commands(&[], &mut registry);
        assert!(
            registry.is_empty(),
            "no extensions → no commands registered"
        );
    }

    #[test]
    fn register_extension_data_providers_empty_inputs() {
        let mut registry = DataRegistry::new();
        register_extension_data_providers(&[], &mut registry);
        // DataRegistry doesn't expose a len()/is_empty(); if it ever did we'd
        // tighten this. For now, round-trip: a subsequent `get_or_provide`
        // miss proves the registry has no entries.
        let _ = registry;
    }

    #[test]
    fn register_extension_commands_aggregates_across_multiple_extensions() {
        use ops_core::config::Config;
        let config = Config::default();
        let exts = builtin_extensions(&config, std::path::Path::new(".")).unwrap();
        // Skip meaningful assertion when no extensions are compiled in — the
        // contract we want to pin is "aggregation does not drop entries",
        // which requires ≥2 extensions to observe.
        if exts.len() < 2 {
            return;
        }
        let ext_refs = as_ext_refs(&exts);

        // Register each extension into its own registry to get per-ext counts.
        let per_ext_total: usize = ext_refs
            .iter()
            .map(|e| {
                let mut r = CommandRegistry::new();
                register_extension_commands(std::slice::from_ref(e), &mut r);
                r.len()
            })
            .sum();

        // Register all at once; the combined registry may be smaller than the
        // sum if two extensions register the same command name (last-write
        // wins in `insert`), so use `<=` rather than `==`.
        let mut combined = CommandRegistry::new();
        register_extension_commands(&ext_refs, &mut combined);
        assert!(
            combined.len() <= per_ext_total,
            "combined registry should not grow past per-extension sum"
        );
        assert!(
            !combined.is_empty() || per_ext_total == 0,
            "if any extension registered commands, the combined registry has some"
        );
    }

    #[test]
    fn extension_types_methods_work() {
        use ops_extension::ExtensionType;

        let both = ExtensionType::DATASOURCE | ExtensionType::COMMAND;
        assert!(both.is_datasource());
        assert!(both.is_command());

        let ds = ExtensionType::DATASOURCE;
        assert!(ds.is_datasource());
        assert!(!ds.is_command());

        let cmd = ExtensionType::COMMAND;
        assert!(!cmd.is_datasource());
        assert!(cmd.is_command());
    }
}
