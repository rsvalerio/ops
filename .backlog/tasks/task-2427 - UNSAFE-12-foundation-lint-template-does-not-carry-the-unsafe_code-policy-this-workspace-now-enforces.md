---
id: TASK-2427
title: 'UNSAFE-12: foundation lint template does not carry the unsafe_code policy this workspace now enforces'
status: Triage
assignee: []
created_date: '2026-10-04 15:28'
labels:
  - code-review-rust
  - unsafe
dependencies: []
modified_files:
  - extensions-rust/foundation/templates/lints.toml
  - docs/foundation.md
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/templates/lints.toml:14`

**What**: The root `Cargo.toml` now sets `unsafe_code = "deny"` in `[workspace.lints.rust]` (TASK-2368), but the lint table `ops init --rust` writes into other repositories still lists only `elided_lifetimes_in_paths`, `unsafe_op_in_unsafe_fn` and `unused_lifetimes`. The template and the workspace it was derived from now disagree on whether unsafe is policed.

**Why it matters**: A scaffolded repository gets no mechanical unsafe policy, and `ops init --rust --check` does not report its absence. Adding it is a policy decision for every adopter (a repo with FFI needs scoped exceptions first), so it was not done inside the wave.

**Origin**: discovered during TASK-2417 while fixing TASK-2368.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Decide whether the foundation template sets unsafe_code (and at which level); either add it with the drift check and docs updated, or record in docs/foundation.md why it is left to each repository
<!-- AC:END -->
