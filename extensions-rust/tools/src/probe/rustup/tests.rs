//! Tests for toolchain probing and `rustup component list` parsing.

use super::*;
use crate::probe::ProbeOutcome;

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

/// PATTERN-1 / TASK-1566: rustup ≥1.28 emits `"no active toolchain configured\n"`
/// (no `error:` prefix). The first whitespace-bounded token is `no`, which is
/// not a real toolchain identifier — the previous parser surfaced it as
/// `Some("no")` and the misleading test name (`*_rejects_*`) pinned the bug.
/// The fix requires the returned token to carry at least one of `-`/`.`/`:`,
/// so bare status words are rejected.
#[test]
fn parse_active_toolchain_rejects_no_active_toolchain_message_returns_none() {
    // rustup ≥1.28 "no active toolchain" output: must yield None, not Some("no").
    assert_eq!(
        parse_active_toolchain("no active toolchain configured\n"),
        None
    );
    // The colon-containing diagnostic variant
    assert_eq!(
        parse_active_toolchain("error: toolchain 'nonexistent' is not installed\n"),
        None
    );
}

/// ERR-1 / TASK-1619 AC#3: probe-failed branch (rustup didn't answer at
/// all — timeout, spawn IO, etc.) maps to `ProbeFailed`, distinct from
/// "answered but no toolchain configured".
#[test]
fn classify_active_toolchain_probe_failed_branch() {
    assert!(matches!(
        classify_active_toolchain(None),
        crate::ActiveToolchain::ProbeFailed
    ));
}

/// ERR-1 / TASK-1619 AC#3: rustup answered with a non-zero exit status —
/// treated as `ProbeFailed` (the answer is not trustworthy), distinct from
/// the "no active toolchain configured" case.
#[test]
fn classify_active_toolchain_non_zero_exit_is_probe_failed() {
    assert!(matches!(
        classify_active_toolchain(Some((false, ""))),
        crate::ActiveToolchain::ProbeFailed
    ));
    assert!(matches!(
        classify_active_toolchain(Some((false, "stable-aarch64-apple-darwin\n"))),
        crate::ActiveToolchain::ProbeFailed
    ));
}

/// ERR-1 / TASK-1619 AC#3: rustup answered, but no toolchain identifier
/// was parseable from stdout (rustup ≥1.28 "no active toolchain
/// configured\n"). Distinct from probe failure: operators should be told
/// to run `rustup default`, not asked to check their rustup install.
#[test]
fn classify_active_toolchain_no_toolchain_configured_is_none() {
    assert!(matches!(
        classify_active_toolchain(Some((true, "no active toolchain configured\n"))),
        crate::ActiveToolchain::None
    ));
    assert!(matches!(
        classify_active_toolchain(Some((true, ""))),
        crate::ActiveToolchain::None
    ));
}

/// ERR-1 / TASK-1619 AC#3: rustup answered with a real toolchain
/// identifier — Resolved.
#[test]
fn classify_active_toolchain_resolved_branch() {
    let resolved = classify_active_toolchain(Some((true, "stable-aarch64-apple-darwin\n")));
    match resolved {
        crate::ActiveToolchain::Resolved(t) => {
            assert_eq!(t, "stable-aarch64-apple-darwin");
        }
        other => panic!("expected Resolved, got {other:?}"),
    }
}

/// PATTERN-1 / TASK-1566: additional bare-word rustup status forms must
/// also return None — the parser must not be tricked into emitting them as
/// fake toolchain names downstream of `rustup component add --toolchain ...`.
#[test]
fn parse_active_toolchain_rejects_other_bare_word_status_forms() {
    assert_eq!(parse_active_toolchain("none configured\n"), None);
    assert_eq!(parse_active_toolchain("unknown\n"), None);
    assert_eq!(parse_active_toolchain("none\n"), None);
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

/// PATTERN-1 / TASK-1583 AC#2: lock the contract on a non-baseline
/// triple so a future regression in `RUSTUP_TARGET_ARCH_PATTERNS` (e.g.
/// dropping `-wasm32-`) lights up here, not in operator output.
#[test]
fn component_list_strips_wasm32_target_triple() {
    let stdout = "rust-std-wasm32-unknown-unknown\nrust-std-wasm32v1-none\n";
    assert!(is_component_in_list(stdout, "rust-std"));
}

#[test]
#[ignore = "requires rustup installed; run with: cargo test -- --ignored"]
fn get_active_toolchain_returns_some() {
    let tc = get_active_toolchain();
    let resolved = match tc {
        crate::ActiveToolchain::Resolved(t) => t,
        other => panic!("rustup should resolve a toolchain in dev environment, got {other:?}"),
    };
    let tc = resolved;
    assert!(
        !tc.is_empty(),
        "toolchain string should not be empty, got: {tc}"
    );
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
#[serial_test::serial]
fn check_rustup_component_installed_nonexistent() {
    assert!(matches!(
        check_rustup_component_installed("nonexistent-component-xyz"),
        ProbeOutcome::Ok(false) | ProbeOutcome::Failed
    ));
}
