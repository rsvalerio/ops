---
id: TASK-0543
title: >-
  READ-5: print_exec_spec lossily renders cwd via Path::display().to_string()
  before expansion
status: Triage
assignee: []
created_date: '2026-04-29 04:58'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/dry_run.rs:95`

**What**: `vars.expand(&cwd.display().to_string())` lossily converts a non-UTF-8 PathBuf before expansion. The dry-run preview a user inspects diverges silently from the actual spawn (which would either fail strict-expand or accept the bytes through to_string_lossy).

**Why it matters**: API-1 in run_external_command already rejects non-UTF-8 argv with a clear error; cwd should follow the same policy or annotate the lossy conversion in dry-run output so the preview is trustworthy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 dry-run either errors on non-UTF-8 cwd with the same message style as the argv check, or appends an explicit annotation
- [ ] #2 Unix-only test exercises a PathBuf with a non-UTF-8 byte sequence end-to-end through dry-run
<!-- AC:END -->
