---
id: TASK-0832
title: >-
  API-9: Public terraform plan model types lack #[non_exhaustive], freezing JSON
  schema as breaking-change surface
status: Triage
assignee: []
created_date: '2026-05-02 09:11'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/model.rs:3-33`

**What**: `Plan`, `ResourceChange`, `Change`, and `Action` are all `pub` with public fields/variants, none marked `#[non_exhaustive]`. They are deserialization shapes for the upstream Terraform plan JSON, which Hashicorp evolves (e.g., the `forget` action introduced in Terraform 1.7).

**Why it matters**: Any downstream consumer that destructures these structs or exhaustively matches `Action` will break the moment a new field/variant is added. The companion crates follow the project rule (e.g., AboutPythonExtension uses #[non_exhaustive]); these do not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add #[non_exhaustive] to Plan, ResourceChange, Change, ClassifiedChange, and Action
- [ ] #2 Update internal constructors/tests to use struct-update syntax where needed
- [ ] #3 Confirm cargo semver-checks (or equivalent) reports no externally observable break
<!-- AC:END -->
