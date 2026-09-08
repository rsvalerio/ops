---
id: TASK-2225
title: >-
  SEC-25: write_plan_json follows a symlink at the plan-JSON path, writing the
  stack's secrets through it and chmod-ing the attacker's target to 0600
status: To Do
assignee:
  - TASK-2235
created_date: '2026-09-08 07:22'
updated_date: '2026-09-08 10:53'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:430-446` (`write_plan_json`)

**What**: The plan JSON is opened with `OpenOptions::new().write(true).create(true).truncate(true)` — no `create_new`, no `O_NOFOLLOW`. If a symlink already sits at the destination, the open resolves through it and the pipeline writes the full plan document (the `after` values of generated passwords and keys, plus complete provider configuration — the crate's own SEC-29 comment says so) to whatever the link points at. The subsequent `file.set_permissions(0o600)` then applies to the *target*, so the operation also silently re-modes a file the run does not own.

The 0700 artifact directory does not close this. `create_artifact_dir` sets `mode(0o700)` via `DirBuilder`, which applies **only to directories that call actually creates**. `.ops/` is the shared ops workspace directory and normally already exists, created by other tooling under the user's umask — typically 0755. On a shared build host or a multi-tenant runner, any other local account can then plant `.ops/tfplan.json -> /tmp/exfil` (or `-> ~victim/.ssh/authorized_keys`) before the run. `--json-out` widens this further to any path the operator names.

Note the asymmetry with the rest of the crate: `cleanup_artifacts` was deliberately hardened against check-then-act (TASK-1942) and `harden_artifact_permissions` was moved ahead of the exit-status check (TASK-1930), but the one place this crate *creates* a secret-bearing file itself still trusts the path.

**Why it matters**: OWASP A01. This is a write-primitive: an unprivileged local account converts a routine `ops plans --keep-plan` into an arbitrary-file overwrite performed with the operator's credentials, and gets the stack's generated secrets delivered to a location it controls. The mode hardening the code performs afterwards actively helps the attacker by making the exfiltrated copy look deliberately protected.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The plan JSON is created without following a pre-existing symlink (O_NOFOLLOW on unix, or create_new into the artifact directory), and an existing symlink at the destination is a hard error naming the path rather than a silent write-through
- [ ] #2 set_permissions is applied to the descriptor this run created, never to a path resolved a second time
- [ ] #3 The artifact directory's mode is verified (not merely requested) before a secret-bearing artifact is written into a directory that already existed
- [ ] #4 A unix test plants a symlink at the --json-out destination and asserts the run errors and the link target is neither written nor chmod-ed
<!-- AC:END -->
