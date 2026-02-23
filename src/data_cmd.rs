//! `cargo ops data` - data provider management commands.

use std::io::{self, IsTerminal};

use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, ColumnConstraint,
    ContentArrangement, Table, Width::Fixed,
};

use crate::config;
use crate::extension::{DataProviderSchema, DataRegistry};
use crate::extensions::{builtin_extensions, register_extension_data_providers};

pub fn run_data_list() -> anyhow::Result<()> {
    let config = config::load_config()?;
    let cwd = std::env::current_dir()?;
    let is_tty = io::stdout().is_terminal();

    let exts = builtin_extensions(&config, &cwd)?;
    let ext_refs: Vec<&dyn crate::extension::Extension> = exts.iter().map(|b| b.as_ref()).collect();
    let mut registry = DataRegistry::new();
    register_extension_data_providers(&ext_refs, &mut registry);

    let schemas = registry.schemas();

    if schemas.is_empty() {
        println!("No data providers registered.");
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Provider", "Description"]);

    for (name, schema) in &schemas {
        let name_cell = if is_tty {
            Cell::new(name).fg(Color::Cyan)
        } else {
            Cell::new(name)
        };

        let desc_cell = if is_tty {
            Cell::new(schema.description).fg(Color::DarkGrey)
        } else {
            Cell::new(schema.description)
        };

        table.add_row(vec![name_cell, desc_cell]);
    }

    let desc_col = table.column_mut(1).expect("description column");
    desc_col.set_constraint(ColumnConstraint::UpperBoundary(Fixed(60)));

    println!("{table}");
    Ok(())
}

pub fn run_data_info(name: &str) -> anyhow::Result<()> {
    let config = config::load_config()?;
    let cwd = std::env::current_dir()?;
    let is_tty = io::stdout().is_terminal();

    let exts = builtin_extensions(&config, &cwd)?;
    let ext_refs: Vec<&dyn crate::extension::Extension> = exts.iter().map(|b| b.as_ref()).collect();
    let mut registry = DataRegistry::new();
    register_extension_data_providers(&ext_refs, &mut registry);

    let provider = registry.get(name).ok_or_else(|| {
        anyhow::anyhow!(
            "data provider not found: {}. Run `cargo ops data list` to see available providers.",
            name
        )
    })?;

    let schema = provider.schema();

    print_provider_info(name, &schema, is_tty);
    Ok(())
}

fn print_provider_info(name: &str, schema: &DataProviderSchema, is_tty: bool) {
    println!(
        "PROVIDER: {}",
        if is_tty {
            format!("\x1b[36m{}\x1b[0m", name)
        } else {
            name.to_string()
        }
    );
    println!("{}", schema.description);
    println!();

    if schema.fields.is_empty() {
        println!("No fields documented.");
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Field", "Type", "Description"]);

    for field in &schema.fields {
        let name_cell = if is_tty {
            Cell::new(field.name).fg(Color::Yellow)
        } else {
            Cell::new(field.name)
        };

        let type_cell = if is_tty {
            Cell::new(field.type_name).fg(Color::DarkGrey)
        } else {
            Cell::new(field.type_name)
        };

        let desc_cell = if is_tty {
            Cell::new(field.description).fg(Color::White)
        } else {
            Cell::new(field.description)
        };

        table.add_row(vec![name_cell, type_cell, desc_cell]);
    }

    let desc_col = table.column_mut(2).expect("description column");
    desc_col.set_constraint(ColumnConstraint::UpperBoundary(Fixed(50)));

    println!("{table}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_data_list_succeeds() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".ops.toml"), "").unwrap();
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");

        let result = run_data_list();
        assert!(result.is_ok(), "run_data_list should succeed");
    }

    #[test]
    fn run_data_info_unknown_provider_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".ops.toml"), "").unwrap();
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");

        let result = run_data_info("nonexistent");
        assert!(
            result.is_err(),
            "run_data_info should error for unknown provider"
        );
    }
}
