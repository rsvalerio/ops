---
id: TASK-0965
title: 'ERR-7: config loader logs paths via Display format (log-injection sweep gap)'
status: Triage
assignee: []
created_date: '2026-05-04 21:47'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:143,240,299`

**What**: Three `tracing::debug!` events log `path = %path.display()`: `.ops.toml` (143), `.ops.d/*.toml` (240), and global config (299). The TASK-0818 / TASK-0930 / TASK-0945 sweep moved path/error tracing fields to `?path.display()` (Debug) so embedded newlines and ANSI escapes cannot forge log records — these sites were missed.

**Why it matters**: `read_conf_d_files` enumerates `.ops.d/*.toml` whose filenames a repo collaborator (or `cargo ops` running on `/tmp/$attacker`) controls. A filename with `\n[fake] info: ...` lands on operator log streams as a forged record. Aligns with the codebase's recent ERR-7 sweep policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both merge_conf_d's 'merging conf.d config' and load_global_config's 'merging global config' debug events use ?path.display() (Debug)
- [ ] #2 read_config_file open-failure context message keeps user-facing format! form, surrounding tracing fields use Debug
- [ ] #3 Regression test: a .ops.d filename with embedded newline produces a single log record with characters escaped
<!-- AC:END -->
