---
id: TASK-0876
title: >-
  API-9: CommandOwner and DataProviderOwner are duplicated local enums in
  registry.rs
status: Done
assignee: []
created_date: '2026-05-02 09:23'
updated_date: '2026-05-02 11:00'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry.rs:133-149` and `217-232`

**What**: Two structurally identical local enums (CommandOwner, DataProviderOwner) plus their seed-and-warn loops are defined inline in register_extension_commands and register_extension_data_providers. Every owner-tracking change has to be made in two places.

**Why it matters**: DUP class - the registry-collision audit logic is the project primary defense against silent extension shadowing, and divergence between the command and data-provider paths is exactly how the asymmetry that TASK-0756 was filed to fix re-emerges. A single generic Owner<&static str> plus a shared seed helper closes that drift channel.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a RegistryOwner<a> (or generic enum Owner<E>) plus a single seed_owners/warn_collision helper used by both paths
- [ ] #2 The warn message strings remain identical (verified by the existing register_extension_commands_collision tests)
- [ ] #3 Reduces this section LOC and removes one place where command vs data behaviour can drift
<!-- AC:END -->
