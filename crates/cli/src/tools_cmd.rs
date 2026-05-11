//! CLI handler for `cargo ops tools` subcommands.

use indexmap::IndexMap;
use std::borrow::Cow;
use std::io::Write;
use std::process::ExitCode;

use ops_core::config::Config;
use ops_core::output::display_width;
use ops_core::style::{cyan, dim, green, red};
use ops_tools::{collect_tools, install_tool, ToolInfo, ToolSource, ToolSpec, ToolStatus};

fn load_tools(config: &Config) -> Vec<ToolInfo> {
    collect_tools(&config.tools)
}

pub fn run_tools_list(config: &Config) -> anyhow::Result<()> {
    run_tools_list_to(config, &mut std::io::stdout())
}

fn run_tools_list_to(config: &Config, w: &mut dyn Write) -> anyhow::Result<()> {
    render_tools_list(&load_tools(config), w)
}

/// TEST-25 / TASK-1295: rendering split out so tests feed a deterministic
/// `Vec<ToolInfo>` and avoid coupling to host rustfmt / cargo-fmt presence.
fn render_tools_list(tools: &[ToolInfo], w: &mut dyn Write) -> anyhow::Result<()> {
    if tools.is_empty() {
        writeln!(w, "No tools configured in .ops.toml")?;
        return Ok(());
    }

    writeln!(w, "Tools configured: {}\n", tools.len())?;

    // DUP-3 / TASK-1235: column padding routes through the shared
    // [`ops_core::output::pad_to_display_width`] helper.
    let max_name_width = tools
        .iter()
        .map(|t| display_width(&t.name))
        .max()
        .unwrap_or(0);

    for tool in tools {
        // READ-7 / TASK-0896: ToolStatus is `#[non_exhaustive]`, so the
        // wildcard arm is mandatory. It renders via `Display` (a stable
        // user contract) instead of leaking `Debug` shape through the UI.
        let (status_icon, status_text): (String, Cow<'static, str>) = match tool.status {
            ToolStatus::Installed => (green("✓"), Cow::Borrowed("")),
            ToolStatus::NotInstalled => (red("✗"), Cow::Borrowed(" (NOT INSTALLED)")),
            // TASK-0992: ToolStatus::Unknown was removed — it was declared
            // but never constructed. The wildcard below keeps the match
            // exhaustive over `#[non_exhaustive]` if a distinct
            // probe-failed signal is added later.
            other => (
                dim("?"),
                Cow::Owned(format!(" ({})", other.to_string().to_uppercase())),
            ),
        };

        let padded_name = ops_core::output::pad_to_display_width(&tool.name, max_name_width);
        writeln!(
            w,
            "  {} {}  {}{}",
            status_icon,
            cyan(&padded_name),
            dim(&tool.description),
            dim(&status_text)
        )?;
    }

    Ok(())
}

pub fn run_tools_check(config: &Config) -> anyhow::Result<ExitCode> {
    run_tools_check_to(config, &mut std::io::stdout(), &mut std::io::stderr())
}

fn run_tools_check_to(
    config: &Config,
    w: &mut dyn Write,
    err: &mut dyn Write,
) -> anyhow::Result<ExitCode> {
    render_tools_check(&load_tools(config), w, err)
}

fn render_tools_check(
    tools: &[ToolInfo],
    w: &mut dyn Write,
    err: &mut dyn Write,
) -> anyhow::Result<ExitCode> {
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

pub fn run_tools_install(config: &Config, name: Option<&str>) -> anyhow::Result<ExitCode> {
    run_tools_install_to(config, name, &mut std::io::stdout(), &mut std::io::stderr())
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
    config: &Config,
    name: Option<&str>,
    w: &mut dyn Write,
    err: &mut dyn Write,
) -> anyhow::Result<ExitCode> {
    // Named path constructs a single-entry map because `collect_tools` /
    // `install_missing_tools` consume `&IndexMap<String, ToolSpec>` keyed by
    // the tool name; the unnamed path borrows `config.tools` directly so we
    // skip the deep-clone of every ToolSpec on the install hot path.
    let single_entry: Option<IndexMap<String, ToolSpec>> = if let Some(tool_name) = name {
        let spec = config
            .tools
            .get(tool_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("tool not found: {}", tool_name))?;
        Some([(tool_name.to_string(), spec)].into_iter().collect())
    } else {
        None
    };
    let tools_to_install: &IndexMap<String, ToolSpec> =
        single_entry.as_ref().unwrap_or(&config.tools);

    let tools = collect_tools(tools_to_install);

    let missing: Vec<&ToolInfo> = tools
        .iter()
        .filter(|t| t.status == ToolStatus::NotInstalled)
        .collect();

    if missing.is_empty() {
        writeln!(w, "All tools already installed")?;
        return Ok(ExitCode::SUCCESS);
    }

    writeln!(w, "Installing {} missing tool(s)...\n", missing.len())?;

    let (installed, failed) = install_missing_tools(&missing, tools_to_install, w, err)?;

    writeln!(w)?;
    let already_present = tools.len() - missing.len();
    if failed > 0 {
        writeln!(
            w,
            "Done: {} installed, {} failed, {} already present",
            installed, failed, already_present
        )?;
        Ok(ExitCode::FAILURE)
    } else {
        writeln!(
            w,
            "Done: {} installed, {} already present",
            installed, already_present
        )?;
        Ok(ExitCode::SUCCESS)
    }
}

fn install_missing_tools(
    missing: &[&ToolInfo],
    specs: &IndexMap<String, ToolSpec>,
    w: &mut dyn Write,
    err: &mut dyn Write,
) -> anyhow::Result<(usize, usize)> {
    let mut installed = 0;
    let mut failed = 0;

    for tool in missing {
        let Some(spec) = specs.get(&tool.name) else {
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

    Ok((installed, failed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_tools::ExtendedToolSpec;

    // -- install_command_description --

    fn tool_info(name: &str, status: ToolStatus, has_rustup: bool) -> ToolInfo {
        ToolInfo::new(name.to_string(), "desc".to_string(), status, has_rustup)
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
        run_tools_list_to(&ops_core::config::load_config_or_default("test"), &mut buf)
            .expect("run_tools_list_to");
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
        run_tools_list_to(&ops_core::config::load_config_or_default("test"), &mut buf)
            .expect("run_tools_list_to");
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("Tools configured: 2"),
            "expected tool count 2, got: {output}"
        );
    }

    #[test]
    fn tools_list_shows_installed_and_missing() {
        // TEST-25 / TASK-1295: feed render_tools_list a deterministic
        // ToolInfo vec so the test does not depend on whether the host
        // has rustfmt / cargo-fmt installed.
        let tools = vec![
            tool_info("cargo-fmt", ToolStatus::Installed, false),
            tool_info("cargo-nonexistent-abc123", ToolStatus::NotInstalled, false),
        ];
        let mut buf = Vec::new();
        render_tools_list(&tools, &mut buf).expect("render_tools_list");
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
        let code = run_tools_check_to(
            &ops_core::config::load_config_or_default("test"),
            &mut out,
            &mut err,
        )
        .expect("run_tools_check_to");
        let output = String::from_utf8(out).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(
            output.contains("No tools configured"),
            "expected 'No tools configured', got: {output}"
        );
    }

    #[test]
    fn tools_check_all_installed() {
        let tools = vec![tool_info("cargo-fmt", ToolStatus::Installed, false)];
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = render_tools_check(&tools, &mut out, &mut err).expect("render_tools_check");
        let output = String::from_utf8(out).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(
            output.contains("All 1 tool(s) installed"),
            "expected all installed, got: {output}"
        );
    }

    #[test]
    fn tools_check_missing_returns_failure() {
        let tools = vec![tool_info(
            "cargo-nonexistent-abc123",
            ToolStatus::NotInstalled,
            false,
        )];
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = render_tools_check(&tools, &mut out, &mut err).expect("render_tools_check");
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

    // TEST-25 / TASK-1295: run_tools_install_to probes the host for
    // cargo-fmt and would fail on a CI image without rustfmt installed.
    // Gated behind #[ignore] until the install path supports probe
    // injection too; the rendering/path-resolution layers are already
    // covered by the deterministic tests above.
    #[test]
    #[ignore = "host-dependent: requires rustfmt/cargo-fmt installed"]
    fn tools_install_all_present() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[tools]
cargo-fmt = "Format code"
"#,
        );
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_tools_install_to(
            &ops_core::config::load_config_or_default("test"),
            None,
            &mut out,
            &mut err,
        )
        .expect("run_tools_install_to");
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
        let result = run_tools_install_to(
            &ops_core::config::load_config_or_default("test"),
            Some("nonexistent"),
            &mut out,
            &mut err,
        );
        assert!(result.is_err(), "should error for unknown tool name");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("tool not found"),
            "expected 'tool not found', got: {err_msg}"
        );
    }

    #[test]
    #[ignore = "host-dependent: requires rustfmt/cargo-fmt installed"]
    fn tools_install_specific_already_installed() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[tools]
cargo-fmt = "Format code"
"#,
        );
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_tools_install_to(
            &ops_core::config::load_config_or_default("test"),
            Some("cargo-fmt"),
            &mut out,
            &mut err,
        )
        .expect("install by name");
        let output = String::from_utf8(out).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(
            output.contains("All tools already installed"),
            "expected already installed, got: {output}"
        );
    }

    /// TASK-0758: non-ASCII tool names must be aligned by display width, not
    /// byte length. A width-2 character plus an ASCII name should still produce
    /// a properly aligned description column.
    #[test]
    fn tools_list_aligns_wide_char_names_by_display_width() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[tools]
"ビルド" = "Build tool"
"cargo-fmt" = "Format code"
"#,
        );
        let mut buf = Vec::new();
        run_tools_list_to(&ops_core::config::load_config_or_default("test"), &mut buf)
            .expect("run_tools_list_to");
        let output = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        let build_line = lines
            .iter()
            .find(|l| l.contains("ビルド"))
            .unwrap_or_else(|| panic!("line with ビルド not found in output:\n{output}"));
        let fmt_line = lines
            .iter()
            .find(|l| l.contains("cargo-fmt"))
            .unwrap_or_else(|| panic!("line with cargo-fmt not found in output:\n{output}"));
        // Both descriptions must start at the same *display column*.
        let build_desc_col = display_width(&build_line[..build_line.find("Build tool").unwrap()]);
        let fmt_desc_col = display_width(&fmt_line[..fmt_line.find("Format code").unwrap()]);
        assert_eq!(
            build_desc_col, fmt_desc_col,
            "description columns should be aligned by display width: ビルド at col {build_desc_col}, cargo-fmt at col {fmt_desc_col}"
        );
    }
}
