//! Cross-cutting tests for the tools extension.
//!
//! Per-module coverage lives next to the code it exercises: probe internals
//! in `probe/{cargo,rustup,path}.rs`, the status dispatcher in `probe/mod.rs`,
//! and install-argument validation in `install.rs`. What remains here is the
//! crate-level surface: `ToolSpec` deserialization, the `ToolStatus` /
//! `ToolInfo` types, `collect_tools` orchestration, and extension metadata.

use super::*;

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

#[test]
fn tool_status_equality() {
    assert_eq!(ToolStatus::Installed, ToolStatus::Installed);
    assert_eq!(ToolStatus::NotInstalled, ToolStatus::NotInstalled);
    assert_ne!(ToolStatus::Installed, ToolStatus::NotInstalled);
}

/// TEST-1 / TASK-1568: pin the `Display` strings that lib.rs documents
/// as the stable user-facing contract. The previous `tool_status_debug`
/// asserted what `#[derive(Debug)]` produces by definition and could
/// not fail without removing the derive; this test instead binds the
/// surface the doc comment promises ("installed" / "not installed" /
/// "probe failed"). Adding a new variant without extending the
/// `Display` impl in `lib.rs` makes this test fail to compile because
/// the match in `Display` is exhaustive — and the match-on-`self`
/// arms below mirror it, so the lint trail flows from `Display` (the
/// authoritative impl) outwards.
#[test]
fn tool_status_display_strings_are_stable() {
    assert_eq!(ToolStatus::Installed.to_string(), "installed");
    assert_eq!(ToolStatus::NotInstalled.to_string(), "not installed");
    assert_eq!(ToolStatus::ProbeFailed.to_string(), "probe failed");
    // Exhaustive match: a new variant added without a Display arm in
    // lib.rs breaks the match in `Display`, and adding a new variant
    // here without extending the assertions above breaks this match.
    // Either way the contract change cannot ship silently.
    let _: &'static str = match ToolStatus::Installed {
        ToolStatus::Installed => "installed",
        ToolStatus::NotInstalled => "not installed",
        ToolStatus::ProbeFailed => "probe failed",
    };
}

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

#[test]
fn extension_constants() {
    assert_eq!(NAME, "tools");
    assert_eq!(SHORTNAME, "tools");
    assert!(!DESCRIPTION.is_empty());
}

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
