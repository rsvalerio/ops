---
id: TASK-1945
title: >-
  ERR-13: three std::fs calls in run_terraform_pipeline propagate a bare
  io::Error with no path
status: To Do
assignee:
  - TASK-2002
created_date: '2026-08-27 15:48'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:276-281` and `:346-348`

**What**: Three filesystem calls use a naked `?`:

    if let Some(parent) = binary_path.parent() {
        std::fs::create_dir_all(parent)?;          // :277
    }
    if let Some(parent) = json_path.parent() {
        std::fs::create_dir_all(parent)?;          // :280
    }
    ...
    if opts.keep_plan {
        std::fs::write(&json_path, &json_str)?;    // :347
    }

The user sees "Permission denied (os error 13)" with no indication of which directory or file, and no way to tell the binary-plan parent from the JSON parent - the two `create_dir_all` calls are indistinguishable in the output. Both paths come from user-supplied `--out` / `--json-out` after shell expansion, so a mistyped or unexpanded path is exactly the case that produces these errors.

Every other filesystem and process call in this file already carries context - `read_json_file` at `:248-260`, the terraform invocations at `:309` and `:334`. These three are the outliers.

**Why it matters**: ERR-13 requires filesystem errors that can reach a user or a CI log to name the path. Without it the message is not actionable.

**Suggested fix**: `.with_context(|| format!("creating plan artifact directory {}", parent.display()))` and the equivalent for the write, or switch this module to `fs_err`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both create_dir_all calls and the plan JSON write attach context naming the path involved
- [ ] #2 The two create_dir_all messages are distinguishable from each other
- [ ] #3 A test asserts the error message for an unwritable artifact path contains that path
<!-- AC:END -->
