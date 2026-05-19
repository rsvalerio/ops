---
id: TASK-1569
title: >-
  DUP-1: install_cargo_tool_with_timeout and
  install_rustup_component_with_timeout duplicate spawn-and-timeout scaffold
status: Triage
assignee: []
created_date: '2026-05-19 16:11'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/install.rs:62-113` (cargo path) and `extensions-rust/tools/src/install.rs:119-141` (rustup path)

**What**: both installer helpers open-code the same shape:

```rust
let child = Command::new(resolve_*_bin())
    .args(...)
    .stdin(Stdio::null())
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .spawn()
    .context("failed to spawn ...")?;
let status = run_with_timeout(child, timeout, &format!("..."))?;
if status.success() { Ok(()) } else { anyhow::bail!("... failed") }
```

The CONC-3 / CONC-5 rationale comment block is even repeated verbatim (`install.rs:79-89` vs `install.rs:126-127` referring back). Differences between the two callers are only:
1. binary (`resolve_cargo_bin` vs `resolve_rustup_bin`)
2. argv (`["install", pkg, "--bin", name]` vs `["component", "add", component, "--toolchain", toolchain]`)
3. spawn-context label and failure-message shape

DUP-1 threshold: 5+ identical lines repeated; here ~10 lines of process-launch boilerplate repeat with the only meaningful variation being argv. Both helpers will need synchronised edits any time the stdio policy changes (e.g. adding `Stdio::piped()` on stderr for log capture).

**Why it matters**:
- The CONC-3 deadlock rationale and the CONC-5 stdin-null rationale are safety properties — drift between the two sites (e.g. someone changing stderr to `Stdio::piped()` on one path only) silently re-introduces the pipe-deadlock referenced in TASK-0650.
- New install paths (e.g. a future `cargo +nightly install`, or a `cargo-binstall` shortcut) will copy this scaffold again, compounding the drift surface.

**Fix sketch**: extract `fn spawn_install_subprocess(bin: PathBuf, args: &[&str], timeout: Duration, label: &str) -> anyhow::Result<()>` that owns the stdio policy and timeout, returning `Ok(())` on success and a structured `InstallFailure { argv: Vec<String> }` error on non-zero exit. Both call sites become 4-5 lines: build argv, build a label string, call the helper, format the failure-message overlay.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A shared spawn-and-timeout helper owns the CONC-3 / CONC-5 stdio policy
- [ ] #2 install_cargo_tool_with_timeout and install_rustup_component_with_timeout consume the helper
- [ ] #3 Existing install-failure tests (install_cargo_tool_failure_names_both_package_and_bin, install_cargo_tool_failure_without_package_keeps_single_identifier) continue to pass
<!-- AC:END -->
