---
id: TASK-1930
title: >-
  SEC-29: plan JSON and .ops artifact directory are created with default
  world-readable permissions
status: Triage
assignee: []
created_date: '2026-08-27 15:46'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:276-281` (`create_dir_all`) and `:346-348` (`std::fs::write`)

**What**: `run_terraform_pipeline` creates the artifact directory with `std::fs::create_dir_all(parent)` (mode 0777 and umask, typically 0755) and, under `--keep-plan`, writes the full plan JSON with `std::fs::write(&json_path, &json_str)` (mode 0666 and umask, typically 0644). Nothing narrows the mode.

Terraform plan JSON is one of the most secret-dense artefacts a stack produces: `after` values for generated passwords and keys, provider configuration, and output values. On a shared build host, a multi-tenant CI runner, or any box with other local accounts, every local user can read `.ops/tfplan.json`. The same applies to `.ops/tfplan.binary`, which terraform itself writes into the directory this code creates.

**Why it matters**: SEC-29 requires that files holding sensitive material are not world-readable. This is a passive disclosure with no attacker interaction required - the artefact just sits there readable. It compounds with the fact that `.ops/` is not in the repo `.gitignore`.

**Suggested fix**: on unix, create the directory with `std::fs::DirBuilder` + `mode(0o700)` and write the JSON through `OpenOptions::new().mode(0o600).write(true).create(true).truncate(true)`, behind a `#[cfg(unix)]` helper. Consider tightening `.ops/tfplan.binary` after terraform writes it, and adding `.ops/` to `.gitignore`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On unix the artifact directory is created with mode 0700 and the plan JSON file with mode 0600
- [ ] #2 A unix-gated test asserts the written plan JSON has permissions 0600 and the created directory 0700
- [ ] #3 The chosen permissions are documented next to the write with a one-line reason referencing plan-JSON secrecy
<!-- AC:END -->
