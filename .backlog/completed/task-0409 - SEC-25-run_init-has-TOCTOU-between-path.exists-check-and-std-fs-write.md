---
id: TASK-0409
title: 'SEC-25: run_init has TOCTOU between path.exists() check and std::fs::write'
status: Done
assignee:
  - TASK-0416
created_date: '2026-04-26 09:53'
updated_date: '2026-04-27 08:31'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/init_cmd.rs:18-28`

**What**: `run_init_to` checks `path.exists()` (line 19) and, if not present (or `--force` was passed), calls `std::fs::write(&path, content)` later (line 28). Between those two syscalls another process — or a malicious symlink planted in the cwd — can create the file. The write then clobbers content the user did not intend to overwrite. The `--force` branch has the same race in reverse: a check-then-write window where the file is replaced by a symlink to a sensitive target between the check and the write.

**Why it matters**: `ops init` is run interactively in the user shell, usually inside a project they own, so the threat model is narrow. But the codebase has already eliminated the same TOCTOU pattern elsewhere (TASK-0222 ensure_config_command, TASK-0309 hook install, TASK-0361 hook write) by switching to `OpenOptions::new().write(true).create_new(true).open()`. Applying the same idiom here keeps `init_cmd` consistent with the rest of the file-creation paths in this workspace and closes a small foot-gun on shared CI runners or container images where two `ops init` processes can race.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Without --force, the existence check and write are merged into a single OpenOptions::create_new(true).open() that fails atomically when the file already exists
- [x] #2 With --force, switch to atomic write-then-rename (or document that the user explicitly asked to clobber)
- [x] #3 Behaviour matches the TOCTOU resolution applied in TASK-0309 / TASK-0361 for hook install
<!-- AC:END -->
