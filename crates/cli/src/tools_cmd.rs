//! CLI handler for `cargo ops tools` subcommands.

use indexmap::IndexMap;
use std::io::Write;
use std::process::ExitCode;

use ops_core::config::load_config;
use ops_core::style::{cyan, dim, green, red};
use ops_tools::{collect_tools, install_tool, ToolInfo, ToolSource, ToolSpec, ToolStatus};

fn load_tools() -> anyhow::Result<Vec<ToolInfo>> {
    let config = load_config()?;
    Ok(collect_tools(&config.tools))
}

pub fn run_tools_list() -> anyhow::Result<()> {
    run_tools_list_to(&mut std::io::stdout())
}

fn run_tools_list_to(w: &mut dyn Write) -> anyhow::Result<()> {
    let tools = load_tools()?;

    if tools.is_empty() {
        writeln!(w, "No tools configured in .ops.toml")?;
        return Ok(());
    }

    writeln!(w, "Tools configured: {}\n", tools.len())?;

    let max_name_len = tools.iter().map(|t| t.name.len()).max().unwrap_or(0);

    for tool in &tools {
        let status_icon = match tool.status {
            ToolStatus::Installed => green("✓"),
            ToolStatus::NotInstalled => red("✗"),
            ToolStatus::Unknown => dim("?"),
        };

        let status_text = match tool.status {
            ToolStatus::Installed => "",
            ToolStatus::NotInstalled => " (NOT INSTALLED)",
            ToolStatus::Unknown => " (UNKNOWN)",
        };

        let padded_name = format!("{:width$}", tool.name, width = max_name_len);
        writeln!(
            w,
            "  {} {}  {}{}",
            status_icon,
            cyan(&padded_name),
            dim(&tool.description),
            dim(status_text)
        )?;
    }

    Ok(())
}

pub fn run_tools_check() -> anyhow::Result<ExitCode> {
    run_tools_check_to(&mut std::io::stdout(), &mut std::io::stderr())
}

fn run_tools_check_to(w: &mut dyn Write, err: &mut dyn Write) -> anyhow::Result<ExitCode> {
    let tools = load_tools()?;

    if tools.is_empty() {
        writeln!(w, "No tools configured")?;
        return Ok(ExitCode::SUCCESS);
    }

    let missing: Vec<&ToolInfo> = tools
        .iter()
        .filter(|t| t.status == ToolStatus::NotInstalled)
        .collect();

    if missing.is_empty() {
        writeln!(w, "All {} tool(s) installed", tools.len())?;
        Ok(ExitCode::SUCCESS)
    } else {
        writeln!(err, "Missing tools:")?;
        for tool in &missing {
            writeln!(err, "  - {}", tool.name)?;
        }
        Ok(ExitCode::FAILURE)
    }
}

pub fn run_tools_install(name: Option<&str>) -> anyhow::Result<ExitCode> {
    run_tools_install_to(name, &mut std::io::stdout(), &mut std::io::stderr())
}

/// Returns the shell command description for installing a tool, or None for system tools.
pub(crate) fn install_command_description(tool: &ToolInfo, spec: &ToolSpec) -> Option<String> {
    if tool.has_rustup_component {
        return spec
            .rustup_component()
            .map(|c| format!("rustup component add {}", c));
    }
    match spec.source() {
        ToolSource::Cargo => {
            let cmd = if let Some(pkg) = spec.package() {
                format!("cargo install {} --bin {}", pkg, tool.name)
            } else {
                format!("cargo install {}", tool.name)
            };
            Some(cmd)
        }
        ToolSource::System => None,
    }
}

fn run_tools_install_to(
    name: Option<&str>,
    w: &mut dyn Write,
    err: &mut dyn Write,
) -> anyhow::Result<ExitCode> {
    let config = load_config()?;

    let tools_to_install: IndexMap<String, ToolSpec> = if let Some(tool_name) = name {
        let spec = config
            .tools
            .get(tool_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("tool not found: {}", tool_name))?;
        [(tool_name.to_string(), spec)].into_iter().collect()
    } else {
        config
            .tools
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    };

    let tools = collect_tools(&tools_to_install);

    let missing: Vec<&ToolInfo> = tools
        .iter()
        .filter(|t| t.status == ToolStatus::NotInstalled)
        .collect();

    if missing.is_empty() {
        writeln!(w, "All tools already installed")?;
        return Ok(ExitCode::SUCCESS);
    }

    writeln!(w, "Installing {} missing tool(s)...\n", missing.len())?;

    let mut installed = 0;
    let mut failed = 0;

    for tool in &missing {
        let Some(spec) = tools_to_install.get(&tool.name) else {
            writeln!(
                err,
                "  {} internal error: spec not found for {}",
                red("!"),
                tool.name
            )?;
            failed += 1;
            continue;
        };
        writeln!(w, "  {}", cyan(&tool.name))?;

        let Some(cmd_desc) = install_command_description(tool, spec) else {
            writeln!(w, "    {}", dim("Skipping (system tool)"))?;
            continue;
        };
        writeln!(w, "    Running: {}", cmd_desc)?;

        match install_tool(&tool.name, spec) {
            Ok(()) => {
                writeln!(w, "    {}", green("✓ Installed"))?;
                installed += 1;
            }
            Err(e) => {
                writeln!(err, "    {} {}", red("✗ Failed:"), e)?;
                failed += 1;
            }
        }
    }

    writeln!(w)?;
    if failed > 0 {
        writeln!(
            w,
            "Done: {} installed, {} failed, {} already present",
            installed,
            failed,
            tools.len() - missing.len()
        )?;
        Ok(ExitCode::FAILURE)
    } else {
        writeln!(
            w,
            "Done: {} installed, {} already present",
            installed,
            tools.len() - missing.len()
        )?;
        Ok(ExitCode::SUCCESS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_tools::ExtendedToolSpec;

    // -- install_command_description --

    fn tool_info(name: &str, status: ToolStatus, has_rustup: bool) -> ToolInfo {
        ToolInfo {
            name: name.to_string(),
            description: "desc".to_string(),
            status,
            has_rustup_component: has_rustup,
        }
    }

    #[test]
    fn install_cmd_desc_cargo_simple() {
        let tool = tool_info("cargo-nextest", ToolStatus::NotInstalled, false);
        let spec = ToolSpec::Simple("A better test runner".to_string());
        let desc = install_command_description(&tool, &spec);
        assert_eq!(desc, Some("cargo install cargo-nextest".to_string()));
    }

    #[test]
    fn install_cmd_desc_cargo_with_package() {
        let tool = tool_info("cargo-llvm-cov", ToolStatus::NotInstalled, false);
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "Code coverage".to_string(),
            rustup_component: None,
            package: Some("cargo-llvm-cov".to_string()),
            source: ToolSource::Cargo,
        });
        let desc = install_command_description(&tool, &spec);
        assert_eq!(
            desc,
            Some("cargo install cargo-llvm-cov --bin cargo-llvm-cov".to_string())
        );
    }

    #[test]
    fn install_cmd_desc_rustup_component() {
        let tool = tool_info("cargo-clippy", ToolStatus::NotInstalled, true);
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "Lints".to_string(),
            rustup_component: Some("clippy".to_string()),
            package: None,
            source: ToolSource::Cargo,
        });
        let desc = install_command_description(&tool, &spec);
        assert_eq!(desc, Some("rustup component add clippy".to_string()));
    }

    #[test]
    fn install_cmd_desc_system_returns_none() {
        let tool = tool_info("git", ToolStatus::NotInstalled, false);
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "Version control".to_string(),
            rustup_component: None,
            package: None,
            source: ToolSource::System,
        });
        let desc = install_command_description(&tool, &spec);
        assert!(desc.is_none());
    }

    #[test]
    fn install_cmd_desc_rustup_takes_priority_over_source() {
        let tool = tool_info("llvm-cov", ToolStatus::NotInstalled, true);
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "Coverage".to_string(),
            rustup_component: Some("llvm-tools-preview".to_string()),
            package: Some("cargo-llvm-cov".to_string()),
            source: ToolSource::Cargo,
        });
        let desc = install_command_description(&tool, &spec);
        assert_eq!(
            desc,
            Some("rustup component add llvm-tools-preview".to_string())
        );
    }

    // -- run_tools_list_to (requires CwdGuard + .ops.toml) --

    #[test]
    fn tools_list_empty() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.echo]
program = "echo"
"#,
        );
        let mut buf = Vec::new();
        run_tools_list_to(&mut buf).expect("run_tools_list_to");
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("No tools configured"),
            "expected 'No tools configured', got: {output}"
        );
    }

    #[test]
    fn tools_list_shows_tool_count() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[tools]
cargo-fmt = "Format code"
cargo-nonexistent-abc123 = "Fake tool"
"#,
        );
        let mut buf = Vec::new();
        run_tools_list_to(&mut buf).expect("run_tools_list_to");
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("Tools configured: 2"),
            "expected tool count 2, got: {output}"
        );
    }

    #[test]
    fn tools_list_shows_installed_and_missing() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[tools]
cargo-fmt = "Format code"
cargo-nonexistent-abc123 = "Fake tool"
"#,
        );
        let mut buf = Vec::new();
        run_tools_list_to(&mut buf).expect("run_tools_list_to");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("cargo-fmt"), "should list cargo-fmt");
        assert!(
            output.contains("cargo-nonexistent-abc123"),
            "should list fake tool"
        );
        assert!(
            output.contains("NOT INSTALLED"),
            "should show NOT INSTALLED for missing tool"
        );
    }

    // -- run_tools_check_to --

    #[test]
    fn tools_check_empty() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.echo]
program = "echo"
"#,
        );
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_tools_check_to(&mut out, &mut err).expect("run_tools_check_to");
        let output = String::from_utf8(out).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(
            output.contains("No tools configured"),
            "expected 'No tools configured', got: {output}"
        );
    }

    #[test]
    fn tools_check_all_installed() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[tools]
cargo-fmt = "Format code"
"#,
        );
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_tools_check_to(&mut out, &mut err).expect("run_tools_check_to");
        let output = String::from_utf8(out).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(
            output.contains("All 1 tool(s) installed"),
            "expected all installed, got: {output}"
        );
    }

    #[test]
    fn tools_check_missing_returns_failure() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[tools]
cargo-nonexistent-abc123 = "Fake tool"
"#,
        );
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_tools_check_to(&mut out, &mut err).expect("run_tools_check_to");
        let err_output = String::from_utf8(err).unwrap();
        assert_eq!(code, ExitCode::FAILURE);
        assert!(
            err_output.contains("Missing tools:"),
            "expected missing tools in stderr, got: {err_output}"
        );
        assert!(
            err_output.contains("cargo-nonexistent-abc123"),
            "expected tool name in stderr, got: {err_output}"
        );
    }

    // -- run_tools_install_to --

    #[test]
    fn tools_install_all_present() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[tools]
cargo-fmt = "Format code"
"#,
        );
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_tools_install_to(None, &mut out, &mut err).expect("run_tools_install_to");
        let output = String::from_utf8(out).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(
            output.contains("All tools already installed"),
            "expected 'All tools already installed', got: {output}"
        );
    }

    #[test]
    fn tools_install_specific_not_found() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[tools]
cargo-fmt = "Format code"
"#,
        );
        let mut out = Vec::new();
        let mut err = Vec::new();
        let result = run_tools_install_to(Some("nonexistent"), &mut out, &mut err);
        assert!(result.is_err(), "should error for unknown tool name");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("tool not found"),
            "expected 'tool not found', got: {err_msg}"
        );
    }

    #[test]
    fn tools_install_specific_already_installed() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[tools]
cargo-fmt = "Format code"
"#,
        );
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code =
            run_tools_install_to(Some("cargo-fmt"), &mut out, &mut err).expect("install by name");
        let output = String::from_utf8(out).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(
            output.contains("All tools already installed"),
            "expected already installed, got: {output}"
        );
    }
}
