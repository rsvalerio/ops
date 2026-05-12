//! `cargo ops extension` - extension management commands.

use std::io::Write;

use ops_core::style;
use ops_core::table::{Cell, Color, OpsTable};
use ops_core::text::capitalize;
use ops_extension::{CommandRegistry, DataProviderSchema};

use crate::registry::{build_data_registry_from, collect_compiled_extensions};
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
        .map(|(_, ext)| {
            audit_command_self_shadow(*ext, &mut warned);
            extension_summary(*ext)
        })
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
fn extension_summary(ext: &dyn ops_extension::Extension) -> (Vec<String>, Vec<String>) {
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
        // FN-1 / TASK-1359: in-extension duplicates surface via the audit
        // helper at the call site (`audit_command_self_shadow`); the
        // summary stays pure so callers that just want the
        // (types, commands) tuple don't have to thread a dedupe set.
        cmd_registry.keys().map(|s| s.to_string()).collect()
    } else {
        info.command_names
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    };
    (types, commands)
}

/// FN-1 / TASK-1359 + ERR-1 / TASK-0710: drain `register_commands` for
/// `ext`, emit one `tracing::warn!` per `(extension, command)` self-shadow
/// pair, deduplicating against `warned` so the list and show handlers
/// share one warning per CLI invocation (PERF-1 / TASK-1142, PATTERN-1 /
/// TASK-1291). Kept separate from [`extension_summary`] so the summary's
/// return contract is not coupled to the audit's side effects.
fn audit_command_self_shadow(ext: &dyn ops_extension::Extension, warned: &mut SelfShadowWarnedSet) {
    let info = ext.info();
    if !info.command_names.is_empty() {
        // Static accessor was used — no register_commands probe, no audit
        // signal to drain.
        return;
    }
    let mut cmd_registry = CommandRegistry::new();
    ext.register_commands(&mut cmd_registry);
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
    run_extension_show_to(
        &mut std::io::stdout(),
        config,
        name,
        crate::tty::is_stdout_tty,
    )
}

/// READ-5 / TASK-1353: writer-injected entry point for `ops extension show`.
/// Delegates to [`run_extension_show_with_tty_check`] which contains the
/// TTY/picker logic; tests construct a buffer and assert on the rendered
/// output directly without going through `std::io::stdout()`.
fn run_extension_show_to<F>(
    w: &mut dyn Write,
    config: &ops_core::config::Config,
    name: Option<&str>,
    is_tty: F,
) -> anyhow::Result<()>
where
    F: FnOnce() -> bool,
{
    run_extension_show_with_tty_check(w, config, name, is_tty)
}

fn run_extension_show_with_tty_check<F>(
    w: &mut dyn Write,
    config: &ops_core::config::Config,
    name: Option<&str>,
    is_tty: F,
) -> anyhow::Result<()>
where
    F: FnOnce() -> bool,
{
    let cwd = crate::cwd()?;
    // PERF-1 / TASK-1380: enumerate `EXTENSION_REGISTRY` exactly once per
    // `extension show` invocation. Both the picker / lookup AND the
    // schema-building path consume this same `compiled` vector — the
    // latter via `build_data_registry_from`. Previously
    // `print_extension_details` re-walked the registry inside
    // `build_data_registry`, doubling factory probes (and their I/O) plus
    // the per-slot `tracing::debug!` decline breadcrumbs.
    let compiled = collect_compiled_extensions(config, &cwd);

    // READ-7 / TASK-1351: surface "no extensions compiled in" before the
    // TTY check so a binary built with no extension features reports the
    // real cause regardless of whether stdout is attached to a terminal.
    // The TTY check is a precondition for the interactive picker; the
    // picker is only useful when there is something to pick.
    let resolved_name: String = match name {
        Some(n) => n.to_string(),
        None => {
            if compiled.is_empty() {
                anyhow::bail!("no extensions compiled in");
            }
            if !is_tty() {
                anyhow::bail!("extension show requires an interactive terminal (or pass a name)");
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

    let resolved_idx = compiled
        .iter()
        .position(|(config_name, _)| *config_name == resolved_name.as_str())
        .ok_or_else(|| {
            // PATTERN-1 / TASK-1378: sort the available list so two
            // consecutive failed `show <missing>` calls produce
            // byte-identical messages. EXTENSION_REGISTRY slot order is
            // link-time-dependent; without sorting the rendered list
            // shifts between builds and breaks bug-report skim-ability.
            let mut available: Vec<&str> = compiled.iter().map(|(n, _)| *n).collect();
            available.sort_unstable();
            anyhow::anyhow!(
                "extension not found: {}. Available: {}",
                resolved_name,
                available.join(", ")
            )
        })?;

    print_extension_details(w, resolved_idx, compiled, config, &cwd)
}

fn print_extension_details(
    w: &mut dyn Write,
    ext_idx: usize,
    compiled: Vec<(&'static str, Box<dyn ops_extension::Extension>)>,
    config: &ops_core::config::Config,
    cwd: &std::path::Path,
) -> anyhow::Result<()> {
    let table = OpsTable::new();
    let is_tty = table.is_tty();

    let (name, info, types, commands) = {
        let (config_name, boxed) = &compiled[ext_idx];
        let ext = boxed.as_ref();
        let mut warned: SelfShadowWarnedSet = SelfShadowWarnedSet::new();
        audit_command_self_shadow(ext, &mut warned);
        let summary = extension_summary(ext);
        (*config_name, ext.info(), summary.0, summary.1)
    };

    let data_provider = info
        .data_provider_name
        .map(|s| s.to_string())
        .unwrap_or_else(|| "-".to_string());

    write_object_header(w, "EXTENSION", name, info.description, is_tty)?;
    writeln!(w, "  Shortname:     {}", info.shortname)?;
    writeln!(w, "  Types:         {}", format_list(&types))?;
    writeln!(w, "  Commands:      {}", format_list(&commands))?;
    writeln!(w, "  Data provider: {}", data_provider)?;

    if let Some(provider_name) = info.data_provider_name {
        // PERF-1 / TASK-1380: build the data registry from the already-
        // collected `compiled` vector so the registry walk happens once
        // per invocation rather than twice.
        match build_data_registry_from(compiled, config, cwd) {
            Ok(registry) => {
                if let Some(provider) = registry.get(provider_name) {
                    writeln!(w)?;
                    print_provider_info(w, provider_name, &provider.schema())?;
                }
            }
            Err(e) => {
                // ERR-1 / TASK-1329 + ERR-7 / TASK-1340: surface the
                // failure on a single channel — the user-facing writer.
                // Operators see the message inline with the other
                // `extension show` output; structured-log readers don't
                // see a duplicate, and we no longer pre-stringify the
                // anyhow chain through `%format!("{e:#}")` (which
                // bypassed tracing's field-recorder escaping for newline
                // / ANSI bytes embedded in workspace paths or subprocess
                // stderr).
                writeln!(
                    w,
                    "\nschema unavailable for provider '{provider_name}': {e:#}"
                )?;
            }
        }
    }
    Ok(())
}

/// DUP-3 / TASK-1360: shared "KIND: name\ndescription\n" preamble for
/// [`print_extension_details`] and [`print_provider_info`]. Collapses two
/// byte-identical 8-line blocks into one helper so future heading-style
/// changes (e.g. bold KIND label, alignment tweak) land in one place.
fn write_object_header(
    w: &mut dyn Write,
    kind: &str,
    name: &str,
    description: &str,
    is_tty: bool,
) -> anyhow::Result<()> {
    writeln!(
        w,
        "{kind}: {}",
        if is_tty {
            style::cyan(name)
        } else {
            name.to_string()
        }
    )?;
    writeln!(w, "{description}")?;
    writeln!(w)?;
    Ok(())
}

fn print_provider_info(
    w: &mut dyn Write,
    name: &str,
    schema: &DataProviderSchema,
) -> anyhow::Result<()> {
    let table = OpsTable::new();
    write_object_header(w, "PROVIDER", name, schema.description, table.is_tty())?;

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
        let mut buf = Vec::new();
        let result = run_extension_show_with_tty_check(&mut buf, &config, None, || false);
        assert!(
            result.is_err(),
            "run_extension_show should fail without TTY when no name given"
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("interactive terminal"));
    }

    /// READ-7 / TASK-1351: when no extensions are compiled in, the
    /// "no extensions compiled in" diagnosis must win over the TTY
    /// gate. Pre-fix, a non-TTY invocation got the misleading
    /// "interactive terminal" error and the operator never saw the
    /// real cause until they re-ran interactively.
    #[test]
    fn run_extension_show_no_extensions_wins_over_no_tty() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[extensions]
enabled = []
"#,
        );
        let config = ops_core::config::load_config_or_default("test");
        let mut buf = Vec::new();
        // Stub `compiled` would be empty under `enabled = []` only for
        // builtin_extensions; collect_compiled_extensions still returns
        // all compiled-in. Under default features extensions are
        // compiled in, so this test instead asserts that the no-TTY
        // path does NOT erroneously claim "no extensions compiled in"
        // when extensions actually are compiled in.
        let result = run_extension_show_with_tty_check(&mut buf, &config, None, || false);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        // When extensions ARE compiled in, the TTY error is the right
        // one; when they aren't, the no-extensions error is. The
        // ordering invariant is what we pin here: no-TTY error must
        // never appear if extensions are missing.
        let compiled_count = collect_compiled_extensions(&config, &crate::cwd().unwrap()).len();
        if compiled_count == 0 {
            assert!(
                err.contains("no extensions compiled in"),
                "no-extensions diagnosis must win, got: {err}"
            );
        } else {
            assert!(
                err.contains("interactive terminal"),
                "with extensions present, TTY error is the correct one, got: {err}"
            );
        }
    }

    /// READ-5 / TASK-1353: happy-path render of `ops extension show <name>`
    /// must be observable through an injected writer (no stdout
    /// interception). Pre-fix, `run_extension_show_with_tty_check`
    /// constructed `let mut w = std::io::stdout()` internally so the
    /// EXTENSION header / field rows could not be asserted from tests.
    #[test]
    #[cfg(feature = "stack-rust")]
    fn run_extension_show_to_writes_extension_header_to_buffer() {
        let (_dir, _guard) = crate::test_utils::with_temp_config("");
        let config = ops_core::config::load_config_or_default("test");
        let mut buf = Vec::new();
        run_extension_show_to(&mut buf, &config, Some("tools"), || false)
            .expect("show should succeed for compiled-in extension");
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("EXTENSION:") && output.contains("tools"),
            "captured buffer must contain the EXTENSION header, got: {output}"
        );
        assert!(
            output.contains("Shortname:"),
            "captured buffer must contain a field row, got: {output}"
        );
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
            let (types, _commands) = extension_summary(ext.as_ref());
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
        let idx = compiled
            .iter()
            .position(|(n, _)| *n == "git")
            .expect("git extension must be compiled in");
        let mut buf = Vec::new();
        print_extension_details(&mut buf, idx, compiled, &config, &cwd)
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

    /// ERR-1 / TASK-1329 + ERR-7 / TASK-1340: the schema-build failure
    /// is surfaced on the user-facing writer only. The structured
    /// tracing channel must NOT emit a duplicate, and the user message
    /// no longer pre-stringifies the anyhow chain through
    /// `%format!("{e:#}")` — paths or stderr embedded in the chain
    /// reach the writer once, untransformed, while the tracing field
    /// recorder is no longer bypassed.
    #[test]
    fn print_extension_details_registry_error_emits_only_to_writer() {
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
        let Some(idx) = compiled.iter().position(|(n, _)| *n == "git") else {
            return; // stack-rust not compiled in; nothing to assert
        };
        let mut buf = Vec::new();
        let logs = crate::test_utils::capture_warnings(|| {
            print_extension_details(&mut buf, idx, compiled, &config, &cwd)
                .expect("should not propagate");
        });
        assert!(
            !logs.contains("could not build data registry for schema display"),
            "tracing channel must not duplicate the writer message, got: {logs}"
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
            audit_command_self_shadow(&ext, &mut warned);
            let (_, commands) = extension_summary(&ext);
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
                audit_command_self_shadow(ext, &mut warned);
                audit_command_self_shadow(ext, &mut warned);
                audit_command_self_shadow(ext, &mut warned);
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
