//! `cargo ops extension` - extension management commands.

use std::io::Write;

use ops_core::style;
use ops_core::table::{Cell, Color, OpsTable};
use ops_core::text::capitalize;
use ops_extension::{CommandRegistry, DataProviderSchema};

use crate::registry::{build_data_registry, collect_compiled_extensions};
use crate::tty::SelectOption;

fn format_list(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_string()
    } else {
        items.join(", ")
    }
}

pub fn run_extension_list(config: &ops_core::config::Config) -> anyhow::Result<()> {
    run_extension_list_to(config, &mut std::io::stdout())
}

fn run_extension_list_to(
    config: &ops_core::config::Config,
    w: &mut dyn Write,
) -> anyhow::Result<()> {
    let cwd = crate::cwd()?;

    let compiled = collect_compiled_extensions(config, &cwd);

    if compiled.is_empty() {
        writeln!(w, "No extensions compiled in.")?;
        return Ok(());
    }

    // Partition extensions: generic (no stack) first, then grouped by stack.
    let mut generic: Vec<(&str, &dyn ops_extension::Extension)> = Vec::new();
    let mut by_stack: indexmap::IndexMap<
        ops_core::stack::Stack,
        Vec<(&str, &dyn ops_extension::Extension)>,
    > = indexmap::IndexMap::new();

    for (config_name, ext) in &compiled {
        let entry = (*config_name, ext.as_ref());
        match ext.stack() {
            None => generic.push(entry),
            Some(s) => by_stack.entry(s).or_default().push(entry),
        }
    }

    if !generic.is_empty() {
        writeln!(w, "Generic extensions:")?;
        write_extension_table(w, &generic)?;
    }

    for (stack, exts) in &by_stack {
        if !generic.is_empty() || by_stack.len() > 1 {
            writeln!(w)?;
        }
        writeln!(w, "{} extensions:", capitalize(stack.as_str()))?;
        write_extension_table(w, exts)?;
    }

    Ok(())
}

/// Maximum display width for the description column in extension/provider tables.
const DESC_TRUNCATION_WIDTH: u16 = 40;
/// Wider truncation budget for the narrower provider-field table.
const PROVIDER_DESC_TRUNCATION_WIDTH: u16 = 50;

/// DUP-3 / TASK-1118: shared helper for the "Description" column lookup used
/// by both `write_extension_table` and `print_provider_info`.
fn description_col(headers: &[&str]) -> usize {
    headers
        .iter()
        .position(|&h| h == "Description")
        .expect("Description header must exist")
}

fn write_extension_table(
    w: &mut dyn Write,
    exts: &[(&str, &dyn ops_extension::Extension)],
) -> anyhow::Result<()> {
    let headers = [
        "Name",
        "Shortname",
        "Types",
        "Commands",
        "Data Provider",
        "Description",
    ];
    let desc_col = description_col(&headers);

    let mut table = OpsTable::new();
    table.set_header(headers.to_vec());

    // PERF-1 / TASK-0859: hoist `extension_summary` out of the per-row loop.
    // It can perform I/O (legacy extensions without static `command_names`
    // run `register_commands`) and was previously called once per render
    // pass — N rows ⇒ N register_commands invocations. Compute once per
    // extension up front and feed the precomputed summary into each row.
    //
    // PATTERN-1 / TASK-1291: own the self-shadow dedupe set locally so the
    // "warn once per CLI invocation" guarantee scopes to *this* handler
    // call, not the process.
    let mut warned: SelfShadowWarnedSet = SelfShadowWarnedSet::new();
    let summaries: Vec<(Vec<String>, Vec<String>)> = exts
        .iter()
        .map(|(_, ext)| extension_summary(*ext, &mut warned))
        .collect();

    for ((config_name, ext), summary) in exts.iter().zip(summaries.iter()) {
        table.add_row(build_extension_row(&table, config_name, *ext, summary));
    }

    table.set_max_width(desc_col, DESC_TRUNCATION_WIDTH);

    writeln!(w, "{table}")?;
    Ok(())
}

/// Collect extension type flags and registered command names.
///
/// PATTERN-1 / TASK-1291 + TEST-1 / TASK-1289: dedupe-state for self-shadow
/// warnings, scoped to one top-level CLI handler invocation. Owned by the
/// caller (`run_extension_list_to` / `print_extension_details`) so two
/// in-process invocations — or two test runs in the same binary — emit
/// independently. Previously the dedupe set lived in a process-global
/// `OnceLock<Mutex<HashSet>>`, which:
///
/// 1. coupled production logging to test isolation (TEST-1), and
/// 2. mismatched the docstring claim that dedup is "per CLI invocation"
///    (PATTERN-1) — the set was actually per *process*, growing unbounded
///    over the binary's lifetime.
///
/// The set is an in-handler cache only; passing it explicitly lets callers
/// construct fresh state per invocation without unsafe `reset_for_tests`
/// dances.
pub(crate) type SelfShadowWarnedSet = std::collections::HashSet<(String, String)>;

/// PERF-1 / TASK-0513: prefer the static `command_names()` accessor (set
/// via `impl_extension! { command_names: &[..] }`) so list/show paths do
/// not re-run `register_commands` (which can perform I/O for some
/// extensions). Falls back to a one-shot `register_commands` only when an
/// extension does not expose a static list, preserving the previous
/// behaviour for legacy extensions.
///
/// PERF-1 / TASK-1142 + PATTERN-1 / TASK-1291: dedupe self-shadow warnings
/// to a single emission per `(extension, command)` pair *per CLI handler
/// invocation*. The caller owns `warned` so two in-process invocations get
/// independent dedupe state.
fn extension_summary(
    ext: &dyn ops_extension::Extension,
    warned: &mut SelfShadowWarnedSet,
) -> (Vec<String>, Vec<String>) {
    let info = ext.info();
    let mut types = Vec::new();
    if info.types.is_datasource() {
        types.push("DATASOURCE".to_string());
    }
    if info.types.is_command() {
        types.push("COMMAND".to_string());
    }
    let commands: Vec<String> = if info.command_names.is_empty() {
        let mut cmd_registry = CommandRegistry::new();
        ext.register_commands(&mut cmd_registry);
        // ERR-1 / TASK-0710: surface within-extension self-shadowing on the
        // operator-facing `extension show` / `extension list` paths. Without
        // this, a regression that re-inserts the same command id is silently
        // collapsed into a single entry on the rendered list while the
        // runner-wiring path (`register_extension_commands`) emits a WARN.
        // Mirror that warning here so operators reading `ops extension show`
        // see the same diagnostic.
        //
        // PERF-1 / TASK-1142: emit at most once per (extension, command)
        // pair per CLI invocation — both list and show handlers route
        // through here and a future call site would compound the warns.
        for dup in cmd_registry.take_duplicate_inserts() {
            let key = (ext.name().to_string(), dup.to_string());
            if warned.insert(key) {
                tracing::warn!(
                    command = %dup,
                    extension = ext.name(),
                    "extension registered the same command id more than once; the later registration shadows the earlier within this extension"
                );
            }
        }
        cmd_registry.keys().map(|s| s.to_string()).collect()
    } else {
        info.command_names
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    };
    (types, commands)
}

/// Build a table row for a single extension.
///
/// PERF-1 / TASK-0859: takes a precomputed `summary` (types + command
/// names) so the caller hoists the potentially-I/O `extension_summary`
/// call out of a per-row loop.
fn build_extension_row(
    table: &OpsTable,
    config_name: &str,
    ext: &dyn ops_extension::Extension,
    summary: &(Vec<String>, Vec<String>),
) -> Vec<Cell> {
    let info = ext.info();
    let (types, commands) = summary;

    let data_provider = info
        .data_provider_name
        .map(|s| s.to_string())
        .unwrap_or_default();

    let data_cell = if table.is_tty() && !data_provider.is_empty() {
        Cell::new(&data_provider).fg(Color::Green)
    } else {
        Cell::new(&data_provider)
    };

    vec![
        table.cell(config_name, Color::Cyan),
        Cell::new(info.shortname),
        Cell::new(format_list(types)),
        Cell::new(format_list(commands)),
        data_cell,
        table.cell(info.description, Color::DarkGrey),
    ]
}

pub fn run_extension_show(
    config: &ops_core::config::Config,
    name: Option<&str>,
) -> anyhow::Result<()> {
    run_extension_show_with_tty_check(config, name, crate::tty::is_stdout_tty)
}

fn run_extension_show_with_tty_check<F>(
    config: &ops_core::config::Config,
    name: Option<&str>,
    is_tty: F,
) -> anyhow::Result<()>
where
    F: FnOnce() -> bool,
{
    let cwd = crate::cwd()?;
    let compiled = collect_compiled_extensions(config, &cwd);

    let resolved_name: String = match name {
        Some(n) => n.to_string(),
        None => {
            if !is_tty() {
                anyhow::bail!("extension show requires an interactive terminal (or pass a name)");
            }

            if compiled.is_empty() {
                anyhow::bail!("no extensions compiled in");
            }

            let options: Vec<SelectOption> = compiled
                .iter()
                .map(|(config_name, ext)| SelectOption {
                    name: config_name.to_string(),
                    description: ext.info().description.to_string(),
                })
                .collect();

            let selected = inquire::Select::new("Select an extension:", options).prompt()?;
            selected.name
        }
    };

    let (_, ext) = compiled
        .iter()
        .find(|(config_name, _)| *config_name == resolved_name.as_str())
        .ok_or_else(|| {
            let available: Vec<&str> = compiled.iter().map(|(n, _)| *n).collect();
            anyhow::anyhow!(
                "extension not found: {}. Available: {}",
                resolved_name,
                available.join(", ")
            )
        })?;

    print_extension_details(
        &mut std::io::stdout(),
        &resolved_name,
        ext.as_ref(),
        config,
        &cwd,
    )
}

fn print_extension_details(
    w: &mut dyn Write,
    name: &str,
    ext: &dyn ops_extension::Extension,
    config: &ops_core::config::Config,
    cwd: &std::path::Path,
) -> anyhow::Result<()> {
    let info = ext.info();
    let table = OpsTable::new();
    let is_tty = table.is_tty();
    let mut warned: SelfShadowWarnedSet = SelfShadowWarnedSet::new();
    let (types, commands) = extension_summary(ext, &mut warned);

    let data_provider = info
        .data_provider_name
        .map(|s| s.to_string())
        .unwrap_or_else(|| "-".to_string());

    writeln!(
        w,
        "EXTENSION: {}",
        if is_tty {
            style::cyan(name)
        } else {
            name.to_string()
        }
    )?;
    writeln!(w, "{}", info.description)?;
    writeln!(w)?;
    writeln!(w, "  Shortname:     {}", info.shortname)?;
    writeln!(w, "  Types:         {}", format_list(&types))?;
    writeln!(w, "  Commands:      {}", format_list(&commands))?;
    writeln!(w, "  Data provider: {}", data_provider)?;

    if let Some(provider_name) = info.data_provider_name {
        match build_data_registry(config, cwd) {
            Ok(registry) => {
                if let Some(provider) = registry.get(provider_name) {
                    writeln!(w)?;
                    print_provider_info(w, provider_name, &provider.schema())?;
                }
            }
            Err(e) => {
                let msg = format!("schema unavailable for provider '{provider_name}': {e:#}");
                tracing::warn!(error = %format!("{e:#}"), provider = provider_name, "could not build data registry for schema display");
                writeln!(w, "\n{msg}")?;
            }
        }
    }
    Ok(())
}

fn print_provider_info(
    w: &mut dyn Write,
    name: &str,
    schema: &DataProviderSchema,
) -> anyhow::Result<()> {
    let table = OpsTable::new();
    writeln!(
        w,
        "PROVIDER: {}",
        if table.is_tty() {
            style::cyan(name)
        } else {
            name.to_string()
        }
    )?;
    writeln!(w, "{}", schema.description)?;
    writeln!(w)?;

    if schema.fields.is_empty() {
        writeln!(w, "No fields documented.")?;
        return Ok(());
    }

    let provider_headers = ["Field", "Type", "Description"];
    let desc_col = description_col(&provider_headers);

    let mut table = OpsTable::new();
    table.set_header(provider_headers.to_vec());

    for field in &schema.fields {
        let row = vec![
            table.cell(field.name, Color::Yellow),
            table.cell(field.type_name, Color::DarkGrey),
            table.cell(field.description, Color::White),
        ];
        table.add_row(row);
    }

    table.set_max_width(desc_col, PROVIDER_DESC_TRUNCATION_WIDTH);

    writeln!(w, "{table}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_extension_list_outputs_extensions() {
        let (_dir, _guard) = crate::test_utils::with_temp_config("");

        let config = ops_core::config::load_config_or_default("test");
        let mut buf = Vec::new();
        run_extension_list_to(&config, &mut buf).expect("should succeed");
        let output = String::from_utf8(buf).unwrap();
        // Should contain table headers or extension names
        assert!(
            output.contains("Name") || output.contains("extensions"),
            "should produce meaningful output: {output}"
        );
    }

    #[test]
    fn run_extension_show_unknown_returns_error() {
        let (_dir, _guard) = crate::test_utils::with_temp_config("");

        let config = ops_core::config::load_config_or_default("test");
        let result = run_extension_show(&config, Some("nonexistent"));
        assert!(
            result.is_err(),
            "run_extension_show should error for unknown extension"
        );
        let err = result.unwrap_err().to_string();
        assert!(err.contains("nonexistent"), "error should mention the name");
    }

    #[test]
    #[cfg(feature = "stack-rust")]
    fn run_extension_show_tools_succeeds() {
        let (_dir, _guard) = crate::test_utils::with_temp_config("");

        let config = ops_core::config::load_config_or_default("test");
        let result = run_extension_show(&config, Some("tools"));
        assert!(
            result.is_ok(),
            "run_extension_show should succeed for 'tools' (requires stack-rust)"
        );
    }

    #[test]
    fn run_extension_show_no_tty_returns_error() {
        let (_dir, _guard) = crate::test_utils::with_temp_config("");

        let config = ops_core::config::load_config_or_default("test");
        let result = run_extension_show_with_tty_check(&config, None, || false);
        assert!(
            result.is_err(),
            "run_extension_show should fail without TTY when no name given"
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("interactive terminal"));
    }

    // -- print_provider_info --

    #[test]
    fn print_provider_info_with_fields() {
        let schema = DataProviderSchema::new(
            "Test provider",
            vec![
                ops_extension::DataField::new("field_a", "String", "First field"),
                ops_extension::DataField::new("field_b", "Vec<u32>", "Second field"),
            ],
        );
        let mut buf = Vec::new();
        print_provider_info(&mut buf, "test_provider", &schema).expect("should succeed");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("test_provider") || output.contains("PROVIDER"));
        assert!(output.contains("Test provider"));
        assert!(output.contains("field_a"));
        assert!(output.contains("field_b"));
    }

    #[test]
    fn print_provider_info_empty_fields() {
        let schema = DataProviderSchema::new("Empty provider", vec![]);
        let mut buf = Vec::new();
        print_provider_info(&mut buf, "empty", &schema).expect("should succeed");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("No fields documented"));
    }

    // -- extension_summary --

    #[cfg(feature = "stack-rust")]
    #[test]
    fn extension_summary_returns_types_and_commands() {
        let config = ops_core::config::Config::empty();
        let cwd = std::path::Path::new(".");
        let compiled = collect_compiled_extensions(&config, cwd);
        if let Some((_, ext)) = compiled.first() {
            let mut warned: SelfShadowWarnedSet = SelfShadowWarnedSet::new();
            let (types, _commands) = extension_summary(ext.as_ref(), &mut warned);
            assert!(!types.is_empty(), "extension should have at least one type");
        }
    }

    // -- run_extension_list edge cases --

    #[test]
    fn run_extension_list_with_no_extensions() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[extensions]
enabled = []
"#,
        );
        // With empty enabled list, collect_compiled_extensions still returns all,
        // but the list output should still work
        let config = ops_core::config::load_config_or_default("test");
        let mut buf = Vec::new();
        let result = run_extension_list_to(&config, &mut buf);
        assert!(result.is_ok());
    }

    #[test]
    fn print_extension_details_surfaces_registry_build_error() {
        // ERR-1: when build_data_registry fails, the user must see a visible
        // note instead of a debug log they will not have enabled.
        let config = ops_core::config::Config {
            extensions: ops_core::config::ExtensionConfig {
                enabled: Some(vec![
                    "git".to_string(),
                    "definitely-not-a-real-extension-xyz".to_string(),
                ]),
            },
            ..ops_core::config::Config::empty()
        };
        let cwd = std::path::PathBuf::from(".");
        let compiled = collect_compiled_extensions(&config, &cwd);
        let entry = compiled
            .iter()
            .find(|(n, _)| *n == "git")
            .expect("git extension must be compiled in");
        let mut buf = Vec::new();
        print_extension_details(&mut buf, "git", entry.1.as_ref(), &config, &cwd)
            .expect("should not propagate");
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("schema unavailable"),
            "expected user-visible schema note, got: {output}"
        );
        assert!(
            output.contains("definitely-not-a-real-extension-xyz"),
            "expected error cause in output, got: {output}"
        );
    }

    /// ERR-1 / TASK-0710: an extension that registers the same command id
    /// twice must surface a WARN through `extension_summary`, mirroring the
    /// runner-wiring path's behaviour. Without this, the operator-facing
    /// `extension show`/`extension list` paths silently collapsed duplicate
    /// inserts and the diagnostic only fired on the runner path.
    #[test]
    fn extension_summary_warns_on_self_shadow() {
        use ops_core::config::{CommandSpec, ExecCommandSpec};
        use ops_extension::{CommandRegistry, Extension};

        struct DoubleRegisterExt;
        impl Extension for DoubleRegisterExt {
            fn name(&self) -> &'static str {
                "double_register_summary"
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
        let captured = crate::test_utils::capture_warnings(|| {
            let mut warned: SelfShadowWarnedSet = SelfShadowWarnedSet::new();
            let (_, commands) = extension_summary(&ext, &mut warned);
            assert_eq!(commands, vec!["lint".to_string()]);
        });
        assert!(
            captured.contains("double_register_summary") && captured.contains("lint"),
            "self-shadow warning must name extension and command id, got: {captured}"
        );
    }

    /// PERF-1 / TASK-1142 + TEST-1 / TASK-1289 + PATTERN-1 / TASK-1291:
    /// emit at most one self-shadow WARN per `(extension, command)` pair
    /// per CLI handler invocation. Both list and show handlers fan
    /// through `extension_summary`; a follow-up call site would compound
    /// the warns. The dedupe set is now owned by the caller (one set per
    /// handler invocation), so two distinct invocations in the same
    /// process emit independently — the pre-fix shape relied on a
    /// process-global `OnceLock<Mutex<HashSet>>` which made this test
    /// order-dependent and impossible to repeat within one binary.
    #[test]
    fn extension_summary_warn_is_dedup_per_cli_invocation() {
        use ops_core::config::{CommandSpec, ExecCommandSpec};
        use ops_extension::{CommandRegistry, Extension};

        struct DupExt;
        impl Extension for DupExt {
            fn name(&self) -> &'static str {
                "dedupe_warn_ext"
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

        fn run_one_invocation_and_count_warns(ext: &DupExt) -> usize {
            crate::test_utils::capture_warnings(|| {
                // Mirror `extension list` then `extension show` calling on
                // the same compiled extension within one CLI invocation.
                let mut warned: SelfShadowWarnedSet = SelfShadowWarnedSet::new();
                let _ = extension_summary(ext, &mut warned);
                let _ = extension_summary(ext, &mut warned);
                let _ = extension_summary(ext, &mut warned);
            })
            .matches("dedupe_warn_ext")
            .count()
        }

        let ext = DupExt;
        // First invocation: dedupe collapses three calls into one warn.
        assert_eq!(
            run_one_invocation_and_count_warns(&ext),
            1,
            "self-shadow warning must be emitted exactly once per CLI invocation"
        );
        // Second invocation in the same process: a fresh warned-set means
        // the warn fires again, exactly once.
        assert_eq!(
            run_one_invocation_and_count_warns(&ext),
            1,
            "two distinct invocations must dedupe independently"
        );
    }

    #[test]
    fn format_list_empty_returns_dash() {
        let items: Vec<String> = vec![];
        assert_eq!(format_list(&items), "-");
    }

    #[test]
    fn format_list_single_item() {
        let items = vec!["DATASOURCE".to_string()];
        assert_eq!(format_list(&items), "DATASOURCE");
    }

    #[test]
    fn format_list_multiple_items() {
        let items = vec!["DATASOURCE".to_string(), "COMMAND".to_string()];
        assert_eq!(format_list(&items), "DATASOURCE, COMMAND");
    }
}
