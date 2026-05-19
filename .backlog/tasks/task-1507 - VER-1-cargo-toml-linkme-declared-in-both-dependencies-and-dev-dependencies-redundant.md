---
id: TASK-1507
title: >-
  VER-1: cargo-toml linkme declared in both [dependencies] and
  [dev-dependencies] (redundant)
status: To Do
assignee:
  - TASK-1573
created_date: '2026-05-18 19:14'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - VER
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/Cargo.toml:11,21`

**What**: `linkme = { workspace = true }` appears in both `[dependencies]` (line 11) and `[dev-dependencies]` (line 21). Because Cargo automatically includes `[dependencies]` in the test/dev compilation graph, the dev-dependencies entry is redundant — there is no test-only consumer of linkme that needs a different feature set, and `linkme` is not referenced directly anywhere under `src/` (the only usage is via the `impl_extension!` macro expansion in `lib.rs`, which is part of the library build, not test-only).

Confirmed by `grep -rn linkme src/` → no hits in this crate; the macro path is `ops_extension::impl_extension!`, whose expansion already pulls `linkme` through the main `[dependencies]` entry.

**Why it matters**: Duplicate dependency entries are a maintenance hazard: a future feature-flag bump on one line silently desynchronises the other (the dev build compiles linkme with one feature set, the lib build with another, and `cargo test` mysteriously fails). It also enlarges the Cargo.toml's surface area for reviewers and tooling that diff manifests.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 linkme is removed from [dev-dependencies] OR a comment is added explaining why both entries are required (e.g. divergent test-only feature set)
- [ ] #2 cargo test --workspace -p ops-cargo-toml still passes after the change
<!-- AC:END -->
