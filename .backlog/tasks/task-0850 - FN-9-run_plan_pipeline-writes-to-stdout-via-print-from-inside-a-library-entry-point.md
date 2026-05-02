---
id: TASK-0850
title: >-
  FN-9: run_plan_pipeline writes to stdout via print! from inside a library
  entry point
status: Triage
assignee: []
created_date: '2026-05-02 09:16'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:36-79` (calls at 52, 57, 64, 78)

**What**: A public library function performs print! four times, returning ExitCode. There is no way to redirect output, capture it for tests, or compose the rendered tables into a different shell.

**Why it matters**: Couples the library to global stdout, prevents in-process testing of the orchestration, and forces every reuse site (LSP plugin, web UI, dry-run) to spawn a subprocess to capture output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a &mut dyn std::io::Write (or return String) so the binary writes to io::stdout() and tests/library callers can supply their own buffer
- [ ] #2 Update the binary entry point to pass &mut io::stdout().lock()
- [ ] #3 Add an integration test that exercises run_plan_pipeline against a fixed-stdin fixture and asserts the exact bytes written
<!-- AC:END -->
