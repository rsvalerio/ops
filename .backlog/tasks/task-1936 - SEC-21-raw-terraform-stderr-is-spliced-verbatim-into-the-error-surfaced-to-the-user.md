---
id: TASK-1936
title: >-
  SEC-21: raw terraform stderr is spliced verbatim into the error surfaced to
  the user
status: Done
assignee:
  - TASK-2002
created_date: '2026-08-27 15:47'
updated_date: '2026-08-28 21:30'
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
**File**: `extensions-terraform/plan/src/lib.rs:336-341`

**What**:

    if !show_output.status.success() {
        bail!(
            "terraform show -json failed: {}",
            String::from_utf8_lossy(&show_output.stderr)
        );
    }

The full stderr of `terraform show -json` becomes the text of an `anyhow::Error` that the CLI prints and that any caller may log. Terraform diagnostics routinely echo the offending value back to the operator - invalid provider credentials, `-var` values, backend connection strings, and sensitive attributes named in a validation failure. Unlike the `terraform plan` invocation at `:296-298`, which inherits stderr straight to the user's terminal and is never captured into a Rust string, this one turns provider output into a program-level error string that flows wherever errors flow.

**Why it matters**: SEC-21 forbids secrets and internal detail in error messages and error chains. This crate's own error handling is otherwise careful (see the `ERR-4` note at `:307-309` about preserving `source()` rather than flattening), which makes this the outlier. It is also unbounded in length (see the SEC-33 finding on the same function).

**Suggested fix**: keep the exit status in the message, route the stderr text to `tracing::debug!` (matching how `cleanup_artifacts` at `:372-376` routes operator-only detail), or truncate and redact before interpolating.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The user-facing error for a failed terraform show names the exit status without embedding raw provider stderr
- [x] #2 The stderr text, if retained at all, goes to a tracing event rather than the anyhow error message, and is length-bounded
- [x] #3 A test asserts the error message for a failed show does not contain the captured stderr body
<!-- AC:END -->
