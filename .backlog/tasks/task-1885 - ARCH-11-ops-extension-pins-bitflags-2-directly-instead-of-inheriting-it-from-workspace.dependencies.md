---
id: TASK-1885
title: >-
  ARCH-11: ops-extension pins bitflags = "2" directly instead of inheriting it
  from [workspace.dependencies]
status: Done
assignee:
  - TASK-1985
created_date: '2026-08-27 15:33'
updated_date: '2026-08-28 19:26'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - crates/extension/Cargo.toml
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/Cargo.toml:18`

**What**: every dependency in this manifest inherits from the workspace except one:

```toml
[dependencies]
ops-core = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
indexmap = { workspace = true }
bitflags = "2"          # <-- the exception
linkme = { workspace = true }
tracing = { workspace = true }
```

`bitflags` has no entry in the root `[workspace.dependencies]` table, so the version floor lives only here. `crates/extension` is currently the only member declaring it (verified across all `Cargo.toml` files in the workspace), which is precisely the moment to centralize it — before a second crate adds its own line with a different constraint.

**Why it matters**: ARCH-11 asks workspaces to centralize shared dependency versions so version drift cannot happen and a CVE upgrade is a single-point change. The workspace has clearly adopted that policy — the root manifest carries ~40 centralized entries and a comment block declaring `[workspace.lints]` mandatory for every member — so this line is an unmarked deviation from an otherwise uniform convention. `bitflags` is not incidental here either: it generates `ExtensionType`, a type in this crate's public API that every extension crate matches on, so its version is part of the framework's compatibility surface.

**Suggested fix**: add `bitflags = "2"` to `[workspace.dependencies]` in the root `Cargo.toml` and change this line to `bitflags = { workspace = true }`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 bitflags is declared once in the root [workspace.dependencies] table
- [ ] #2 crates/extension/Cargo.toml inherits it with bitflags = { workspace = true } and the workspace builds unchanged
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1: bitflags = "2" added to the root [workspace.dependencies] table, next to indexmap.
AC#2: crates/extension/Cargo.toml now reads bitflags = { workspace = true }; the workspace builds unchanged (ops verify green, 2539 tests pass).
<!-- SECTION:NOTES:END -->
