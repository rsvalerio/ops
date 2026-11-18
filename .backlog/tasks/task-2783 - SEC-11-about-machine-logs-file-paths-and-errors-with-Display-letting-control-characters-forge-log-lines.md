---
id: TASK-2783
title: 'SEC-11: about machine logs file paths and errors with Display, letting control characters forge log lines'
status: Done
assignee: []
created_date: '2026-11-18 18:44'
updated_date: '2026-11-18 21:18'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2857'
modified_files:
  - extensions/about/src/machine.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-11:extensions/about/src/machine.rs:read_layer'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/machine.rs:195` (`push_with_includes`), `:257` (`include_paths`), `:318`, `:325` (`read_layer`), `:671` (`resolve_rustflags`, `error = %e` where the message embeds config paths), `:874`, `:878` (`probe`)

**What**: These `tracing` calls record paths and errors with `%` (Display): `path = %path.display()`, `path = %layer.path.display()`, `error = %e`. The rest of the crate — and its own `push_with_includes` at line 214, which comments "Debug, not Display: a path is filesystem-controlled data" — records such fields with `?` so newlines and ANSI escapes are escaped. `test_support::assert_debug_escapes_control_chars` exists to pin exactly that contract, and machine.rs has no such test.

**Why it matters**: Cargo config include paths and the files themselves are repo-controlled; a path or TOML parse error carrying `\n` or ESC is written raw into the log stream, so a checkout can forge or hide log records (log injection), inconsistent with the policy every other about module follows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All path and error fields in machine.rs tracing calls use Debug (?) formatting
- [x] #2 A machine.rs test calls test_support::assert_debug_escapes_control_chars for a representative path field

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).

Fixed in wave TASK-2857: every path/error tracing field in machine/ now uses ? (errors as ?e.to_string(), so toml errors do not dump the whole file); probe status stays % (not path/error data). Test layers::tests::config_path_debug_escapes_control_characters.

<!-- SECTION:NOTES:END -->
