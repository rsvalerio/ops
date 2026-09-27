---
id: TASK-2318
title: 'TEST: tfplan hung-terraform stub tests flake with ETXTBSY (Text file busy) under parallel test execution'
status: Triage
assignee: []
created_date: '2026-09-27 15:46'
labels:
  - code-review-rust
  - test
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:1334` and `:1398` (`a_hung_terraform_plan_is_killed_cleaned_up_and_reported`, `a_hung_terraform_show_is_killed_cleaned_up_and_reported`; the same stub-script pattern is used at ~1246-1352)

**What**: Each test writes a shell stub with `std::fs::write` + `set_permissions(0o755)` and then execs it. With cargo's parallel test threads, a sibling test's `fork()` can briefly inherit the stub's still-open write descriptor (between fork and exec), so the exec of the stub fails with `Text file busy (os error 26)`. Observed once: `failed to run terraform plan: Text file busy (os error 26)` from `a_hung_terraform_show_is_killed_cleaned_up_and_reported`; passed on rerun.

**Why it matters**: Intermittent failures in `cargo test -p ops-tfplan` erode trust in the gate and cost reruns.

**Origin**: discovered during TASK-2315 while fixing TASK-2313 (test run to confirm comfy-table 7.2.2 behaviour was unchanged; unrelated to the dependency bump).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Stub-exec tests in ops-tfplan no longer fail with ETXTBSY (e.g. retry exec on ETXTBSY, write stubs before any test forks, or serialise stub write+exec)
<!-- AC:END -->
