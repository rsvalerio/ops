---
id: TASK-1850
title: >-
  API-9: DepsOptions is the one public type in the crate without
  #[non_exhaustive], breaking the convention lib.rs itself documents
status: To Do
assignee:
  - TASK-1997
created_date: '2026-08-27 15:25'
updated_date: '2026-08-28 14:13'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions-rust/deps/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:139-142`

**What**:

```rust
/// Options for the deps command.
pub struct DepsOptions {
    pub refresh: bool,
}
```

Every other public type this crate exports carries `#[non_exhaustive]`: `UpgradeEntry`, `UpgradeResult`, `AdvisoryEntry`, `DenyEntry`, `DenyResult`, `DepsReport` (`types.rs:10, 23, 31, 41, 85, 95`) and `DepsExtension` (`lib.rs:257`), the last with an explicit rationale comment — *"API-9 / TASK-0922: … `#[non_exhaustive]` keeps a future state field additive at the type level"*. `DepsOptions` is the sole exception, and it is the type most likely to grow: it is the options bag for the command, its single field arrived with the `--refresh` flag, and the next `ops deps` flag lands in it.

Because it is a plain public struct with a public field, its only construction form is the exhaustive literal — `crates/cli/src/subcommands.rs:56-59` builds `DepsOptions { refresh }` that way. Adding a second field breaks every such literal at once, which is exactly the churn `#[non_exhaustive]` plus a constructor (or `Default` + struct-update syntax) is there to prevent. `DepsOptions` also has no doc on its field and no `Default`, so a caller adding one flag must know every other flag's intended value.

This is a small change, but it is worth doing as part of the same pass rather than later: `#[non_exhaustive]` cannot be added once external construction sites exist without breaking them, so the cost only goes up.

**Why it matters**: the crate has an explicit, documented API-9 posture that every type it publishes follows except this one. A convention with a silent exception stops being a convention — the next reviewer has to decide case by case whether the omission was deliberate, and the next options field turns a one-line addition into a cross-crate edit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DepsOptions carries #[non_exhaustive], matching every other public type this crate exports
- [ ] #2 A constructor (or Default plus struct-update syntax) is provided so callers outside the crate can still build the value
- [ ] #3 crates/cli/src/subcommands.rs is updated to the new construction form and still compiles
- [ ] #4 The refresh field carries a doc comment stating what it does
<!-- AC:END -->
