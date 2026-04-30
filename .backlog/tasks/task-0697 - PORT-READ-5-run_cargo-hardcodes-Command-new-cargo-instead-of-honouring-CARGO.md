---
id: TASK-0697
title: >-
  PORT/READ-5: run_cargo hardcodes Command::new("cargo") instead of honouring
  $CARGO
status: To Do
assignee:
  - TASK-0743
created_date: '2026-04-30 05:26'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:239-250`

**What**: `run_cargo` builds `Command::new("cargo")` directly. Cargo subcommands (which this is invoked from, since callers are `cargo ops *` extensions) inherit `$CARGO` from the parent cargo process pointing at the exact toolchain binary that drove the invocation. Hardcoding the literal string forces a fresh `$PATH` lookup that may resolve to a different rustup toolchain than the one running the parent — particularly under `cargo +nightly ops <cmd>` or in vendored toolchain layouts.

**Why it matters**: Reproducing toolchain mismatches reported by users requires aligning child cargo with parent. Standard Cargo plugins (clippy, llvm-cov) honour `$CARGO`; ops should too so subcommand + nested tooling stays on the same toolchain.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Resolve cargo binary via std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into())
- [ ] #2 Add a regression test asserting CARGO is honoured when set
- [ ] #3 Document the precedence in run_cargo doc comment
<!-- AC:END -->
