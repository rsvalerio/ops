//! `cargo ops extension` - extension management commands.

use std::io::{self, IsTerminal};

use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, ColumnConstraint,
    ContentArrangement, Table, Width::Fixed,
};

use crate::config;
use crate::extension::CommandRegistry;
use crate::extensions::collect_compiled_extensions;

fn format_list(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_string()
    } else {
        items.join(", ")
    }
}

pub fn run_extension_list() -> anyhow::Result<()> {
    let config = config::load_config()?;
    let cwd = std::env::current_dir()?;
    let is_tty = io::stdout().is_terminal();

    let compiled = collect_compiled_extensions(&config, &cwd);

    if compiled.is_empty() {
        println!("No extensions compiled in.");
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        "Name",
        "Shortname",
        "Types",
        "Commands",
        "Data Provider",
        "Description",
    ]);

    for (config_name, ext) in &compiled {
        let info = ext.info();

        let mut types = Vec::new();
        if info.types.is_datasource() {
            types.push("DATASOURCE".to_string());
        }
        if info.types.is_command() {
            types.push("COMMAND".to_string());
        }

        let mut cmd_registry = CommandRegistry::new();
        ext.register_commands(&mut cmd_registry);
        let commands: Vec<String> = cmd_registry.keys().map(|s| s.to_string()).collect();

        let data_provider = info
            .data_provider_name
            .map(|s| s.to_string())
            .unwrap_or_default();

        let name_cell = if is_tty {
            Cell::new(config_name).fg(Color::Cyan)
        } else {
            Cell::new(config_name)
        };

        let desc_cell = if is_tty {
            Cell::new(info.description).fg(Color::DarkGrey)
        } else {
            Cell::new(info.description)
        };

        let data_cell = if is_tty && !data_provider.is_empty() {
            Cell::new(&data_provider).fg(Color::Green)
        } else {
            Cell::new(&data_provider)
        };

        table.add_row(vec![
            name_cell,
            Cell::new(info.shortname),
            Cell::new(format_list(&types)),
            Cell::new(format_list(&commands)),
            data_cell,
            desc_cell,
        ]);
    }

    let desc_col = table.column_mut(5).expect("description column");
    desc_col.set_constraint(ColumnConstraint::UpperBoundary(Fixed(40)));

    println!("{table}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_extension_list_succeeds() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".ops.toml"), "").unwrap();
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");

        let result = run_extension_list();
        assert!(result.is_ok(), "run_extension_list should succeed");
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
