---
id: TASK-2561
title: 'API-13: hook entry points reachable by two public paths (pub mod plus root pub use)'
status: Done
assignee: []
created_date: '2026-10-10 15:40'
updated_date: '2026-10-10 22:10'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/hook-common/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'API-13:extensions/hook-common/src/lib.rs:crate-root pub use re-exports'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:22-36`

**What**: The crate root declares `pub mod config; pub mod git; pub mod install;` and *also* re-exports their entry points: `pub use config::ensure_config_command;`, `pub use git::find_git_dir;`, `pub use install::install_hook;`. Each of the three items is therefore reachable by two public paths (`ops_hook_common::install_hook` and `ops_hook_common::install::install_hook`), doubling every mention in docs, search results, and error messages. `pub mod git_state` has no re-export and `paths` is correctly `pub(crate)` — the correct single-path shape is already demonstrated one module down.

**Why it matters**: API-13 — a public item should be reachable by exactly one path; a `pub use` that merely duplicates a still-public module path is the characteristic refactoring artifact. Same shape as TASK-2072 (extensions/about, now fixed): the fix there demoted the modules and kept the root re-exports.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each of ensure_config_command, find_git_dir, install_hook is reachable by exactly one public path (modules demoted to pub(crate) with the root re-exports kept, or the re-exports removed)
- [x] #2 Genuine root re-exports carry #[doc(inline)] so they render with their siblings
- [x] #3 Workspace crates referencing the module-qualified paths are updated; cargo check/test -p ops-hook-common and dependent hook crates pass

<!-- AC:END -->
