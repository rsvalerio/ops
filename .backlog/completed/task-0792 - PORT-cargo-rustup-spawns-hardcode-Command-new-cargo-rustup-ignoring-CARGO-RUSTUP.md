---
id: TASK-0792
title: >-
  PORT: cargo/rustup spawns hardcode Command::new("cargo")/("rustup"), ignoring
  $CARGO/$RUSTUP
status: Done
assignee:
  - TASK-0821
created_date: '2026-05-01 05:59'
updated_date: '2026-05-01 06:45'
labels:
  - code-review-rust
  - portability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:123`, `extensions-rust/tools/src/install.rs:74,102`, `extensions-rust/tools/src/probe.rs:12,52,206,342,351`

**What**: After TASK-0697 fixed core::subprocess::run_cargo, several extension-side spawns still call Command::new("cargo") and Command::new("rustup") directly: deps::check_tool, tools::install_*, every tools::probe::* helper. None consult std::env::var_os("CARGO") / var_os("RUSTUP").

**Why it matters**: Under `cargo +nightly ops …` or vendored rustup layouts, the parent cargo sets $CARGO to the exact toolchain binary that ran. Hardcoding the bare name forces a fresh $PATH lookup that can resolve a different toolchain — exactly the mismatch TASK-0697 fixed for run_cargo.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Resolve cargo via std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()) (and rustup via RUSTUP) at every Command::new site in deps, tools/install, tools/probe
- [ ] #2 Keep behaviour identical when the env var is unset (fallback to literal name)
- [ ] #3 Add a regression test that sets CARGO to a fake binary path and confirms the probe / install logic invokes that path
<!-- AC:END -->
