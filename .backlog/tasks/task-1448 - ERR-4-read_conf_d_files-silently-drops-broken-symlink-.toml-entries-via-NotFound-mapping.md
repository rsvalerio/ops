---
id: TASK-1448
title: >-
  ERR-4: read_conf_d_files silently drops broken-symlink .toml entries via
  NotFound mapping
status: To Do
assignee:
  - TASK-1453
created_date: '2026-05-13 18:45'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:248-264`

**What**: `DirEntry::path()` returns the symlink target; if the target is missing, `read_config_file` opens it and surfaces `NotFound`, which `read_capped_toml_file` maps to `Ok(None)` and silently drops. That contradicts the "loud failure" contract documented on `merge_conf_d` (lines 268-272). A broken-symlink overlay file disappears with no diagnostic.

**Why it matters**: Sibling of TASK-1400 (`read_conf_d_files silently skips unreadable`) but distinct code path — that task covers `DirEntry` errors; this one covers `Path::is_symlink() && !exists()`. Same "silent drop" symptom, different mechanism.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A broken symlink in .ops.d/ emits a warn (or hard-fails, matching the documented contract) instead of being silently ignored
- [ ] #2 Regression test creates a broken symlink in a temp .ops.d and asserts the diagnostic
- [ ] #3 Doc on merge_conf_d is updated if behaviour intentionally diverges from 'loud failure'
<!-- AC:END -->
