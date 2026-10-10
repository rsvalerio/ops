---
id: TASK-2504
title: 'API-15: ops-tfplan declares no rust-version despite the workspace defining one to inherit'
status: To Do
assignee: []
created_date: '2026-10-10 15:31'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions-terraform/plan/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'API-15:extensions-terraform/plan/Cargo.toml:package'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/Cargo.toml:1`

**What**: The manifest inherits version, edition and license from [workspace.package] but not rust-version, and declares no value of its own — so ops-tfplan, a library consumed by crates/cli, has no MSRV at all. The workspace root pins rust-version = "1.97" (root Cargo.toml line 11), but that binds only the root package: a member without the key has an accidental MSRV that floats with whatever toolchain last compiled it. Sibling grep confirms no extensions-*/extensions-rust crate inherits it either, so the gap is uniform — this task covers the ops-tfplan instance only.

**Why it matters**: API-15 — a library with no declared MSRV has one anyway, it is just implicit and changes without a decision. The crate already relies on APIs that make the MSRV load-bearing (let-else, let_chains-free but IsTerminal, u64::try_from, Option::is_some_and), and every rule that names a stabilization version is gated on the declared MSRV, which this crate does not have. The fix is one line: rust-version.workspace = true in [package], raised per workspace policy in minor versions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extensions-terraform/plan/Cargo.toml declares rust-version (rust-version.workspace = true inheriting the workspace 1.97, or an explicit value) and cargo check -p ops-tfplan still passes
<!-- AC:END -->
