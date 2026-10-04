---
id: TASK-2377
title: 'API-14: MetadataProvider::schema documents a typed accessor API the provider does not return'
status: Triage
assignee: []
created_date: '2026-10-04 14:13'
labels:
  - code-review-rust
  - API
dependencies: []
modified_files:
  - extensions-rust/metadata/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'API-14:extensions-rust/metadata/src/lib.rs:MetadataProvider::schema'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:285-355` (`MetadataProvider::schema`)

**What**: `schema()` lists fields such as `members`, `default_members`, `root_package`, `package_by_name` (`fn(&str) -> Option<Package>`), and `Package.*`/`Dependency.*`/`Target.*` typed as `Iterator<...>`/`Option<...>`/`enum`. The provider actually returns the raw `cargo metadata` `serde_json::Value`, whose top level has `packages`, `workspace_members`, `workspace_default_members`, `resolve`, `target_directory`, `workspace_root`, etc. (`members`, `default_members`, `root_package`, `package_by_name` do not exist, and `Package.dependencies` is a JSON array of objects with `req`, not `version_req`). The crate docs say "The shape of that value is documented by `MetadataProvider::schema`", so the documented contract is the wrong one; it is a leftover from a removed typed wrapper.

**Why it matters**: Consumers (and `ops` schema output) are told to read fields that are always absent; documentation that contradicts behaviour is worse than none (API-14).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 schema() lists only keys present in the returned JSON, using cargo metadata's real names (workspace_members, workspace_default_members, packages[].dependencies[].req, ...)
- [ ] #2 A test compares schema field names against the keys of a real or fixture cargo metadata document
<!-- AC:END -->
