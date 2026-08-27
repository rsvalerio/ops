---
id: TASK-1786
title: >-
  ARCH-11: ops-about-terraform declares `anyhow` and `ops-git` dependencies it
  never uses
status: Triage
assignee: []
created_date: '2026-08-27 11:23'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions-terraform/about/Cargo.toml
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/Cargo.toml:12` (`ops-git`) and `:15` (`anyhow`)

**What**: Both crates are declared in `[dependencies]` and neither appears anywhere in the crate. `grep -n "anyhow\|ops_git" extensions-terraform/about/src/lib.rs` returns nothing; the only external types used are `ops_about`, `ops_core`, `ops_extension`, `serde_json` (in the `provide` signature) and `tracing`. `linkme` *is* required, indirectly — `ops_extension::impl_extension!` expands to `#[linkme::distributed_slice(...)]` — so it must stay.

The git-remote repository fallback the crate relies on is implemented inside `ops_about::identity::build_identity_value`, which carries its own `ops-git` dependency; this crate never touches it.

**Why it matters**: Unused direct dependencies are compile-time cost and a false signal about what this crate is coupled to. `anyhow` in particular reads as "this crate propagates `anyhow::Result`" to anyone auditing error policy, when in fact its only error type is the typed `DataProviderError` — precisely the distinction ERR-2 cares about. A reviewer trusting the manifest will look for git usage that does not exist.

**Cross-crate note**: the identical pair of unused declarations exists in every sibling about crate (`extensions-go`, `extensions-node`, `extensions-python`, `extensions-java`, `extensions-rust`) — TASK-1738 covers `ops-about-go`. This task is scoped to the terraform manifest; a sweep across all six would be a reasonable way to land them together, but each crate must be verified separately since some siblings may genuinely use them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 anyhow and ops-git are removed from extensions-terraform/about/Cargo.toml [dependencies] after confirming they are unused
- [ ] #2 linkme is retained (required by the impl_extension macro expansion)
- [ ] #3 cargo build -p ops-about-terraform and cargo test -p ops-about-terraform pass
<!-- AC:END -->
