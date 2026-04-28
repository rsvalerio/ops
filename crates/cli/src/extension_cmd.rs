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

pub fn run_extension_list() -> anyhow::Result<()> {
    run_extension_list_to(&mut std::io::stdout())
}

fn run_extension_list_to(w: &mut dyn Write) -> anyhow::Result<()> {
    let (config, cwd) = crate::load_config_and_cwd()?;

    let compiled = collect_compiled_extensions(&config, &cwd);

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

fn write_extension_table(
    w: &mut dyn Write,
    exts: &[(&str, &dyn ops_extension::Extension)],
) -> anyhow::Result<()> {
    let mut table = OpsTable::new();
    table.set_header(vec![
        "Name",
        "Shortname",
        "Types",
        "Commands",
        "Data Provider",
        "Description",
    ]);

    for (config_name, ext) in exts {
        table.add_row(build_extension_row(&table, config_name, *ext));
    }

    table.set_max_width(5, 40);

    writeln!(w, "{table}")?;
    Ok(())
}

/// Collect extension type flags and registered command names.
///
/// PERF-1 / TASK-0513: prefer the static `command_names()` accessor (set
/// via `impl_extension! { command_names: &[..] }`) so list/show paths do
/// not re-run `register_commands` (which can perform I/O for some
/// extensions). Falls back to a one-shot `register_commands` only when an
/// extension does not expose a static list, preserving the previous
/// behaviour for legacy extensions.
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
fn build_extension_row(
    table: &OpsTable,
    config_name: &str,
    ext: &dyn ops_extension::Extension,
) -> Vec<Cell> {
    let info = ext.info();
    let (types, commands) = extension_summary(ext);

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
        Cell::new(format_list(&types)),
        Cell::new(format_list(&commands)),
        data_cell,
        table.cell(info.description, Color::DarkGrey),
    ]
}

pub fn run_extension_show(name: Option<&str>) -> anyhow::Result<()> {
    run_extension_show_with_tty_check(name, crate::tty::is_stdout_tty)
}

fn run_extension_show_with_tty_check<F>(name: Option<&str>, is_tty: F) -> anyhow::Result<()>
where
    F: FnOnce() -> bool,
{
    let (config, cwd) = crate::load_config_and_cwd()?;
    let compiled = collect_compiled_extensions(&config, &cwd);

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
        &config,
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
    let (types, commands) = extension_summary(ext);

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

    let mut table = OpsTable::new();
    table.set_header(vec!["Field", "Type", "Description"]);

    for field in &schema.fields {
        let row = vec![
            table.cell(field.name, Color::Yellow),
            table.cell(field.type_name, Color::DarkGrey),
            table.cell(field.description, Color::White),
        ];
        table.add_row(row);
    }

    table.set_max_width(2, 50);

    writeln!(w, "{table}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_extension_list_outputs_extensions() {
        let (_dir, _guard) = crate::test_utils::with_temp_config("");

        let mut buf = Vec::new();
        run_extension_list_to(&mut buf).expect("should succeed");
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

        let result = run_extension_show(Some("nonexistent"));
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

        let result = run_extension_show(Some("tools"));
        assert!(
            result.is_ok(),
            "run_extension_show should succeed for 'tools' (requires stack-rust)"
        );
    }

    #[test]
    fn run_extension_show_no_tty_returns_error() {
        let (_dir, _guard) = crate::test_utils::with_temp_config("");

        let result = run_extension_show_with_tty_check(None, || false);
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
        let config = ops_core::config::Config::default();
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
        let mut buf = Vec::new();
        let result = run_extension_list_to(&mut buf);
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
            ..ops_core::config::Config::default()
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
