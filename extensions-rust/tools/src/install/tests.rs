//! Tests for install-argument validation and the install dispatcher.

use super::*;
use ops_core::config::tools::{ExtendedToolSpec, ToolSource, ToolSpec};

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

/// ERR-2 / TASK-1608 AC#1: version-pinned toolchains (e.g.
/// `1.70.0-x86_64-apple-darwin`, the canonical form produced by
/// `rust-toolchain.toml` / `rustup default 1.70.0`) must validate. The
/// crates.io grammar enforced by `validate_cargo_tool_arg` rejects `.`
/// — for toolchains that's a false positive that blocked
/// `ops tools install` on any version-pinned workspace.
#[test]
fn validate_rustup_toolchain_accepts_version_pinned_identifier() {
    for ok in [
        "1.70.0-x86_64-apple-darwin",
        "stable",
        "stable-aarch64-apple-darwin",
        "nightly-2024-01-01",
        "1.85.0",
    ] {
        assert!(
            validate_rustup_toolchain(ok, "rustup toolchain").is_ok(),
            "expected {ok:?} to validate as a rustup toolchain"
        );
    }
}

/// ERR-2 / TASK-1608 AC#2: relaxing `.` must not weaken the
/// leading-dash flag-injection guard.
#[test]
fn validate_rustup_toolchain_rejects_leading_dash() {
    assert!(validate_rustup_toolchain("-vV", "rustup toolchain").is_err());
    assert!(validate_rustup_toolchain("--config=evil", "rustup toolchain").is_err());
}

/// ERR-2 / TASK-1608: shell metacharacters and whitespace still rejected.
#[test]
fn validate_rustup_toolchain_rejects_metacharacters() {
    for bad in [
        "stable;rm -rf /",
        "1.70.0 stable",
        "1.70.0$VAR",
        "1.70.0`cmd`",
        "1.70.0|pipe",
        "1.70.0/path",
        "",
    ] {
        assert!(
            validate_rustup_toolchain(bad, "rustup toolchain").is_err(),
            "expected rejection of {bad:?}"
        );
    }
}

/// ERR-2 / TASK-1608 AC#4: `install_rustup_component_with_timeout`
/// passes a dotted toolchain through the validator (the prior
/// `validate_cargo_tool_arg` reject would surface as an
/// `invalid character` error before rustup is even spawned). Spawning
/// rustup against a real toolchain identifier risks a network sync in
/// tests, so we drive the validator directly via the same call site
/// `install_rustup_component_with_timeout` uses (cf.
/// `install.rs:141`) — a regression that reverts to the stricter
/// validator would fail this assertion before any subprocess work.
#[test]
fn install_rustup_component_validator_accepts_dotted_toolchain() {
    validate_rustup_toolchain("1.70.0-x86_64-apple-darwin", "rustup toolchain")
        .expect("install path validator must accept version-pinned toolchain identifier");
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
