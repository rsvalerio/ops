---
id: TASK-2209
title: >-
  SEC-33: terraform subprocesses run with no wall-clock bound, so every resource
  in the plan pipeline is capped except the one an unresponsive provider
  actually exhausts
status: To Do
assignee:
  - TASK-2235
created_date: '2026-09-08 07:20'
updated_date: '2026-09-08 10:54'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:519-541` (`run_terraform_plan`), `extensions-terraform/plan/src/lib.rs:600-625` (`capture_plan_json`)

**What**: The pipeline bounds every byte-valued resource it touches — plan JSON via `plan_json_max_bytes()` / `read_capped`, terraform stderr via `TERRAFORM_STDERR_MAX_BYTES` plus a drain thread — but neither child process has a timeout. `run_terraform_plan` blocks in `plan_cmd.status()` and `capture_plan_json` blocks in `read_capped(&mut stdout, ...)` and then `child.wait()`, all indefinitely.

The crate's own doc comments name the threat model explicitly: "a wrapped `terraform` on `PATH`" (`read_capped`), "a chatty or hostile `terraform` on `PATH`" (`TERRAFORM_STDERR_MAX_BYTES`). Against that adversary, and against the far more common benign case of a provider blocked on an unreachable backend or a hung credential-helper prompt, wall-clock is the unbounded resource. A `terraform` that opens its pipes and then never writes wedges `ops plans` forever with no diagnostic and no artifact cleanup — `with_artifact_cleanup` only runs once `plan_pipeline_body` returns, so a hang also strands the secret-dense `.ops/tfplan.binary` on disk for the duration.

`wait-timeout` is already a workspace dependency (see the completed TASK-1039), so the bound can be added without a new dep.

**Why it matters**: `ops plans` is a CI gate — the `--detailed-exitcode` contract exists precisely so pipelines can branch on it. A hang there consumes a runner slot until the CI-level job timeout kills it, which produces a generic "job timed out" with none of the context that a "`terraform plan` produced no output for N seconds" error would carry. It also defeats the SEC-32 cleanup guarantee: the artifact this crate goes to considerable lengths to delete on every exit path survives, at whatever mode terraform left it, for as long as the hang lasts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both terraform invocations are bounded by a configurable wall-clock timeout with a documented default and an env override, in the same style as OPS_PLAN_JSON_MAX_BYTES
- [ ] #2 On timeout the child is killed and reaped, the error names which terraform invocation timed out and the limit that fired, and artifact cleanup still runs
- [ ] #3 A test drives the timeout path with a stub child that never exits and asserts the error message plus that the recorded artifact was removed
<!-- AC:END -->
