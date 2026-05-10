---
id: TASK-0850
title: >-
  FN-9: run_plan_pipeline writes to stdout via print! from inside a library
  entry point
status: Done
assignee: []
created_date: '2026-05-02 09:16'
updated_date: '2026-05-02 14:19'
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
- [x] #1 Add a &mut dyn std::io::Write (or return String) so the binary writes to io::stdout() and tests/library callers can supply their own buffer
- [x] #2 Update the binary entry point to pass &mut io::stdout().lock()
- [x] #3 Add an integration test that exercises run_plan_pipeline against a fixed-stdin fixture and asserts the exact bytes written
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added pub fn run_plan_pipeline_to(opts, &mut dyn Write) -> Result<ExitCode> with the four print! sites converted to write!(out, ...). The existing run_plan_pipeline now locks io::stdout() and delegates, preserving the public signature so the binary entry point at crates/cli/src/main.rs:220 stays unchanged. Added run_plan_pipeline_to_writes_to_supplied_buffer test driving the minimal.json fixture into a Vec<u8> and asserting the rendered tables.
<!-- SECTION:NOTES:END -->
