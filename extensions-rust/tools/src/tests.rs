use super::*;
use crate::probe::{is_component_in_list, is_in_cargo_list, parse_active_toolchain};

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

#[test]
fn parse_active_toolchain_modern_rustup_format() {
    // rustup >=1.28 prints the toolchain on the first line, then an
    // explanatory "active because: ..." line. The parser must take the
    // first non-empty line and ignore subsequent annotation lines.
    let output = "stable-aarch64-apple-darwin\nactive because: it's the default toolchain\n";
    assert_eq!(
        parse_active_toolchain(output),
        Some("stable-aarch64-apple-darwin".to_string())
    );
}

#[test]
fn parse_active_toolchain_skips_leading_blank_lines() {
    let output = "\n\n  \nstable-aarch64-apple-darwin (default)\n";
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
#[ignore = "requires rustup installed; run with: cargo test -- --ignored"]
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
#[ignore = "requires rustup installed; run with: cargo test -- --ignored"]
fn check_binary_installed_finds_rustup() {
    assert!(check_binary_installed("rustup"));
}

#[test]
fn check_binary_installed_nonexistent() {
    assert!(!check_binary_installed("nonexistent-binary-abc123xyz"));
}

#[test]
#[ignore = "requires rustup + cargo-fmt installed; run with: cargo test -- --ignored"]
fn check_cargo_tool_installed_fmt() {
    // cargo-fmt ships with rustup, should always be present
    assert!(check_cargo_tool_installed("cargo-fmt"));
}

#[test]
fn check_cargo_tool_installed_nonexistent() {
    assert!(!check_cargo_tool_installed("cargo-nonexistent-abc123"));
}

#[test]
#[ignore = "requires rustup + rustfmt component installed; run with: cargo test -- --ignored"]
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
#[ignore = "requires rustup + cargo-fmt installed; run with: cargo test -- --ignored"]
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
#[ignore = "requires rustup + clippy component installed; run with: cargo test -- --ignored"]
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
#[ignore = "requires rustup installed; run with: cargo test -- --ignored"]
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
#[ignore = "requires rustup + cargo-fmt installed; run with: cargo test -- --ignored"]
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
#[ignore = "requires rustup + clippy component installed; run with: cargo test -- --ignored"]
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

// --- SEC-13: cargo install argument validation ---

use crate::install::validate_cargo_tool_arg;

#[test]
fn validate_cargo_tool_arg_accepts_real_crate_names() {
    assert!(validate_cargo_tool_arg("cargo-llvm-cov", "tool name").is_ok());
    assert!(validate_cargo_tool_arg("ripgrep", "tool name").is_ok());
    assert!(validate_cargo_tool_arg("crate_with_underscore", "tool name").is_ok());
    assert!(validate_cargo_tool_arg("crate.with.dots", "tool name").is_ok());
    assert!(validate_cargo_tool_arg("a", "tool name").is_ok());
}

/// SEC-13 AC #1+#2: a name beginning with `-` would be parsed by `cargo
/// install` as a flag (--config, --git, …) and silently change install
/// semantics. Reject before invocation.
#[test]
fn validate_cargo_tool_arg_rejects_leading_dash() {
    assert!(validate_cargo_tool_arg("-config=foo", "tool name").is_err());
    assert!(validate_cargo_tool_arg("--git=https://attacker", "tool name").is_err());
}

#[test]
fn validate_cargo_tool_arg_rejects_empty() {
    assert!(validate_cargo_tool_arg("", "tool name").is_err());
}

#[test]
fn validate_cargo_tool_arg_rejects_other_metacharacters() {
    for bad in [
        "name;rm -rf /",
        "name with space",
        "name$VAR",
        "name`cmd`",
        "name|pipe",
        "name/slash",
        "name\\bslash",
        "name\nnewline",
    ] {
        assert!(
            validate_cargo_tool_arg(bad, "tool name").is_err(),
            "expected rejection of {bad:?}"
        );
    }
}

// --- SEC-13: PATH-walking binary detection ---

use crate::probe::{find_on_path, find_on_path_in};

/// SEC-13 AC #2: cross-platform — a binary placed in a directory on PATH is
/// located. Uses `find_on_path_in` so the test does not have to mutate the
/// process-wide PATH (which would race against parallel tests).
#[cfg(unix)]
#[test]
fn find_on_path_in_locates_executable_unix() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let bin_path = dir.path().join("ops_marker_unix");
    std::fs::write(&bin_path, b"#!/bin/sh\n").unwrap();
    let mut perms = std::fs::metadata(&bin_path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&bin_path, perms).unwrap();

    let path_var = std::env::join_paths([dir.path().to_path_buf()]).unwrap();
    assert_eq!(
        find_on_path_in("ops_marker_unix", &path_var),
        Some(bin_path)
    );
}

/// SEC-13: on Unix a non-executable file in a PATH directory must not be
/// reported — `is_executable` requires the executable bit.
#[cfg(unix)]
#[test]
fn find_on_path_in_skips_non_executable_unix() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin_path = dir.path().join("ops_marker_unix_noexec");
    std::fs::write(&bin_path, b"data\n").unwrap();
    let path_var = std::env::join_paths([dir.path().to_path_buf()]).unwrap();
    assert_eq!(find_on_path_in("ops_marker_unix_noexec", &path_var), None);
}

/// SEC-13: documented Windows fallback is the PATHEXT suffix loop (mirrors
/// `which` / PowerShell). The helper appends each suffix and checks for a
/// regular file.
#[cfg(windows)]
#[test]
fn find_on_path_in_locates_executable_with_pathext_windows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin_path = dir.path().join("ops_marker_win.exe");
    std::fs::write(&bin_path, b"\0").unwrap();
    let path_var = std::env::join_paths([dir.path().to_path_buf()]).unwrap();
    assert_eq!(find_on_path_in("ops_marker_win", &path_var), Some(bin_path));
}

#[test]
fn find_on_path_returns_none_for_missing_binary() {
    assert!(find_on_path("nonexistent-binary-abc123xyz-zzz").is_none());
}
