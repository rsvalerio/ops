---
id: TASK-2080
title: 'API-1: looks_like_secret_value re-exported under the meaningless alias looks_like_secret_value_public'
status: To Do
assignee: []
created_date: '2026-09-07 22:57'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - crates/runner/src/command/mod.rs
  - crates/cli/src/run_cmd/dry_run.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:50`

**What**: `pub use secret_patterns::looks_like_secret_value as looks_like_secret_value_public;` renames the function on re-export, so the public path is `ops_runner::command::looks_like_secret_value_public`. No symbol collision exists to justify the rename — `command::mod` has no other `looks_like_secret_value` (the in-crate tests import the original name straight from the private `secret_patterns` module), and the sibling re-export on the previous line keeps its own name (`is_sensitive_env_key`).

**Why it matters**: API-1 — the `_public` suffix names an implementation detail of the re-export site, not anything about the function's behaviour; a reader at the sole external call site (`crates/cli/src/run_cmd/dry_run.rs:8,108`) sees `looks_like_secret_value_public(&expanded)` and cannot guess it is the same detector defined in `secret_patterns` and warned with in `build_command_with`. It also reads as if a private twin exists, which it does not. One public path, one name.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Re-export is pub use secret_patterns::looks_like_secret_value; (no alias) and the one CLI import in dry_run.rs is updated
- [ ] #2 No other in-crate use imports the _public alias (in-crate tests already import from the private module path)
- [ ] #3 ops verify / ops qa gates pass

<!-- AC:END -->
