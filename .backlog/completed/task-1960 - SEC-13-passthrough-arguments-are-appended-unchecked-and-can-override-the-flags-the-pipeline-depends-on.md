---
id: TASK-1960
title: >-
  SEC-13: passthrough arguments are appended unchecked and can override the
  flags the pipeline depends on
status: Done
assignee:
  - TASK-2002
created_date: '2026-08-27 15:51'
updated_date: '2026-08-28 21:31'
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
**File**: `extensions-terraform/plan/src/lib.rs:283-294`, `:313-328`, `:330-333`

**What**: `plan_cmd.args(&opts.passthrough)` appends user-supplied arguments *after* the flags this pipeline relies on, with no validation of what they contain:

    plan_cmd.arg("plan")
        .arg(format!("-out={}", binary_path.display()))
        .arg("-input=false")
        .arg("-no-color");
    if opts.detailed_exitcode { plan_cmd.arg("-detailed-exitcode"); }
    plan_cmd.args(&opts.passthrough);

Terraform takes the last occurrence of a repeated flag, so a passthrough argument silently wins over the pipeline's own:

- `ops plans -- -detailed-exitcode` (without the `--detailed-exitcode` ops flag) makes terraform exit 2 on a plan with changes. The `else if !status.success()` branch at `:323` then reports `terraform plan failed with exit code 2` for a perfectly successful plan.
- `ops plans -- -out=elsewhere.tfplan` redirects the binary plan, after which `terraform show -json` at `:330-333` still reads `binary_path`. If a previous run left an artefact there (see the SEC-32 finding - artefacts survive every error path), the operator is shown a **stale plan rendered as the current one**, with no indication anything is wrong.
- `-input=true` re-enables interactive prompts under a `Stdio::null()` stdout.

To be clear about what is *not* wrong: `Command::args` is used correctly - there is no shell, so this is not shell injection. The defect is the absence of any check that the caller is not overriding flags the surrounding logic assumes.

**Why it matters**: SEC-13 covers sanitising the arguments handed to a subprocess. The stale-plan case is the serious one: the entire purpose of this command is to show an operator what an apply will do, and this path can show them a different plan than the one on disk.

**Suggested fix**: reject passthrough arguments that set `-out`, `-input`, `-detailed-exitcode` or `-json` with a clear error naming the ops flag to use instead, and document the reservation in the `--` help text at `:54-56`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Passthrough arguments that set -out, -input, -detailed-exitcode or -json are rejected with an error naming the equivalent ops flag
- [x] #2 The passthrough help text states which terraform flags are reserved
- [x] #3 A test asserts each reserved flag in passthrough produces an error before terraform is invoked
<!-- AC:END -->
