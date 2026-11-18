---
id: TASK-2781
title: 'SEC-11: about machine text output writes repo-controlled config values and process names to the terminal unsanitized'
status: To Do
assignee: []
created_date: '2026-11-18 18:44'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2857'
modified_files:
  - extensions/about/src/machine.rs
priority: medium
ordinal: 1000
dedup_key: 'SEC-11:extensions/about/src/machine.rs:format_machine_report'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/machine.rs:1012-1088` (`format_machine_report`, via `setting_line` and `fs_line`)

**What**: The text form of `ops about machine` interpolates values straight into terminal lines: `Setting::value`/`Setting::source` (from `.cargo/config.toml` keys such as `target.<triple>.linker`, `build.rustflags`, `build.target-dir`, `build.rustc-wrapper`, plus config-file paths), `BuildProcess::name` (from `ps -axo comm=`, settable by any local process), `FsReport::path` and `fs_type`. None of these pass through the crate's display-safety predicate (`text_util::contains_control_chars` / `ops_core::text::contains_unsafe_display_chars`) or an escaping cell constructor (`OpsTable::text_cell`), which every other about page uses for attacker-controllable text.

**Why it matters**: `ops about` is documented (text_util.rs `has_allowed_url_scheme`) as routinely run against third-party checkouts. A checked-in `.cargo/config.toml` with ESC/OSC sequences or bidi overrides in `linker` or `rustflags` can rewrite the terminal, forge lines (e.g. hide a `WARNING: tmpfs`), or plant OSC 8 hyperlinks when a user inspects the machine report.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every repo- or process-derived string rendered by format_machine_report is checked with the shared unsafe-display predicate (dropped/replaced) or escaped before printing
- [ ] #2 A test renders a report whose linker/rustflags/process name carry ESC, CR/LF and U+202E and asserts none survive in the text output
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
