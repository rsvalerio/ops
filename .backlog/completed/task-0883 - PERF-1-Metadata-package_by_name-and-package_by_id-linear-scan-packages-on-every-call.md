---
id: TASK-0883
title: >-
  PERF-1: Metadata::package_by_name and package_by_id linear-scan packages on
  every call
status: Done
assignee: []
created_date: '2026-05-02 09:37'
updated_date: '2026-05-02 11:05'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:183-190`

**What**: `package_by_name` and `package_by_id` both call `self.packages().find(...)`, doing an O(n) iteration over the full `packages` JSON array on every lookup. For a workspace with hundreds of dependencies (cargo metadata for the openbao/k8s.io class of project), repeated lookups across providers (about/units/coverage/deps) all pay this cost. The crate already builds `OnceLock<HashSet<String>>` for `member_ids` / `default_member_ids` (TASK-0477) — the same lazy-built indexed lookup is missing for the by-name / by-id paths.

**Why it matters**: `package_by_name` is the only ergonomic accessor exposed for caller-driven lookups (DataProviderSchema even advertises it in the public schema), so callers naturally use it in loops. A 500-package workspace × N providers × M lookups per provider amplifies a per-provide() cost from ~µs to ~ms. Add a lazily-built `OnceLock<HashMap<String, usize>>` keyed by name and id pointing at the index in `packages[]`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 package_by_name/package_by_id are O(1) average-case after first call
- [ ] #2 lazy index follows the OnceLock pattern already used for member_ids
<!-- AC:END -->
