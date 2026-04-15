---
id: TASK-0052
title: 'CD-10: LicenseEntry/BanEntry/SourceEntry are structurally identical structs'
status: Done
assignee: []
created_date: '2026-04-14 20:31'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-duplication
  - DUP-2
  - DUP-6
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `extensions-rust/deps/src/lib.rs:55-75`
**Anchor**: `struct LicenseEntry`, `struct BanEntry`, `struct SourceEntry`
**Impact**: Three structs with identical fields (`package: String`, `message: String`, `severity: String`) and identical derives (`Debug, Clone, PartialEq, Eq, Serialize, Deserialize`). The existing `format_deny_section` already abstracts over them with a closure extractor, confirming they share the same shape. A single `DenyEntry { kind: DenyKind, package, message, severity }` or a type alias would reduce the three definitions to one.

DUP-2: 3 structs with identical structure differing only in name. DUP-6: use a shared type.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single DenyEntry type (or equivalent) replaces the three identical struct definitions
<!-- AC:END -->
