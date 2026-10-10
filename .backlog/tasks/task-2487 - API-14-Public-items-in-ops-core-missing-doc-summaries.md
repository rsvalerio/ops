---
id: TASK-2487
title: 'API-14: Public items in ops-core missing doc summaries'
status: Done
assignee: []
created_date: '2026-10-10 15:30'
updated_date: '2026-10-10 21:25'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2610'
modified_files:
  - crates/core/src/config/command_id.rs
  - crates/core/src/config/commands.rs
  - crates/core/src/project_identity.rs
  - crates/core/src/project_identity/card.rs
priority: medium
ordinal: 1000
dedup_key: 'API-14:crates/core/src/config/command_id.rs:CommandId'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/command_id.rs:15,20` (`CommandId::new`, `CommandId::as_str`), `crates/core/src/config/commands.rs:508` (`CommandSpec::timeout`), `crates/core/src/project_identity.rs:204,241,263,289,305,324` (`UnitTarget::new`, `CoverageStats::new`, `UnitCoverage::new`, `ProjectCoverage::new`, `UnitDeps::new`, `ProjectDependencies::new`), `crates/core/src/project_identity/card.rs:161,166,242,248,254` (`AboutCard::from_identity`, `AboutCard::from_identity_filtered`, `AboutCardBuilder::description`, `AboutCardBuilder::fields`, `AboutCardBuilder::build`)

**What**: Public items without the mandatory summary sentence. `CommandId::new` and `as_str` sit in a file whose every other item is documented; `CommandSpec::timeout` is the one undocumented accessor in an impl block whose siblings (`display_cmd`, `validate_env`) carry full doc blocks; the six `new` constructors in project_identity.rs and the `from_identity*` / builder-setter group on AboutCard have no docs at all. The workspace lints do not include `missing_docs`, so nothing mechanical catches these.

**Why it matters**: API-14: the summary sentence is mandatory on every public item — it is what rustdoc lifts into the module index, so an undocumented constructor or accessor is invisible in generated docs. One line each suffices (e.g. `timeout`: "Configured timeout as a `Duration`, or `None` when `timeout_secs` is unset.").

<!-- scan confidence: verified by direct read -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every public item listed above carries at least a one-sentence doc summary
- [x] #2 cargo doc renders summaries for these items in the module index

<!-- AC:END -->
