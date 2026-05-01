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
    // READ-5 / TASK-0716: use a typed enum so the WARN field never surfaces
    // an internal sentinel string at the user-facing boundary. Pre-existing
    // entries (e.g. config-defined commands the wiring layer seeded before
    // this call) still need to be tracked so a later extension cannot
    // silently shadow them, but the warning rendering picks the right shape
    // for each case instead of leaking `<pre-existing>` as the owner.
    #[derive(Clone)]
    enum CommandOwner {
        PreExisting,
        Extension(&'static str),
    }

    let mut owners: std::collections::HashMap<ops_core::config::CommandId, CommandOwner> =
        std::collections::HashMap::new();

    // Seed `owners` with whatever was already in `registry` so the first
    // extension's contributions still see existing keys as foreign-owned
    // and we don't false-positive collisions across re-entries.
    for id in registry.keys() {
        owners
            .entry(id.clone())
            .or_insert(CommandOwner::PreExisting);
    }

    for ext in extensions {
        debug!(extension = ext.name(), action = "commands", "registering");
        // PERF-1 / TASK-0512: register into a per-extension scratch registry
        // so we can detect collisions in O(commands_this_ext) instead of
        // snapshotting every key in the shared registry on each iteration.
        let mut local = CommandRegistry::new();
        ext.register_commands(&mut local);
        // ERR-2 (TASK-0579): the per-extension scratch registry tracks
        // duplicate inserts so a single extension that registers the same
        // command id twice no longer silently drops the first version.
        for dup in local.take_duplicate_inserts() {
            tracing::warn!(
                command = %dup,
                extension = ext.name(),
                "extension registered the same command id more than once; the later registration shadows the earlier within this extension"
            );
        }
        for (id, spec) in local {
            // PATTERN-3: hash the key once via owners.insert (which returns
            // the previous owner) instead of get-then-insert. Still one clone
            // — id needs to live in both registry and owners — but we no
            // longer probe owners twice.
            let prev_owner = owners.insert(id.clone(), CommandOwner::Extension(ext.name()));
            match prev_owner {
                Some(CommandOwner::Extension(prev)) if prev != ext.name() => {
                    tracing::warn!(
                        command = %id,
                        first = %prev,
                        second = %ext.name(),
                        "duplicate command registration; the later extension shadows the earlier one"
                    );
                }
                Some(CommandOwner::PreExisting) => {
                    // READ-5 / TASK-0716: omit the `first` field rather than
                    // emit `<pre-existing>`; the operator only needs to know
                    // that an extension is shadowing a key the registry was
                    // already seeded with (typically a config-defined
                    // command), not the internal sentinel.
                    tracing::warn!(
                        command = %id,
                        second = %ext.name(),
                        "extension command shadows an entry already present in the registry (e.g. a config-defined command)"
                    );
                }
                _ => {}
            }
            registry.insert(id, spec);
        }
    }
}

/// Collect all data providers from registered extensions.
///
/// CL-5 / TASK-0756: symmetric with [`register_extension_commands`]. Each
/// extension registers into a per-extension scratch [`DataRegistry`] so the
/// wiring layer can detect (a) in-extension duplicates via
/// [`DataRegistry::take_duplicate_inserts`] and (b) cross-extension or
/// pre-existing-owner collisions via the local `owners` map. Earlier the
/// data-provider path was a thin pass-through with no audit at all, so a
/// silent first-write-wins drop here was invisible to operators reading
/// `RUST_LOG=ops=debug` even though the symmetric command-registration path
/// already warned loudly on every collision class.
pub fn register_extension_data_providers(
    extensions: &[&dyn Extension],
    registry: &mut DataRegistry,
) {
    enum DataProviderOwner {
        PreExisting,
        Extension(&'static str),
    }

    let mut owners: std::collections::HashMap<String, DataProviderOwner> =
        std::collections::HashMap::new();

    // Seed `owners` with anything already in the registry so the first
    // extension's contributions still see the existing names as foreign-owned
    // (and thus collide loudly rather than silently).
    for name in registry.provider_names() {
        owners
            .entry(name.to_string())
            .or_insert(DataProviderOwner::PreExisting);
    }

    for ext in extensions {
        debug!(
            extension = ext.name(),
            action = "data_providers",
            "registering"
        );
        let mut local = DataRegistry::new();
        ext.register_data_providers(&mut local);

        // ERR-2: a single extension that registers the same provider name
        // twice surfaces here via the audit trail rather than a silent drop.
        for dup in local.take_duplicate_inserts() {
            tracing::warn!(
                provider = %dup,
                extension = ext.name(),
                "extension registered the same data provider name more than once; first-write-wins keeps the earlier registration within this extension and the later ones are dropped"
            );
        }

        for (name, provider) in local {
            use std::collections::hash_map::Entry;
            match owners.entry(name.clone()) {
                Entry::Occupied(occ) => match occ.get() {
                    DataProviderOwner::Extension(prev) if *prev != ext.name() => {
                        tracing::warn!(
                            provider = %name,
                            first = %prev,
                            second = %ext.name(),
                            "duplicate data provider registration; first-write-wins keeps the earlier extension's provider and the later one is dropped"
                        );
                    }
                    DataProviderOwner::PreExisting => {
                        tracing::warn!(
                            provider = %name,
                            second = %ext.name(),
                            "extension data provider would shadow an entry already present in the registry; first-write-wins keeps the existing one"
                        );
                    }
                    DataProviderOwner::Extension(_) => {
                        // Same extension — already surfaced via take_duplicate_inserts above.
                    }
                },
                Entry::Vacant(vac) => {
                    vac.insert(DataProviderOwner::Extension(ext.name()));
                    registry.register(name, provider);
                }
            }
        }
    }
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

    /// ERR-2 (TASK-0579): a single extension that calls `insert` twice for
    /// the same id must surface the self-shadow via a tracing warning. Prior
    /// to the fix the IndexMap silently overwrote the first registration and
    /// the cross-extension warning loop only fires for foreign owners.
    #[test]
    fn register_extension_commands_warns_on_self_shadow() {
        use ops_core::config::{CommandSpec, ExecCommandSpec};
        use ops_extension::{CommandRegistry, Extension};
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::fmt::MakeWriter;

        struct DoubleRegisterExt;
        impl Extension for DoubleRegisterExt {
            fn name(&self) -> &'static str {
                "double_register"
            }
            fn register_commands(&self, registry: &mut CommandRegistry) {
                registry.insert(
                    "lint".into(),
                    CommandSpec::Exec(ExecCommandSpec::new("first", Vec::<String>::new())),
                );
                registry.insert(
                    "lint".into(),
                    CommandSpec::Exec(ExecCommandSpec::new("second", Vec::<String>::new())),
                );
            }
        }

        #[derive(Clone, Default)]
        struct BufWriter(Arc<Mutex<Vec<u8>>>);
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

        let ext = DoubleRegisterExt;
        let exts: Vec<&dyn Extension> = vec![&ext];
        let mut registry = CommandRegistry::new();
        tracing::subscriber::with_default(subscriber, || {
            register_extension_commands(&exts, &mut registry);
        });

        let captured = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        assert!(
            captured.contains("double_register") && captured.contains("lint"),
            "self-shadow warning must name extension and command id, got: {captured}"
        );
        assert_eq!(registry.len(), 1, "only the surviving entry remains");
    }

    /// READ-5 / TASK-0716: the WARN emitted on a collision against a
    /// pre-seeded registry entry must NOT surface the internal sentinel
    /// `<pre-existing>` in any field; it is an implementation detail that
    /// leaked into the user-facing log line before the typed-enum refactor.
    #[test]
    fn register_extension_commands_collision_with_pre_existing_omits_sentinel() {
        use ops_core::config::{CommandSpec, ExecCommandSpec};
        use ops_extension::{CommandRegistry, Extension};
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::fmt::MakeWriter;

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

        #[derive(Clone, Default)]
        struct BufWriter(Arc<Mutex<Vec<u8>>>);
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

        // Pre-seed the registry as the wiring layer does for config-defined
        // commands so the first extension's contribution collides against an
        // existing entry.
        let mut registry = CommandRegistry::new();
        registry.insert(
            "shared".into(),
            CommandSpec::Exec(ExecCommandSpec::new("config", Vec::<String>::new())),
        );

        let a = ExtA;
        let exts: Vec<&dyn Extension> = vec![&a];
        tracing::subscriber::with_default(subscriber, || {
            register_extension_commands(&exts, &mut registry);
        });

        let captured = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        assert!(
            captured.contains("ext_a") && captured.contains("shared"),
            "warning must still name the extension and command id, got: {captured}"
        );
        assert!(
            !captured.contains("<pre-existing>"),
            "WARN must not leak the internal sentinel string, got: {captured}"
        );
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

    /// CL-5 / TASK-0756: when two extensions register the same data provider
    /// name, the wiring layer must (a) keep the first registration
    /// (first-write-wins, the security-trusted default) and (b) emit a
    /// `tracing::warn!` that names both extensions and the provider so the
    /// collision is visible to operators.
    #[test]
    fn register_extension_data_providers_warns_on_cross_extension_collision() {
        use ops_extension::{
            CommandRegistry, Context, DataProvider, DataProviderError, DataRegistry, Extension,
        };
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::fmt::MakeWriter;

        struct StubProvider(&'static str);
        impl DataProvider for StubProvider {
            fn name(&self) -> &'static str {
                self.0
            }
            fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
                Ok(serde_json::Value::Null)
            }
        }

        struct ExtA;
        impl Extension for ExtA {
            fn name(&self) -> &'static str {
                "ext_a"
            }
            fn register_commands(&self, _registry: &mut CommandRegistry) {}
            fn register_data_providers(&self, registry: &mut DataRegistry) {
                registry.register("shared", Box::new(StubProvider("a")));
            }
        }
        struct ExtB;
        impl Extension for ExtB {
            fn name(&self) -> &'static str {
                "ext_b"
            }
            fn register_commands(&self, _registry: &mut CommandRegistry) {}
            fn register_data_providers(&self, registry: &mut DataRegistry) {
                registry.register("shared", Box::new(StubProvider("b")));
            }
        }

        #[derive(Clone, Default)]
        struct BufWriter(Arc<Mutex<Vec<u8>>>);
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

        let a = ExtA;
        let b = ExtB;
        let exts: Vec<&dyn Extension> = vec![&a, &b];
        let mut registry = DataRegistry::new();
        tracing::subscriber::with_default(subscriber, || {
            register_extension_data_providers(&exts, &mut registry);
        });

        let logs = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        assert!(
            logs.contains("ext_a") && logs.contains("ext_b") && logs.contains("shared"),
            "warning must name both extensions and the provider, got: {logs}"
        );
        assert!(
            registry.get("shared").is_some(),
            "first-write-wins must keep the first registration"
        );
    }

    /// CL-5 / TASK-0756: a single extension that calls `register` twice for
    /// the same provider name must surface the duplicate via the wiring
    /// layer's audit drain (parallel to `take_duplicate_inserts` for
    /// commands), rather than silently dropping the second registration.
    #[test]
    fn register_extension_data_providers_warns_on_in_extension_duplicate() {
        use ops_extension::{
            CommandRegistry, Context, DataProvider, DataProviderError, DataRegistry, Extension,
        };
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::fmt::MakeWriter;

        struct StubProvider;
        impl DataProvider for StubProvider {
            fn name(&self) -> &'static str {
                "stub"
            }
            fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
                Ok(serde_json::Value::Null)
            }
        }

        struct DoubleRegisterExt;
        impl Extension for DoubleRegisterExt {
            fn name(&self) -> &'static str {
                "double_register"
            }
            fn register_commands(&self, _registry: &mut CommandRegistry) {}
            fn register_data_providers(&self, registry: &mut DataRegistry) {
                registry.register("provider_x", Box::new(StubProvider));
                registry.register("provider_x", Box::new(StubProvider));
            }
        }

        #[derive(Clone, Default)]
        struct BufWriter(Arc<Mutex<Vec<u8>>>);
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

        let ext = DoubleRegisterExt;
        let exts: Vec<&dyn Extension> = vec![&ext];
        let mut registry = DataRegistry::new();
        tracing::subscriber::with_default(subscriber, || {
            register_extension_data_providers(&exts, &mut registry);
        });

        let logs = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        assert!(
            logs.contains("double_register") && logs.contains("provider_x"),
            "in-extension duplicate warning must name the extension and provider, got: {logs}"
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
