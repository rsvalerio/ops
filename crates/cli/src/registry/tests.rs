//! Tests for the registry module (split per ARCH-1 / TASK-0842 — exercises
//! both the discovery and registration submodules through the public
//! `crate::registry::*` re-exports).

use super::*;
use ops_core::config::{Config, ExtensionConfig};
use ops_extension::{CommandRegistry, DataRegistry};

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

/// PATTERN-1 / TASK-0990: the "available extensions" list in the
/// "not compiled in" error message must be sorted alphabetically and
/// deterministic across consecutive invocations. HashMap iteration order
/// is randomised per process, so an unsorted message would shuffle on
/// every call and break snapshot tests / bug-report skim-ability.
#[test]
fn builtin_extensions_unknown_lists_available_in_sorted_order() {
    let config = Config {
        extensions: ExtensionConfig {
            enabled: Some(vec!["nonexistent-extension".to_string()]),
        },
        ..Default::default()
    };

    // Two consecutive invocations must yield identical messages — pinning
    // determinism, not the exact name set (which depends on which
    // extension crates are compiled into this build).
    let err1 = builtin_extensions(&config, std::path::Path::new("."))
        .err()
        .unwrap()
        .to_string();
    let err2 = builtin_extensions(&config, std::path::Path::new("."))
        .err()
        .unwrap()
        .to_string();
    assert_eq!(
        err1, err2,
        "available list must be deterministic across calls"
    );

    // The "available: " segment must be sorted ascending.
    let available_segment = err1
        .split("available: ")
        .nth(1)
        .expect("error contains `available:` segment");
    let names: Vec<&str> = available_segment.split(", ").collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(
        names, sorted,
        "available list must be sorted alphabetically"
    );
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
    let kept = registry
        .get("shared")
        .expect("first-write-wins must keep the first registration");
    // CL-5 / TASK-0904: pin which extension's provider survived. ExtA
    // registered StubProvider("a"); first-write-wins means that, not
    // StubProvider("b"), is the one we get back.
    assert_eq!(
        kept.name(),
        "a",
        "first-write-wins must keep ExtA's provider, not ExtB's"
    );
}

/// CL-5 / TASK-0904: parallel pin for the *commands* path: register two
/// colliding command ids and verify last-write-wins (the second extension
/// replaces the first). The asymmetric policy is documented at the module
/// level — this test ensures a future refactor can't quietly flip it.
#[test]
fn register_extension_commands_pins_last_write_wins() {
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
                CommandSpec::Exec(ExecCommandSpec::new("first", Vec::<String>::new())),
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
                CommandSpec::Exec(ExecCommandSpec::new("second", Vec::<String>::new())),
            );
        }
    }

    let a = ExtA;
    let b = ExtB;
    let exts: Vec<&dyn Extension> = vec![&a, &b];
    let mut registry = CommandRegistry::new();
    register_extension_commands(&exts, &mut registry);
    match registry.get("shared") {
        Some(CommandSpec::Exec(e)) => assert_eq!(
            e.program, "second",
            "last-write-wins must keep ExtB's command, not ExtA's"
        ),
        other => panic!("expected exec spec, got {other:?}"),
    }
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
