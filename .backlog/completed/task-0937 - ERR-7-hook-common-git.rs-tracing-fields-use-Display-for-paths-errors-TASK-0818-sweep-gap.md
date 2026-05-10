---
id: TASK-0937
title: >-
  ERR-7: hook-common git.rs tracing fields use Display for paths/errors
  (TASK-0818 sweep gap)
status: Done
assignee: []
created_date: '2026-05-02 15:51'
updated_date: '2026-05-02 17:26'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:73-78,107-112,162-167,170-175`

**What**: Four `tracing::debug!` call sites in `read_gitdir_pointer` / `find_git_dir` log path values via `%path.display()` and `%anchor.display()` / `%target.display()`. Display-formatted paths do not escape control characters / ANSI escapes, so an attacker-controlled path component (a `gitdir:` pointer pointing to `name\nADMIN: forged`) can forge log lines or smuggle ANSI through a structured-log shipper.

**Why it matters**: TASK-0665 / TASK-0818 swept `Display` → `Debug` formatting for path log fields across about/manifest_io and node/python parsers. The hook-common `git.rs` siblings were missed in that sweep. The codebase already has the established pattern (`?path.display()` or `path = ?path.display()`); applying it here keeps the policy uniform and closes the last log-injection gap in the git-dir resolver.

<!-- scan confidence: candidates to inspect -->
- `extensions/hook-common/src/git.rs:75` — `path = %file.display()` in `read_gitdir_pointer` debug log
- `extensions/hook-common/src/git.rs:107-110` — `anchor`/`target` fields use `%display()` in containment-rejection debug log
- `extensions/hook-common/src/git.rs:162` — `parent = %parent.display()` in `sync_parent_dir`-style debug log (hook-common/src/install.rs sibling already uses this pattern; leave unless ERR-7 requires changes there too)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All %path.display() / %dir.display() tracing field bindings in extensions/hook-common/src/git.rs are converted to ?path.display()
- [x] #2 Regression test (similar to manifest_io::path_display_debug_escapes_control_characters) pins the Debug-format escaping for at least one of the changed sites
<!-- AC:END -->
