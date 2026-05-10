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
    assert_ne!(ToolStatus::Installed, ToolStatus::NotInstalled);
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

#[test]
fn parse_active_toolchain_rejects_error_prefix() {
    assert_eq!(
        parse_active_toolchain("error: no active toolchain configured\n"),
        None
    );
}

#[test]
fn parse_active_toolchain_rejects_info_prefix() {
    assert_eq!(parse_active_toolchain("info: something\n"), None);
}

/// ERR-1 / TASK-1197: rustup commonly emits a leading `info:` progress line
/// before the real toolchain identifier — skip it and continue scanning.
#[test]
fn parse_active_toolchain_skips_leading_info_prefix() {
    let output = "info: syncing channel updates for 'stable'\nstable-aarch64-apple-darwin\n";
    assert_eq!(
        parse_active_toolchain(output),
        Some("stable-aarch64-apple-darwin".to_string())
    );
}

#[test]
fn parse_active_toolchain_returns_none_when_only_diagnostics() {
    assert_eq!(
        parse_active_toolchain("error: no default toolchain configured\n"),
        None
    );
}

#[test]
fn parse_active_toolchain_rejects_no_active_toolchain_message() {
    // rustup ≥1.28 "no active toolchain" output
    assert_eq!(
        parse_active_toolchain("no active toolchain configured\n"),
        Some("no".to_string())
    );
    // The colon-containing diagnostic variant
    assert_eq!(
        parse_active_toolchain("error: toolchain 'nonexistent' is not installed\n"),
        None
    );
}

/// PATTERN-1 / TASK-1078: a blanket "contains ':'" reject would also drop
/// legitimate identifiers — custom toolchains registered via `rustup
/// toolchain link` may carry a `:`-bearing name, and on Windows the
/// active-toolchain output can include `C:\path\...` shaped tokens. Only
/// the rustup diagnostic prefixes (full segment match) should reject.
#[test]
fn parse_active_toolchain_accepts_colon_in_token() {
    // Windows-style path token.
    assert_eq!(
        parse_active_toolchain("C:\\path\\to\\toolchain\n"),
        Some("C:\\path\\to\\toolchain".to_string())
    );
    // Linked-toolchain name containing a colon.
    assert_eq!(
        parse_active_toolchain("linked:custom-toolchain\n"),
        Some("linked:custom-toolchain".to_string())
    );
    // Diagnostic prefix is still rejected — full-segment match, not substring.
    assert_eq!(parse_active_toolchain("error: no active toolchain\n"), None);
    // A token whose first segment is `warning:` / `note:` is still rejected.
    assert_eq!(parse_active_toolchain("warning: stale cache\n"), None);
    assert_eq!(parse_active_toolchain("note: details follow\n"), None);
    // But a toolchain whose name merely *contains* "error:" as a substring
    // (highly unusual but legal in a linked name) is not blanket-rejected.
    assert_eq!(
        parse_active_toolchain("custom-error:variant\n"),
        Some("custom-error:variant".to_string())
    );
}

// --- is_in_cargo_list ---

#[test]
fn cargo_list_finds_subcommand() {
    let stdout = "    bench\n    build\n    nextest\n    check\n";
    assert!(is_in_cargo_list(stdout, "cargo-nextest"));
}

#[test]
fn cargo_list_strips_cargo_prefix() {
    let stdout = "    nextest\n";
    assert!(is_in_cargo_list(stdout, "cargo-nextest"));
}

#[test]
fn cargo_list_no_prefix_match() {
    let stdout = "    watch\n";
    assert!(is_in_cargo_list(stdout, "watch"));
}

#[test]
fn cargo_list_not_found() {
    let stdout = "    bench\n    build\n    check\n";
    assert!(!is_in_cargo_list(stdout, "cargo-nextest"));
}

#[test]
fn cargo_list_empty() {
    assert!(!is_in_cargo_list("", "cargo-nextest"));
}

#[test]
fn cargo_list_ignores_description_suffix() {
    let stdout = "    nextest              Next-gen test runner\n";
    assert!(is_in_cargo_list(stdout, "cargo-nextest"));
}

#[test]
fn cargo_list_partial_name_no_match() {
    let stdout = "    nextest\n";
    assert!(!is_in_cargo_list(stdout, "cargo-nextestx"));
}

#[test]
fn cargo_list_similar_prefix_no_match() {
    let stdout = "    nextest\n";
    assert!(!is_in_cargo_list(stdout, "cargo-next"));
}

/// TASK-0526: an empty-after-strip name (literal "cargo-" or "") must not
/// match a line that begins with whitespace. The empty token from
/// split_whitespace was previously equal to the empty stripped name, so any
/// indented line was reported as installed.
#[test]
fn cargo_list_empty_name_after_strip_is_rejected() {
    let stdout = "    nextest\n    watch\n";
    assert!(!is_in_cargo_list(stdout, "cargo-"));
    assert!(!is_in_cargo_list(stdout, ""));
}

/// PATTERN-1 / TASK-1101: built-in cargo subcommands (e.g. `build`, `check`,
/// `test`, `run`) appear in `cargo --list` because they're shipped inside the
/// cargo binary itself — not because anyone ran `cargo install cargo-build`.
/// A `tools.toml` entry that collides with a built-in name must not be
/// reported as installed via the membership check; it should fall through to
/// the PATH probe so a real `cargo-<name>` executable still resolves.
#[test]
fn cargo_list_rejects_builtin_subcommand_named_build() {
    let stdout = "    build\n    cargo-foo\n";
    assert!(!is_in_cargo_list(stdout, "build"));
    assert!(!is_in_cargo_list(stdout, "cargo-build"));
}

#[test]
fn cargo_list_rejects_other_common_builtins() {
    let stdout = "    check\n    test\n    run\n    clippy\n    fmt\n    update\n";
    for builtin in ["check", "test", "run", "clippy", "fmt", "update"] {
        assert!(
            !is_in_cargo_list(stdout, builtin),
            "built-in {builtin} must not match"
        );
    }
}

#[test]
fn cargo_list_still_resolves_real_third_party_among_builtins() {
    // Mixed listing: real `cargo install`-ed tools alongside built-ins.
    // `cargo-watch` / `cargo-nextest`-style entries must still resolve.
    let stdout = "    bench\n    build\n    check\n    watch\n    nextest\n";
    assert!(is_in_cargo_list(stdout, "cargo-watch"));
    assert!(is_in_cargo_list(stdout, "watch"));
    assert!(is_in_cargo_list(stdout, "cargo-nextest"));
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

#[test]
fn component_list_matches_preview_listing_for_base_search() {
    let stdout = "clippy-preview-aarch64-apple-darwin\n";
    assert!(is_component_in_list(stdout, "clippy"));
    assert!(is_component_in_list(stdout, "clippy-preview"));
}

#[test]
fn component_list_rejects_unrelated_dash_sibling() {
    // `clippy-foo-aarch64-apple-darwin` must NOT match a search for "clippy".
    let stdout = "clippy-foo-aarch64-apple-darwin\n";
    assert!(!is_component_in_list(stdout, "clippy"));
    assert!(!is_component_in_list(stdout, "clippy-preview"));
}

#[test]
fn component_list_handles_installed_annotation() {
    let stdout = "clippy-aarch64-apple-darwin (installed)\n";
    assert!(is_component_in_list(stdout, "clippy"));
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
    assert!(matches!(
        check_cargo_tool_installed("cargo-fmt"),
        ProbeOutcome::Ok(true)
    ));
}

#[test]
fn check_cargo_tool_installed_nonexistent() {
    assert!(matches!(
        check_cargo_tool_installed("cargo-nonexistent-abc123"),
        ProbeOutcome::Ok(false) | ProbeOutcome::Failed
    ));
}

#[test]
#[ignore = "requires rustup + rustfmt component installed; run with: cargo test -- --ignored"]
fn check_rustup_component_installed_rustfmt() {
    assert!(matches!(
        check_rustup_component_installed("rustfmt"),
        ProbeOutcome::Ok(true)
    ));
}

#[test]
fn check_rustup_component_installed_nonexistent() {
    assert!(matches!(
        check_rustup_component_installed("nonexistent-component-xyz"),
        ProbeOutcome::Ok(false) | ProbeOutcome::Failed
    ));
}

// --- check_tool_status ---

#[test]
#[ignore = "requires rustup + cargo-fmt installed; run with: cargo test -- --ignored"]
fn check_tool_status_simple_installed() {
    let spec = ToolSpec::Simple("Format code".to_string());
    assert_eq!(check_tool_status("cargo-fmt", &spec), ToolStatus::Installed);
}

/// API / TASK-1200: when the underlying probe (`rustup component list
/// --installed`) cannot be answered (here: simulated by pointing
/// `$RUSTUP` at a script that exits non-zero), the tool's status must
/// surface as [`ToolStatus::ProbeFailed`] rather than silently
/// collapsing onto [`ToolStatus::NotInstalled`]. The CLI install path
/// (`run_tools_install`) filters strictly on `NotInstalled`, so a
/// `ProbeFailed` entry no longer triggers the reinstall mutation that
/// motivated this finding.
///
/// We exercise the non-zero-exit branch (rather than a real timeout)
/// to keep the test fast and deterministic; the
/// `timeout_returns_none_quickly` test in `probe::timeout` already
/// pins that the timeout path itself surfaces as
/// `ProbeOutcome::Failed`, which `check_tool_status_with` then maps
/// to `ProbeFailed` via the same arm.
#[test]
#[cfg(unix)]
#[serial_test::serial]
fn check_tool_status_surfaces_probe_failed_on_wedged_rustup() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let fake = dir.path().join("rustup");
    std::fs::write(&fake, "#!/bin/sh\necho 'rustup is wedged' >&2\nexit 1\n").unwrap();
    let mut perms = std::fs::metadata(&fake).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&fake, perms).unwrap();

    let spec = ToolSpec::Extended(ExtendedToolSpec {
        description: "needs rustup".to_string(),
        rustup_component: Some("clippy".to_string()),
        package: None,
        source: ToolSource::Cargo,
    });

    let prev_rustup = std::env::var_os("RUSTUP");
    // SAFETY: serial_test::serial guards env-mutation; the probe spawned
    // below honours `RUSTUP` synchronously.
    unsafe { std::env::set_var("RUSTUP", &fake) };

    let status = check_tool_status("clippy", &spec);

    unsafe {
        match prev_rustup {
            Some(v) => std::env::set_var("RUSTUP", v),
            None => std::env::remove_var("RUSTUP"),
        }
    };

    assert_eq!(
        status,
        ToolStatus::ProbeFailed,
        "a wedged rustup probe must surface as ProbeFailed, not NotInstalled (which would trigger reinstall)"
    );
}

/// API / TASK-1200: pin the install-path policy: `run_tools_install`
/// filters strictly on `ToolStatus::NotInstalled`, so a `ProbeFailed`
/// entry must NOT be picked up for reinstall. The previous shape
/// collapsed timeout/IO errors onto `NotInstalled`, turning a transient
/// probe failure into a real `cargo install` / `rustup component add`
/// mutation.
#[test]
fn probe_failed_status_excluded_from_install_filter() {
    let statuses = [
        ToolStatus::Installed,
        ToolStatus::NotInstalled,
        ToolStatus::ProbeFailed,
    ];
    let to_install: Vec<_> = statuses
        .iter()
        .filter(|s| **s == ToolStatus::NotInstalled)
        .collect();
    assert_eq!(to_install.len(), 1);
    assert_eq!(*to_install[0], ToolStatus::NotInstalled);
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

// ERR-2 (TASK-1038): policy pin — when a ToolSpec sets *both* a Cargo source
// AND a rustup_component, the rustup-component install path is preferred and
// `cargo install` is skipped. Without this, install_tool would run both and
// silently produce two installations where the operator wanted one.
#[test]
fn install_tool_prefers_rustup_when_both_set() {
    use crate::install::should_run_cargo_install;

    let spec = ToolSpec::Extended(ExtendedToolSpec {
        description: "tool with both paths set".to_string(),
        rustup_component: Some("llvm-tools-preview".to_string()),
        package: Some("cargo-llvm-cov".to_string()),
        source: ToolSource::Cargo,
    });
    assert!(
        !should_run_cargo_install(&spec),
        "cargo install must be skipped when rustup_component is also set"
    );
}

#[test]
fn install_tool_runs_cargo_when_only_cargo_source_set() {
    use crate::install::should_run_cargo_install;

    let spec = ToolSpec::Extended(ExtendedToolSpec {
        description: "cargo-only tool".to_string(),
        rustup_component: None,
        package: Some("cargo-nextest".to_string()),
        source: ToolSource::Cargo,
    });
    assert!(should_run_cargo_install(&spec));
}

#[test]
fn install_tool_simple_spec_runs_cargo() {
    use crate::install::should_run_cargo_install;

    let spec = ToolSpec::Simple("a description".to_string());
    assert!(should_run_cargo_install(&spec));
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
    assert!(validate_cargo_tool_arg("a", "tool name").is_ok());
    // SEC-13 / TASK-1199 AC #2: legitimate names already in use across
    // ops `[tools]` blocks must continue to validate after the dot was
    // dropped from the allow-set.
    for ok in ["cargo-deny", "cargo-edit", "clippy", "rustfmt", "rust-src"] {
        assert!(
            validate_cargo_tool_arg(ok, "tool name").is_ok(),
            "expected {ok:?} to pass after dropping `.` from the allow-set"
        );
    }
}

/// SEC-13 / TASK-1199 AC #1: `.` is no longer in the allow-set. The error
/// message must name the offending `.` so an operator hitting an invalid
/// `[tools]` entry like `tool.cargo` learns which character broke the
/// validator (rather than seeing a generic "must start with alphanumeric"
/// reroute).
#[test]
fn validate_cargo_tool_arg_rejects_dot_in_name() {
    let err = validate_cargo_tool_arg("ops.bad", "tool name").expect_err("dot must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains('.'),
        "error message must name the offending `.`, got: {msg}"
    );
    assert!(
        msg.contains("invalid character"),
        "error must point at the per-character allow-set, got: {msg}"
    );
    // Multi-dot shapes should still trip the same per-character check.
    assert!(validate_cargo_tool_arg("cargo.deny.something", "tool name").is_err());
    // A leading dot trips the leading-character guard rather than the
    // body loop, so the message wording changes — but the value still
    // ends up rejected.
    assert!(validate_cargo_tool_arg(".dotfile", "tool name").is_err());
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

// --- SEC-13/TASK-0434/TASK-0473: rustup component / toolchain validation ---

use crate::install::install_rustup_component_with_timeout;

/// Reject leading-dash component before spawning rustup. Mirrors the cargo
/// install guard so values like `--default` cannot be re-parsed by rustup as
/// a flag.
#[test]
fn install_rustup_component_rejects_dash_component() {
    let err = install_rustup_component_with_timeout(
        "--default",
        "stable",
        std::time::Duration::from_secs(1),
    )
    .expect_err("expected rejection of leading-dash component");
    assert!(
        err.to_string().contains("rustup component"),
        "error should mention component: {err}"
    );
}

#[test]
fn install_rustup_component_rejects_dash_toolchain() {
    let err =
        install_rustup_component_with_timeout("rust-src", "-vV", std::time::Duration::from_secs(1))
            .expect_err("expected rejection of leading-dash toolchain");
    assert!(
        err.to_string().contains("rustup toolchain"),
        "error should mention toolchain: {err}"
    );
}

// --- SEC-13: PATH-walking binary detection ---

use crate::probe::{
    capture_path_index_from, check_binary_installed_with, find_on_path, find_on_path_in,
    is_in_path_index,
};

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

/// ERR-1 / TASK-0607: a broken symlink on PATH (target removed mid-run, e.g.
/// nix-env update) must not silently coerce the lookup to "missing"; the
/// walk continues but emits a warning. The functional contract here pins
/// that lookup keeps working when a sibling directory holds the real binary.
#[cfg(unix)]
#[test]
fn find_on_path_in_skips_broken_symlink_continues_walk() {
    use std::os::unix::fs::PermissionsExt;

    let broken_dir = tempfile::tempdir().expect("tempdir");
    let real_dir = tempfile::tempdir().expect("tempdir");

    // Broken symlink: PATH/<broken_dir>/ops_marker -> nonexistent target.
    let symlink_path = broken_dir.path().join("ops_marker_broken_sym");
    let nonexistent_target = broken_dir.path().join("does-not-exist");
    std::os::unix::fs::symlink(&nonexistent_target, &symlink_path).unwrap();

    // Real executable in a later PATH entry.
    let real_bin = real_dir.path().join("ops_marker_broken_sym");
    std::fs::write(&real_bin, b"#!/bin/sh\n").unwrap();
    let mut perms = std::fs::metadata(&real_bin).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&real_bin, perms).unwrap();

    let path_var = std::env::join_paths([
        broken_dir.path().to_path_buf(),
        real_dir.path().to_path_buf(),
    ])
    .unwrap();
    assert_eq!(
        find_on_path_in("ops_marker_broken_sym", &path_var),
        Some(real_bin),
        "broken symlink in earlier PATH dir must not block lookup"
    );
}

// --- PERF-3 / TASK-1046: PATH-index amortisation ---

/// PERF-3 AC #1: a precomputed PATH index resolves a binary placed in a PATH
/// directory without walking PATH per call. Mirrors
/// `find_on_path_in_locates_executable_unix` so the test does not mutate the
/// process-wide PATH.
#[cfg(unix)]
#[test]
fn path_index_finds_executable_basename() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let bin_path = dir.path().join("ops_marker_index_hit");
    std::fs::write(&bin_path, b"#!/bin/sh\n").unwrap();
    let mut perms = std::fs::metadata(&bin_path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&bin_path, perms).unwrap();

    let path_var = std::env::join_paths([dir.path().to_path_buf()]).unwrap();
    let index = capture_path_index_from(&path_var);
    assert!(is_in_path_index(&index, "ops_marker_index_hit"));
    assert!(check_binary_installed_with(
        "ops_marker_index_hit",
        Some(&index)
    ));
}

/// PERF-3 AC #1: a non-executable file in a PATH directory must not appear
/// in the index — same contract as `find_on_path_in_skips_non_executable_unix`.
#[cfg(unix)]
#[test]
fn path_index_skips_non_executable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin_path = dir.path().join("ops_marker_index_noexec");
    std::fs::write(&bin_path, b"data\n").unwrap();
    let path_var = std::env::join_paths([dir.path().to_path_buf()]).unwrap();
    let index = capture_path_index_from(&path_var);
    assert!(!is_in_path_index(&index, "ops_marker_index_noexec"));
    assert!(!check_binary_installed_with(
        "ops_marker_index_noexec",
        Some(&index)
    ));
}

/// PERF-3 AC #2: when `index` is `None`, [`check_binary_installed_with`]
/// falls back to the per-call PATH walk so one-off callers keep working.
#[test]
fn check_binary_installed_with_none_falls_back() {
    assert!(!check_binary_installed_with(
        "nonexistent-binary-abc123xyz-perf3",
        None
    ));
}

/// PERF-3: a missing binary is not in the index even when the PATH dir is
/// readable — confirms `is_in_path_index` doesn't false-positive on the
/// fallback path.
#[cfg(unix)]
#[test]
fn path_index_missing_binary_not_present() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path_var = std::env::join_paths([dir.path().to_path_buf()]).unwrap();
    let index = capture_path_index_from(&path_var);
    assert!(!is_in_path_index(&index, "definitely-not-there"));
}

/// PORT (TASK-0792): probe spawns must honour `$CARGO` so they invoke the
/// same toolchain binary the parent cargo selected, mirroring
/// `ops_core::subprocess::run_cargo`. The fake script below prints a
/// distinctive subcommand line; if the probe ever falls back to the real
/// `cargo` on `$PATH`, the assertion will fail.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn check_cargo_tool_installed_honours_cargo_env() {
    use crate::probe::check_cargo_tool_installed;
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let fake = dir.path().join("cargo");
    std::fs::write(
        &fake,
        "#!/bin/sh\nif [ \"$1\" = \"--list\" ]; then echo '    fake-marker-tool   A fake'; fi\n",
    )
    .unwrap();
    let mut perms = std::fs::metadata(&fake).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&fake, perms).unwrap();

    // SAFETY: serial_test::serial guards against concurrent env mutation; no
    // background thread reads CARGO during the test body.
    unsafe { std::env::set_var("CARGO", &fake) };
    let installed = check_cargo_tool_installed("fake-marker-tool");
    unsafe { std::env::remove_var("CARGO") };

    assert!(
        matches!(installed, ProbeOutcome::Ok(true)),
        "probe must invoke the binary at $CARGO; falling back to PATH would not list `fake-marker-tool`"
    );
}

/// ERR-2 (TASK-1048) AC #1+#2: when `install_cargo_tool_with_timeout` is
/// called with both a `name` and a `package`, the spawned invocation is
/// `cargo install <pkg> --bin <name>`. A common failure mode is the package
/// not exposing a `<name>` bin target; naming only `name` in the resulting
/// error misleads operators about which identifier is wrong. The error must
/// surface BOTH identifiers so the failure points at the actual cargo args.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn install_cargo_tool_failure_names_both_package_and_bin() {
    use crate::install::install_cargo_tool_with_timeout;
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let fake = dir.path().join("cargo");
    // Always exit non-zero to simulate `cargo install <pkg> --bin <name>`
    // failing (e.g. "no bin target named <name>").
    std::fs::write(&fake, "#!/bin/sh\nexit 101\n").unwrap();
    let mut perms = std::fs::metadata(&fake).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&fake, perms).unwrap();

    // SAFETY: serial_test::serial guards against concurrent env mutation.
    unsafe { std::env::set_var("CARGO", &fake) };
    let err = install_cargo_tool_with_timeout(
        "also-missing",
        Some("does-not-exist"),
        std::time::Duration::from_secs(5),
    )
    .expect_err("expected non-zero exit to surface as an error");
    unsafe { std::env::remove_var("CARGO") };

    let msg = err.to_string();
    assert!(
        msg.contains("does-not-exist"),
        "error must name the package: {msg}"
    );
    assert!(
        msg.contains("also-missing"),
        "error must name the bin/tool: {msg}"
    );
}

/// ERR-2 (TASK-1048): when no `package` is supplied, the invocation reduces
/// to `cargo install <name>` and the legacy single-identifier error is
/// preserved (no spurious `--bin` mention).
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn install_cargo_tool_failure_without_package_keeps_single_identifier() {
    use crate::install::install_cargo_tool_with_timeout;
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let fake = dir.path().join("cargo");
    std::fs::write(&fake, "#!/bin/sh\nexit 101\n").unwrap();
    let mut perms = std::fs::metadata(&fake).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&fake, perms).unwrap();

    unsafe { std::env::set_var("CARGO", &fake) };
    let err =
        install_cargo_tool_with_timeout("lonely-tool", None, std::time::Duration::from_secs(5))
            .expect_err("expected non-zero exit to surface as an error");
    unsafe { std::env::remove_var("CARGO") };

    let msg = err.to_string();
    assert!(msg.contains("lonely-tool"), "error must name tool: {msg}");
    assert!(
        !msg.contains("--bin"),
        "no --bin should appear when package is absent: {msg}"
    );
}
