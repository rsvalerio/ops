---
id: TASK-2020
title: >-
  SEC-21: .ops.toml-derived command ids and aliases still reach tracing fields
  via Display in ops-runner
status: To Do
assignee:
  - TASK-2043
created_date: '2026-08-28 19:28'
updated_date: '2026-08-29 11:35'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/runner/src/command/resolve.rs
  - crates/runner/src/command/mod.rs
  - crates/runner/src/command/exec.rs
  - crates/runner/src/command/parallel.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/resolve.rs:48,356,466-468`, `crates/runner/src/command/mod.rs:339-341,350-352,441,454`, `crates/runner/src/command/exec.rs:783,899`, `crates/runner/src/command/parallel.rs:236,247`

**What**: the SEC-21 sweep required by TASK-1937 AC #3 found that the env key was not the last `.ops.toml`-derived string rendered with the Display (`%`) formatter. Command ids and aliases come from `.ops.toml` table keys and `aliases` arrays — arbitrary text with no character restrictions — and still reach `tracing` fields as `id = %id`, `alias = %alias`, `existing = %existing`, `new = %name`, `command = %id`, `step_id = %id`, and `#[instrument(fields(id = %id))]`. Under Display an embedded newline plus a crafted prefix forges what reads as an additional log record, and `\u{1b}[` repaints the operator's terminal.

TASK-1937 closed the two `warn_if_sensitive_env` sites and the three `apply_escape_policy` path fields in `build.rs`; these remaining sites are in files outside that wave's scope (`resolve.rs`, `mod.rs`) or are non-security log lines whose field shape several tests may read.

Note `CommandId` derives `Debug`, so a bare `?id` renders `CommandId("x")` and changes the field shape — the mechanical fix is `?id.as_str()` (or `?&*id`), which renders `"x"`.

**Why it matters**: it is the same already-adopted hardening policy (SEC-21 / TASK-1127 for `program`, ERR-7 / TASK-0940 for tap paths, TASK-1937 for env keys and cwd paths) left half-applied. A partially-applied policy reads to the next reviewer as a deliberate exception rather than an oversight, and the ids are the single most frequently logged config-derived string in the crate.

**Origin**: discovered during TASK-1986 while fixing TASK-1937 (its AC #3 sweep).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 every .ops.toml-derived string (command id, alias, display label) reaching a tracing field in ops-runner is rendered with the Debug formatter
- [ ] #2 CommandId fields render as the bare quoted id (?id.as_str()), not as CommandId("..."), so existing log-shape expectations hold
- [ ] #3 a unit test pins the escape for an id containing a newline and an ANSI escape, mirroring program_field_debug_escapes_control_characters
<!-- AC:END -->
