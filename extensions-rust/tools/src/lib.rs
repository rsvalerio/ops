//! Cargo tools extension: install and manage development tools.

// Re-export tool types from core for convenience of downstream users.
pub use ops_core::config::tools::{ExtendedToolSpec, ToolSource, ToolSpec};

use anyhow::Context;
use indexmap::IndexMap;
use ops_extension::ExtensionType;
use std::process::Command;

pub const NAME: &str = "tools";
pub const DESCRIPTION: &str = "Install and manage cargo development tools";
pub const SHORTNAME: &str = "tools";

pub struct ToolsExtension;

ops_extension::impl_extension! {
    ToolsExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::COMMAND,
    data_provider_name: None,
    register_data_providers: |_self, _registry| {},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Installed,
    NotInstalled,
    #[allow(dead_code)]
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub status: ToolStatus,
    pub has_rustup_component: bool,
}

pub fn get_active_toolchain() -> Option<String> {
    let output = Command::new("rustup")
        .args(["show", "active-toolchain"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_active_toolchain(&stdout)
}

fn parse_active_toolchain(stdout: &str) -> Option<String> {
    let line = stdout.lines().next()?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    parts.first().map(|s| s.to_string())
}

pub fn check_cargo_tool_installed(name: &str) -> bool {
    let output = match Command::new("cargo").args(["--list"]).output() {
        Ok(o) => o,
        Err(_) => return false,
    };

    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Standalone binaries installed via `cargo install` (e.g. tokei) don't
    // appear in `cargo --list` — fall back to checking PATH.
    is_in_cargo_list(&stdout, name) || check_binary_installed(name)
}

fn is_in_cargo_list(stdout: &str, name: &str) -> bool {
    let cargo_name = name.strip_prefix("cargo-").unwrap_or(name);
    stdout.lines().any(|line| {
        line.split_whitespace()
            .next()
            .is_some_and(|cmd| cmd == cargo_name)
    })
}

pub fn check_binary_installed(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .is_ok_and(|o| o.status.success())
}

pub fn check_rustup_component_installed(component: &str) -> bool {
    let output = match Command::new("rustup")
        .args(["component", "list", "--installed"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return false,
    };

    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    is_component_in_list(&stdout, component)
}

fn is_component_in_list(stdout: &str, component: &str) -> bool {
    let base_name = component.strip_suffix("-preview").unwrap_or(component);
    stdout
        .lines()
        .any(|line| line.trim().starts_with(&format!("{}-", base_name)) || line.trim() == base_name)
}

pub fn check_tool_status(name: &str, spec: &ToolSpec) -> ToolStatus {
    if let Some(component) = spec.rustup_component() {
        if !check_rustup_component_installed(component) {
            return ToolStatus::NotInstalled;
        }
    }

    let is_installed = match spec.source() {
        ToolSource::Cargo => check_cargo_tool_installed(name),
        ToolSource::System => check_binary_installed(name),
    };

    if is_installed {
        ToolStatus::Installed
    } else {
        ToolStatus::NotInstalled
    }
}

pub fn install_cargo_tool(name: &str, package: Option<&str>) -> anyhow::Result<()> {
    let mut args = vec!["install"];
    if let Some(pkg) = package {
        args.push(pkg);
        args.push("--bin");
        args.push(name);
    } else {
        args.push(name);
    }

    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("failed to run cargo install")?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("cargo install {} failed", name)
    }
}

pub fn install_rustup_component(component: &str, toolchain: &str) -> anyhow::Result<()> {
    let status = Command::new("rustup")
        .args(["component", "add", component, "--toolchain", toolchain])
        .status()
        .context("failed to run rustup component add")?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("rustup component add {} failed", component)
    }
}

pub fn collect_tools(tools: &IndexMap<String, ToolSpec>) -> Vec<ToolInfo> {
    tools
        .iter()
        .map(|(name, spec)| {
            let status = check_tool_status(name, spec);
            ToolInfo {
                name: name.clone(),
                description: spec.description().to_string(),
                status,
                has_rustup_component: spec.rustup_component().is_some(),
            }
        })
        .collect()
}

pub fn install_tool(name: &str, spec: &ToolSpec) -> anyhow::Result<()> {
    if let Some(component) = spec.rustup_component() {
        let toolchain = get_active_toolchain()
            .ok_or_else(|| anyhow::anyhow!("could not determine active toolchain"))?;
        install_rustup_component(component, &toolchain)?;
    }

    match spec.source() {
        ToolSource::Cargo => {
            install_cargo_tool(name, spec.package())?;
        }
        ToolSource::System => {
            if spec.rustup_component().is_none() {
                anyhow::bail!("system tools cannot be auto-installed: {}", name);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ToolSpec ---

    #[test]
    fn tool_spec_simple_description() {
        let spec = ToolSpec::Simple("desc".to_string());
        assert_eq!(spec.description(), "desc");
    }

    #[test]
    fn tool_spec_simple_no_rustup() {
        let spec = ToolSpec::Simple("desc".to_string());
        assert!(spec.rustup_component().is_none());
    }

    #[test]
    fn tool_spec_simple_source_is_cargo() {
        let spec = ToolSpec::Simple("desc".to_string());
        assert_eq!(spec.source(), ToolSource::Cargo);
    }

    #[test]
    fn tool_spec_simple_package_is_none() {
        let spec = ToolSpec::Simple("desc".to_string());
        assert!(spec.package().is_none());
    }

    #[test]
    fn tool_spec_extended_description() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "extended desc".to_string(),
            rustup_component: None,
            package: None,
            source: ToolSource::Cargo,
        });
        assert_eq!(spec.description(), "extended desc");
    }

    #[test]
    fn tool_spec_extended_rustup() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "desc".to_string(),
            rustup_component: Some("llvm-tools".to_string()),
            package: None,
            source: ToolSource::Cargo,
        });
        assert_eq!(spec.rustup_component(), Some("llvm-tools"));
    }

    #[test]
    fn tool_spec_extended_no_rustup() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "desc".to_string(),
            rustup_component: None,
            package: None,
            source: ToolSource::Cargo,
        });
        assert!(spec.rustup_component().is_none());
    }

    #[test]
    fn tool_spec_extended_package() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "desc".to_string(),
            rustup_component: None,
            package: Some("cargo-llvm-cov".to_string()),
            source: ToolSource::Cargo,
        });
        assert_eq!(spec.package(), Some("cargo-llvm-cov"));
    }

    #[test]
    fn tool_spec_extended_system_source() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "desc".to_string(),
            rustup_component: None,
            package: None,
            source: ToolSource::System,
        });
        assert_eq!(spec.source(), ToolSource::System);
    }

    // --- ToolStatus ---

    #[test]
    fn tool_status_equality() {
        assert_eq!(ToolStatus::Installed, ToolStatus::Installed);
        assert_eq!(ToolStatus::NotInstalled, ToolStatus::NotInstalled);
        assert_eq!(ToolStatus::Unknown, ToolStatus::Unknown);
        assert_ne!(ToolStatus::Installed, ToolStatus::NotInstalled);
        assert_ne!(ToolStatus::Installed, ToolStatus::Unknown);
        assert_ne!(ToolStatus::NotInstalled, ToolStatus::Unknown);
    }

    #[test]
    fn tool_status_debug() {
        assert_eq!(format!("{:?}", ToolStatus::Installed), "Installed");
        assert_eq!(format!("{:?}", ToolStatus::NotInstalled), "NotInstalled");
    }

    #[test]
    fn tool_status_clone() {
        let status = ToolStatus::Installed;
        let cloned = status;
        assert_eq!(status, cloned);
    }

    // --- ToolInfo ---

    #[test]
    fn tool_info_fields() {
        let info = ToolInfo {
            name: "cargo-nextest".to_string(),
            description: "A better test runner".to_string(),
            status: ToolStatus::Installed,
            has_rustup_component: false,
        };
        assert_eq!(info.name, "cargo-nextest");
        assert_eq!(info.description, "A better test runner");
        assert_eq!(info.status, ToolStatus::Installed);
        assert!(!info.has_rustup_component);
    }

    #[test]
    fn tool_info_clone() {
        let info = ToolInfo {
            name: "test".to_string(),
            description: "desc".to_string(),
            status: ToolStatus::NotInstalled,
            has_rustup_component: true,
        };
        let cloned = info.clone();
        assert_eq!(cloned.name, "test");
        assert_eq!(cloned.status, ToolStatus::NotInstalled);
        assert!(cloned.has_rustup_component);
    }

    // --- parse_active_toolchain ---

    #[test]
    fn parse_active_toolchain_typical() {
        let output = "stable-aarch64-apple-darwin (default)\n";
        assert_eq!(
            parse_active_toolchain(output),
            Some("stable-aarch64-apple-darwin".to_string())
        );
    }

    #[test]
    fn parse_active_toolchain_nightly() {
        let output = "nightly-x86_64-unknown-linux-gnu (overridden)\n";
        assert_eq!(
            parse_active_toolchain(output),
            Some("nightly-x86_64-unknown-linux-gnu".to_string())
        );
    }

    #[test]
    fn parse_active_toolchain_no_annotation() {
        let output = "stable-aarch64-apple-darwin\n";
        assert_eq!(
            parse_active_toolchain(output),
            Some("stable-aarch64-apple-darwin".to_string())
        );
    }

    #[test]
    fn parse_active_toolchain_empty() {
        assert_eq!(parse_active_toolchain(""), None);
    }

    #[test]
    fn parse_active_toolchain_blank_line() {
        assert_eq!(parse_active_toolchain("   "), None);
    }

    #[test]
    fn parse_active_toolchain_multiline() {
        let output = "stable-aarch64-apple-darwin (default)\nextra line\n";
        assert_eq!(
            parse_active_toolchain(output),
            Some("stable-aarch64-apple-darwin".to_string())
        );
    }

    // --- is_in_cargo_list ---

    #[test]
    fn cargo_list_finds_subcommand() {
        let stdout = "    bench\n    build\n    fmt\n    check\n";
        assert!(is_in_cargo_list(stdout, "cargo-fmt"));
    }

    #[test]
    fn cargo_list_strips_cargo_prefix() {
        let stdout = "    fmt\n";
        assert!(is_in_cargo_list(stdout, "cargo-fmt"));
    }

    #[test]
    fn cargo_list_no_prefix_match() {
        let stdout = "    fmt\n";
        assert!(is_in_cargo_list(stdout, "fmt"));
    }

    #[test]
    fn cargo_list_not_found() {
        let stdout = "    bench\n    build\n    check\n";
        assert!(!is_in_cargo_list(stdout, "cargo-nextest"));
    }

    #[test]
    fn cargo_list_empty() {
        assert!(!is_in_cargo_list("", "cargo-fmt"));
    }

    #[test]
    fn cargo_list_ignores_description_suffix() {
        let stdout = "    fmt                  Format Rust code\n";
        assert!(is_in_cargo_list(stdout, "cargo-fmt"));
    }

    #[test]
    fn cargo_list_partial_name_no_match() {
        let stdout = "    fmt\n";
        assert!(!is_in_cargo_list(stdout, "cargo-fmtx"));
    }

    #[test]
    fn cargo_list_similar_prefix_no_match() {
        let stdout = "    nextest\n";
        assert!(!is_in_cargo_list(stdout, "cargo-next"));
    }

    // --- is_component_in_list ---

    #[test]
    fn component_list_finds_exact() {
        let stdout = "clippy\nrustfmt\n";
        assert!(is_component_in_list(stdout, "clippy"));
    }

    #[test]
    fn component_list_finds_with_target_suffix() {
        let stdout = "rustfmt-aarch64-apple-darwin\nclippy-aarch64-apple-darwin\n";
        assert!(is_component_in_list(stdout, "rustfmt"));
        assert!(is_component_in_list(stdout, "clippy"));
    }

    #[test]
    fn component_list_not_found() {
        let stdout = "clippy-aarch64-apple-darwin\nrustfmt-aarch64-apple-darwin\n";
        assert!(!is_component_in_list(stdout, "miri"));
    }

    #[test]
    fn component_list_empty() {
        assert!(!is_component_in_list("", "clippy"));
    }

    #[test]
    fn component_list_preview_suffix_stripped() {
        let stdout = "rust-analyzer-aarch64-apple-darwin\n";
        assert!(is_component_in_list(stdout, "rust-analyzer-preview"));
    }

    #[test]
    fn component_list_whitespace_trimmed() {
        let stdout = "  clippy-aarch64-apple-darwin  \n  rustfmt  \n";
        assert!(is_component_in_list(stdout, "clippy"));
        assert!(is_component_in_list(stdout, "rustfmt"));
    }

    #[test]
    fn component_list_llvm_tools() {
        let stdout = "llvm-tools-aarch64-apple-darwin\n";
        assert!(is_component_in_list(stdout, "llvm-tools"));
        assert!(is_component_in_list(stdout, "llvm-tools-preview"));
    }

    // --- Integration tests (require rustup/cargo in PATH) ---

    #[test]
    fn get_active_toolchain_returns_some() {
        let tc = get_active_toolchain();
        assert!(
            tc.is_some(),
            "rustup should be available in dev environment"
        );
        let tc = tc.unwrap();
        assert!(
            !tc.is_empty(),
            "toolchain string should not be empty, got: {tc}"
        );
    }

    #[test]
    fn check_binary_installed_finds_rustup() {
        assert!(check_binary_installed("rustup"));
    }

    #[test]
    fn check_binary_installed_nonexistent() {
        assert!(!check_binary_installed("nonexistent-binary-abc123xyz"));
    }

    #[test]
    fn check_cargo_tool_installed_fmt() {
        // cargo-fmt ships with rustup, should always be present
        assert!(check_cargo_tool_installed("cargo-fmt"));
    }

    #[test]
    fn check_cargo_tool_installed_nonexistent() {
        assert!(!check_cargo_tool_installed("cargo-nonexistent-abc123"));
    }

    #[test]
    fn check_rustup_component_installed_rustfmt() {
        assert!(check_rustup_component_installed("rustfmt"));
    }

    #[test]
    fn check_rustup_component_installed_nonexistent() {
        assert!(!check_rustup_component_installed(
            "nonexistent-component-xyz"
        ));
    }

    // --- check_tool_status ---

    #[test]
    fn check_tool_status_simple_installed() {
        let spec = ToolSpec::Simple("Format code".to_string());
        assert_eq!(check_tool_status("cargo-fmt", &spec), ToolStatus::Installed);
    }

    #[test]
    fn check_tool_status_simple_not_installed() {
        let spec = ToolSpec::Simple("desc".to_string());
        assert_eq!(
            check_tool_status("cargo-nonexistent-abc123", &spec),
            ToolStatus::NotInstalled
        );
    }

    #[test]
    fn check_tool_status_extended_with_rustup_component() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "Clippy lints".to_string(),
            rustup_component: Some("clippy".to_string()),
            package: None,
            source: ToolSource::Cargo,
        });
        assert_eq!(
            check_tool_status("cargo-clippy", &spec),
            ToolStatus::Installed
        );
    }

    #[test]
    fn check_tool_status_system_binary() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "Rust toolchain manager".to_string(),
            rustup_component: None,
            package: None,
            source: ToolSource::System,
        });
        assert_eq!(check_tool_status("rustup", &spec), ToolStatus::Installed);
    }

    #[test]
    fn check_tool_status_system_missing() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "desc".to_string(),
            rustup_component: None,
            package: None,
            source: ToolSource::System,
        });
        assert_eq!(
            check_tool_status("nonexistent-abc123", &spec),
            ToolStatus::NotInstalled
        );
    }

    #[test]
    fn check_tool_status_missing_rustup_component() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "desc".to_string(),
            rustup_component: Some("nonexistent-component-xyz".to_string()),
            package: None,
            source: ToolSource::Cargo,
        });
        assert_eq!(
            check_tool_status("cargo-fmt", &spec),
            ToolStatus::NotInstalled
        );
    }

    // --- collect_tools ---

    #[test]
    fn collect_tools_empty() {
        let tools = IndexMap::new();
        let result = collect_tools(&tools);
        assert!(result.is_empty());
    }

    #[test]
    fn collect_tools_preserves_order() {
        let mut tools = IndexMap::new();
        tools.insert(
            "cargo-fmt".to_string(),
            ToolSpec::Simple("Format code".to_string()),
        );
        tools.insert(
            "nonexistent-abc123".to_string(),
            ToolSpec::Simple("Missing tool".to_string()),
        );
        let result = collect_tools(&tools);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "cargo-fmt");
        assert_eq!(result[0].status, ToolStatus::Installed);
        assert!(!result[0].has_rustup_component);
        assert_eq!(result[1].name, "nonexistent-abc123");
        assert_eq!(result[1].status, ToolStatus::NotInstalled);
    }

    #[test]
    fn collect_tools_with_rustup_component() {
        let mut tools = IndexMap::new();
        tools.insert(
            "cargo-clippy".to_string(),
            ToolSpec::Extended(ExtendedToolSpec {
                description: "Clippy".to_string(),
                rustup_component: Some("clippy".to_string()),
                package: None,
                source: ToolSource::Cargo,
            }),
        );
        let result = collect_tools(&tools);
        assert_eq!(result.len(), 1);
        assert!(result[0].has_rustup_component);
        assert_eq!(result[0].status, ToolStatus::Installed);
    }

    // --- install_tool ---

    #[test]
    fn install_tool_system_no_rustup_errors() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "system tool".to_string(),
            rustup_component: None,
            package: None,
            source: ToolSource::System,
        });
        let result = install_tool("some-system-tool", &spec);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("cannot be auto-installed"),
            "expected 'cannot be auto-installed', got: {err}"
        );
    }

    // --- Extension metadata ---

    #[test]
    fn extension_constants() {
        assert_eq!(NAME, "tools");
        assert_eq!(SHORTNAME, "tools");
        assert!(!DESCRIPTION.is_empty());
    }

    // --- TOML deserialization ---

    #[test]
    fn tool_spec_deserializes_simple_from_string() {
        let toml_str = r#"
[tools]
cargo-nextest = "A better test runner"
"#;
        let val: toml::Value = toml::from_str(toml_str).unwrap();
        let table = val["tools"].as_table().unwrap();
        for (name, v) in table {
            let spec: ToolSpec = v.clone().try_into().unwrap();
            assert_eq!(name, "cargo-nextest");
            assert_eq!(spec.description(), "A better test runner");
            assert_eq!(spec.source(), ToolSource::Cargo);
            assert!(spec.rustup_component().is_none());
            assert!(spec.package().is_none());
        }
    }

    #[test]
    fn tool_spec_deserializes_extended_from_table() {
        let toml_str = r#"
[tools.cargo-llvm-cov]
description = "Code coverage"
rustup-component = "llvm-tools-preview"
package = "cargo-llvm-cov"
source = "cargo"
"#;
        let val: toml::Value = toml::from_str(toml_str).unwrap();
        let table = val["tools"].as_table().unwrap();
        for (name, v) in table {
            let spec: ToolSpec = v.clone().try_into().unwrap();
            assert_eq!(name, "cargo-llvm-cov");
            assert_eq!(spec.description(), "Code coverage");
            assert_eq!(spec.rustup_component(), Some("llvm-tools-preview"));
            assert_eq!(spec.package(), Some("cargo-llvm-cov"));
            assert_eq!(spec.source(), ToolSource::Cargo);
        }
    }
}
