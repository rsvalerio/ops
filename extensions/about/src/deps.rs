//! Stack-agnostic `about dependencies` subpage: per-unit dependency tree.
//!
//! Calls the `project_dependencies` data provider registered by the active stack.

use std::io::{IsTerminal, Write};

use ops_core::project_identity::ProjectDependencies;
use ops_core::style::{cyan, dim};
use ops_extension::{Context, DataRegistry};

use crate::providers::{load_or_default, warm_providers};
use crate::text_util::tty_style;

pub const PROJECT_DEPENDENCIES_PROVIDER: &str = "project_dependencies";

pub fn run_about_deps(data_registry: &DataRegistry) -> anyhow::Result<()> {
    let is_tty = std::io::stdout().is_terminal();
    run_about_deps_with(data_registry, &mut std::io::stdout(), is_tty)
}

/// READ-5/TASK-0411: `is_tty` reflects the `writer` the caller hands in.
/// See [`crate::units::run_about_units_with`] for the rationale.
pub fn run_about_deps_with(
    data_registry: &DataRegistry,
    writer: &mut dyn Write,
    is_tty: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = std::sync::Arc::new(ops_core::config::Config::default());
    let mut ctx = Context::new(config, cwd);

    warm_providers(&mut ctx, data_registry, &["duckdb", "metadata"], "deps");

    let deps: ProjectDependencies =
        load_or_default(&mut ctx, data_registry, PROJECT_DEPENDENCIES_PROVIDER)?;

    let lines = format_dependencies_section(&deps, is_tty);
    if lines.is_empty() {
        writeln!(writer, "No dependency data available.")?;
        return Ok(());
    }
    writeln!(writer, "{}", lines.join("\n"))?;
    Ok(())
}

pub fn format_dependencies_section(deps: &ProjectDependencies, is_tty: bool) -> Vec<String> {
    let mut units: Vec<&ops_core::project_identity::UnitDeps> =
        deps.units.iter().filter(|u| !u.deps.is_empty()).collect();
    if units.is_empty() {
        return vec![];
    }
    units.sort_by(|a, b| a.unit_name.cmp(&b.unit_name));

    let mut lines = vec![String::new(), "  DEPENDENCIES".to_string()];

    for unit in units {
        lines.push(String::new());
        lines.push(format!("  {}", tty_style(&unit.unit_name, cyan, is_tty)));

        // READ-5 (TASK-0591): use `split_last` so the connector choice never
        // depends on `len() - 1` over a possibly-zero-length slice. The
        // outer filter already rejects empty deps today, but a future
        // refactor that adds another filter and lets an empty slice through
        // would otherwise panic on `0usize - 1`.
        let Some((last, rest)) = unit.deps.split_last() else {
            continue;
        };
        for (name, version) in rest {
            lines.push(format!(
                "  {}",
                tty_style(
                    &format!("\u{251c}\u{2500}\u{2500} {} {}", name, version),
                    dim,
                    is_tty,
                )
            ));
        }
        let (last_name, last_version) = last;
        lines.push(format!(
            "  {}",
            tty_style(
                &format!("\u{2514}\u{2500}\u{2500} {} {}", last_name, last_version),
                dim,
                is_tty,
            )
        ));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::project_identity::UnitDeps;

    #[test]
    fn format_dependencies_section_empty() {
        assert!(format_dependencies_section(&ProjectDependencies::default(), false).is_empty());
    }

    #[test]
    fn format_dependencies_section_renders_tree() {
        let deps = ProjectDependencies {
            units: vec![
                UnitDeps {
                    unit_name: "ops-core".to_string(),
                    deps: vec![
                        ("anyhow".to_string(), "^1.0".to_string()),
                        ("serde".to_string(), "^1.0".to_string()),
                    ],
                },
                UnitDeps {
                    unit_name: "ops-cli".to_string(),
                    deps: vec![("clap".to_string(), "^4.0".to_string())],
                },
            ],
        };
        let out = format_dependencies_section(&deps, false).join("\n");
        assert!(out.contains("DEPENDENCIES"));
        assert!(out.contains("ops-cli"));
        assert!(out.contains("ops-core"));
        assert!(out.contains("\u{251c}\u{2500}\u{2500} anyhow"));
        assert!(out.contains("\u{2514}\u{2500}\u{2500} serde"));
        // Sorted alphabetically
        assert!(out.find("ops-cli").unwrap() < out.find("ops-core").unwrap());
    }

    /// READ-5/TASK-0411: when the caller declares the writer is not a TTY,
    /// the rendered output must not contain ANSI escape bytes — even if the
    /// process's stdout happens to be a real terminal at test time.
    #[test]
    fn format_dependencies_section_emits_no_ansi_when_is_tty_false() {
        let deps = ProjectDependencies {
            units: vec![UnitDeps {
                unit_name: "core".to_string(),
                deps: vec![("anyhow".to_string(), "^1.0".to_string())],
            }],
        };
        let out = format_dependencies_section(&deps, false).join("\n");
        assert!(
            !out.contains('\x1b'),
            "non-TTY writer must receive plain text: {out:?}"
        );
    }

    /// READ-5 (TASK-0591): pass a unit with empty deps directly and verify
    /// no panic. The outer filter currently skips it, but the inner loop
    /// must be safe for an empty slice on its own merits.
    #[test]
    fn format_dependencies_section_unit_with_empty_deps_does_not_panic() {
        let deps = ProjectDependencies {
            units: vec![UnitDeps {
                unit_name: "lonely".to_string(),
                deps: vec![],
            }],
        };
        let out = format_dependencies_section(&deps, false);
        assert!(out.is_empty(), "expected empty output, got: {out:?}");
    }

    #[test]
    fn format_dependencies_section_skips_empty_deps() {
        let deps = ProjectDependencies {
            units: vec![
                UnitDeps {
                    unit_name: "has-deps".to_string(),
                    deps: vec![("x".to_string(), "1".to_string())],
                },
                UnitDeps {
                    unit_name: "empty".to_string(),
                    deps: vec![],
                },
            ],
        };
        let out = format_dependencies_section(&deps, false).join("\n");
        assert!(out.contains("has-deps"));
        assert!(!out.contains("empty"));
    }
}
