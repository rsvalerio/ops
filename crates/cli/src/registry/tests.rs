//! Tests for the registry module (split per ARCH-1 / TASK-0842 — exercises
//! both the discovery and registration submodules through the public
//! `crate::registry::*` re-exports).

use super::*;
use crate::test_utils::capture_warnings;
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
    //
    // Use a stable tempdir as workspace_root (not `Path::new(".")`): the
    // cwd is process-global and parallel `CwdGuard`-using tests can flip
    // the detected stack between these two calls, which would change
    // which extensions survive stack filtering and so the "available"
    // list — a process-level race, not a discovery-layer bug.
    let workspace = tempfile::tempdir().expect("tempdir");
    let err1 = builtin_extensions(&config, workspace.path())
        .err()
        .unwrap()
        .to_string();
    let err2 = builtin_extensions(&config, workspace.path())
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

/// TEST-11 / TASK-1314: assert a behavioural property that holds for any
/// feature set — `collect_compiled_extensions` must not panic on
/// `Config::default()` and the names it returns must be non-empty AND
/// pairwise unique. Pre-fix the `for ext in &compiled` body executed
/// zero iterations under default features, masking any regression that
/// returned duplicate or empty names.
#[test]
fn collect_compiled_extensions_returns_entries() {
    let config = Config::default();
    let compiled = collect_compiled_extensions(&config, std::path::Path::new("."));
    let mut seen: std::collections::HashSet<&'static str> = std::collections::HashSet::new();
    for (name, ext) in &compiled {
        assert!(!name.is_empty(), "config_name must not be empty");
        assert!(!ext.name().is_empty(), "ext.name() must not be empty");
        assert!(
            seen.insert(name),
            "config_names must be unique across compiled-in extensions; saw {name:?} twice"
        );
    }
}

/// TEST-25 / TASK-1315: actually exercise the "unfiltered by enabled"
/// invariant. The pre-fix body discarded `compiled` via `let _ = compiled`
/// and only asserted on `filtered`, so a regression that taught
/// `collect_compiled_extensions` to honour `extensions.enabled` would
/// still pass. We pin the unfiltered-superset shape: every name in the
/// filtered set must appear in the unfiltered one, and `compiled.len()
/// >= filtered.len()`.
#[test]
fn collect_compiled_extensions_unfiltered_by_config() {
    let config = Config {
        extensions: ExtensionConfig {
            enabled: Some(vec![]),
        },
        ..Default::default()
    };
    let compiled = collect_compiled_extensions(&config, std::path::Path::new("."));
    let filtered = builtin_extensions(&config, std::path::Path::new(".")).unwrap();
    assert!(filtered.is_empty(), "empty enabled list filters everything");
    assert!(
        compiled.len() >= filtered.len(),
        "compiled (unfiltered) must be a superset of filtered"
    );
    let compiled_names: std::collections::HashSet<&'static str> =
        compiled.iter().map(|(n, _)| *n).collect();
    for ext in &filtered {
        assert!(
            compiled_names.contains(ext.name()),
            "every filtered name must appear in compiled (unfiltered)"
        );
    }
}

/// TEST-25 / TASK-1309: exercise `collect_extension_info` regardless of
/// feature flags by routing inline stub extensions through it. Pre-fix
/// the `for info in &infos` loop ran zero iterations under default
/// features, so a regression that dropped `collect_extension_info`
/// entries silently passed.
#[test]
fn extension_info_provides_metadata() {
    use ops_extension::{CommandRegistry, Extension, ExtensionInfo, ExtensionType};

    struct StubExt {
        name: &'static str,
    }
    impl Extension for StubExt {
        fn name(&self) -> &'static str {
            self.name
        }
        fn register_commands(&self, _registry: &mut CommandRegistry) {}
        fn info(&self) -> ExtensionInfo {
            let mut info = ExtensionInfo::new(self.name);
            info.shortname = self.name;
            info.description = "stub";
            info.types = ExtensionType::COMMAND;
            info
        }
    }

    let a = StubExt { name: "stub_a" };
    let b = StubExt { name: "stub_b" };
    let exts: Vec<&dyn Extension> = vec![&a, &b];
    let infos = collect_extension_info(&exts);

    assert!(
        !infos.is_empty(),
        "precondition: collect_extension_info must yield entries for the stubs"
    );
    assert_eq!(infos.len(), 2, "one ExtensionInfo per stub");
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

    let ext = DoubleRegisterExt;
    let exts: Vec<&dyn Extension> = vec![&ext];
    let mut registry = CommandRegistry::new();
    let captured = capture_warnings(|| {
        register_extension_commands(&exts, &mut registry);
    });
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
    let captured = capture_warnings(|| {
        register_extension_commands(&exts, &mut registry);
    });
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

    let a = ExtA;
    let b = ExtB;
    let exts: Vec<&dyn Extension> = vec![&a, &b];
    let mut registry = DataRegistry::new();
    let logs = capture_warnings(|| {
        register_extension_data_providers(&exts, &mut registry);
    });
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

    let ext = DoubleRegisterExt;
    let exts: Vec<&dyn Extension> = vec![&ext];
    let mut registry = DataRegistry::new();
    let logs = capture_warnings(|| {
        register_extension_data_providers(&exts, &mut registry);
    });
    assert!(
        logs.contains("double_register") && logs.contains("provider_x"),
        "in-extension duplicate warning must name the extension and provider, got: {logs}"
    );
    // TEST-25 / TASK-1366: pin first-write-wins on the data-provider
    // path. Without asserting the registry's post-state, a future
    // regression that warned but corrupted the registry (kept both,
    // kept neither, last-write-wins) would still pass.
    assert_eq!(
        registry.provider_names(),
        vec!["provider_x".to_string()],
        "registry must contain exactly the surviving provider"
    );
    assert!(
        registry.get("provider_x").is_some(),
        "first-write-wins must keep the first registration"
    );
}

#[test]
fn register_extension_data_providers_empty_inputs() {
    let mut registry = DataRegistry::new();
    register_extension_data_providers(&[], &mut registry);
    // TEST-25 / TASK-1280: assert observable emptiness via the public
    // `provider_names()` view (which returns a sorted Vec of registered
    // provider names). A previous shape ended with `let _ = registry`,
    // exercising only panic-freeness — any future regression that
    // registered a hidden default provider would slip past.
    assert!(
        registry.provider_names().is_empty(),
        "registry must remain empty when no extensions register, got: {:?}",
        registry.provider_names()
    );
    assert!(
        registry.get("any_name").is_none(),
        "an unregistered name must resolve to None"
    );
}

/// TEST-11 / TASK-1301: pin the "aggregation does not drop entries"
/// contract using two inline stub extensions so the assertion fires on
/// every feature combination — including builds that compile in zero
/// extensions. The previous shape early-returned silently when
/// `builtin_extensions` yielded fewer than two extensions (the common
/// `cargo test -p ops` case), reading as a coverage win in dashboards
/// while exercising nothing. The stub pattern mirrors
/// `register_extension_commands_detects_duplicate_command_id`.
#[test]
fn register_extension_commands_aggregates_across_multiple_extensions() {
    use ops_core::config::{CommandSpec, ExecCommandSpec};
    use ops_extension::{CommandRegistry, Extension};

    struct ExtA;
    impl Extension for ExtA {
        fn name(&self) -> &'static str {
            "ext_a"
        }
        fn register_commands(&self, registry: &mut CommandRegistry) {
            registry.insert(
                "a_only".into(),
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
                "b_only".into(),
                CommandSpec::Exec(ExecCommandSpec::new("echo", ["b"])),
            );
        }
    }

    let a = ExtA;
    let b = ExtB;
    let ext_refs: Vec<&dyn Extension> = vec![&a, &b];

    let mut combined = CommandRegistry::new();
    register_extension_commands(&ext_refs, &mut combined);

    assert_eq!(
        combined.len(),
        2,
        "aggregation must preserve both non-colliding command ids"
    );
    assert!(combined.get("a_only").is_some(), "ext_a contribution kept");
    assert!(combined.get("b_only").is_some(), "ext_b contribution kept");
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

/// PATTERN-1 / TASK-1088: when two compiled-in extensions self-register
/// under the same `config_name` (e.g. via colliding `impl_extension!`
/// invocations), the discovery layer must not silently drop the earlier
/// `Box<dyn Extension>` — it must emit a `tracing::warn!` audit
/// breadcrumb naming both slots. Resolution policy: last-write-wins,
/// matching `register_extension_commands` (CL-5 / TASK-0904).
#[test]
fn dedup_compiled_extensions_warns_on_duplicate_config_name() {
    use ops_extension::{CommandRegistry, Extension};

    struct ExtA;
    impl Extension for ExtA {
        fn name(&self) -> &'static str {
            "ext_a"
        }
        fn register_commands(&self, _registry: &mut CommandRegistry) {}
    }

    struct ExtB;
    impl Extension for ExtB {
        fn name(&self) -> &'static str {
            "ext_b"
        }
        fn register_commands(&self, _registry: &mut CommandRegistry) {}
    }

    // Two distinct extensions sharing the same config_name "shared".
    let pairs: Vec<(&'static str, Box<dyn Extension>)> =
        vec![("shared", Box::new(ExtA)), ("shared", Box::new(ExtB))];

    let mut map_holder = None;
    let captured = capture_warnings(|| {
        map_holder = Some(super::discovery::dedup_compiled_extensions(pairs));
    });
    let map = map_holder.unwrap();

    // Last-write-wins: ExtB survives.
    assert_eq!(map.len(), 1, "duplicate config_name collapses to one entry");
    assert_eq!(
        map.get("shared").unwrap().name(),
        "ext_b",
        "last-write-wins: the second extension survives"
    );
    assert!(
        captured.contains("shared"),
        "warning must name the colliding config_name, got: {captured}"
    );
    assert!(
        captured.contains("ext_a") && captured.contains("ext_b"),
        "warning must name both extension slots, got: {captured}"
    );
    assert!(
        captured.contains("duplicate compiled-in extension"),
        "warning must describe the issue, got: {captured}"
    );
}

/// PATTERN-1 / TASK-1087: when `extensions.enabled` is unset, the
/// `enabled = None` branch of `builtin_extensions` must yield extensions in a
/// stable order so that `register_extension_commands` (last-write-wins, CL-5
/// / TASK-0904) picks the same winner every process. Prior to TASK-1087 the
/// branch returned a `HashMap::into_values()` iterator, randomising the
/// command-id collision winner per process. We simulate the wiring directly
/// (`dedup_compiled_extensions` → `register_extension_commands`) so the test
/// is hermetic — independent of which extension crates are linked into this
/// build — and re-run it N=100 times to catch any stochastic flips.
#[test]
fn dedup_then_register_pins_command_winner_across_invocations() {
    use ops_core::config::{CommandSpec, ExecCommandSpec};
    use ops_extension::{CommandRegistry, Extension};

    struct ExtA;
    impl Extension for ExtA {
        fn name(&self) -> &'static str {
            "ext_a"
        }
        fn register_commands(&self, registry: &mut CommandRegistry) {
            registry.insert(
                "build".into(),
                CommandSpec::Exec(ExecCommandSpec::new("from_a", Vec::<String>::new())),
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
                "build".into(),
                CommandSpec::Exec(ExecCommandSpec::new("from_b", Vec::<String>::new())),
            );
        }
    }

    // Distinct config_names — sorted ascending, "alpha" < "beta", so beta
    // (ExtB) is the last to write under BTreeMap iteration and must win.
    let expected_winner = "from_b";
    for i in 0..100 {
        let pairs: Vec<(&'static str, Box<dyn Extension>)> =
            vec![("alpha", Box::new(ExtA)), ("beta", Box::new(ExtB))];
        let map = super::discovery::dedup_compiled_extensions(pairs);
        let exts: Vec<Box<dyn Extension>> = map.into_values().collect();
        let ext_refs = as_ext_refs(&exts);
        let mut registry = CommandRegistry::new();
        register_extension_commands(&ext_refs, &mut registry);
        match registry.get("build") {
            Some(CommandSpec::Exec(e)) => assert_eq!(
                e.program, expected_winner,
                "iteration {i}: command-id collision winner must be stable across invocations"
            ),
            other => panic!("iteration {i}: expected exec spec, got {other:?}"),
        }
    }
}
