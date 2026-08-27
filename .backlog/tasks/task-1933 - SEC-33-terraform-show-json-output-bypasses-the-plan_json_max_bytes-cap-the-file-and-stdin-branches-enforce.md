---
id: TASK-1933
title: >-
  SEC-33: terraform show -json output bypasses the plan_json_max_bytes cap the
  file and stdin branches enforce
status: Triage
assignee: []
created_date: '2026-08-27 15:46'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:330-344` (`run_terraform_pipeline`)

**What**: The default (no `--json-file`) branch collects the whole plan document with `Command::output()`, which buffers stdout and stderr without any limit:

    let show_output = std::process::Command::new("terraform")
        .args(["show", "-json"])
        .arg(&binary_path)
        .output()

The other two ingress points are capped at `plan_json_max_bytes()` - `read_json_file` at `:254-268` and `read_stdin_capped` at `:211-229`. The comment on `read_stdin_capped` states the intent explicitly: "both feed the same `parse_and_classify` pipeline so the cap should be uniform". The terraform branch, which is the default and most common path, is the one that is not capped.

Peak memory here is roughly 3x the document: the `Vec<u8>` from `output()`, the `String` from `String::from_utf8` at `:343`, and the `serde_json` value graph built in `parse_and_classify`. `show_output.stderr` is likewise unbounded and is interpolated whole into a `bail!` at `:336-341`.

**Why it matters**: SEC-33 asks for uniform bounds on external input. The gap means the documented `OPS_PLAN_JSON_MAX_BYTES` control silently does nothing for the default invocation, and a very large stack (or a compromised/wrapped `terraform` on `PATH`) can OOM the CLI. The inconsistency is also a correctness trap: a reviewer reading the cap code would conclude all three paths are covered.

**Suggested fix**: spawn with piped stdio and read stdout through the same capped `Read` helper used by `read_stdin_capped`, and truncate stderr to a fixed number of bytes before it reaches the error message.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The terraform show -json branch enforces plan_json_max_bytes on the captured stdout, using the same helper as read_stdin_capped
- [ ] #2 Captured stderr is truncated to a documented fixed byte budget before being used in an error message
- [ ] #3 A test drives the capped read helper over an oversized reader and asserts the error names the cap and the OPS_PLAN_JSON_MAX_BYTES override
<!-- AC:END -->
