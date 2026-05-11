---
id: TASK-1293
title: >-
  READ-5: main() collapses every Err into ExitCode::FAILURE, losing specific
  exit codes
status: To Do
assignee:
  - TASK-1306
created_date: '2026-05-11 16:10'
updated_date: '2026-05-11 16:49'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:62-70`

**What**: `main()` matches `run()` and maps every `Err(e)` to `ExitCode::FAILURE` (1). Other CLI paths deliberately return `Ok(ExitCode::from(130))` for Ctrl-C cancellation (subcommands.rs:126). An error path that wants to signal SIGINT (130), SIGPIPE (141), or any other specific exit code cannot bubble that through `anyhow::Error`.

**Why it matters**: Today only the Ok(130) path surfaces the cancel exit code. Any future error-returning path (e.g., a downstream that returns `Err(...)` after detecting Ctrl-C) silently becomes exit 1, breaking shell scripts and CI gates that distinguish 130 from generic failure.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Define an error sentinel (anyhow context or typed error) for 'use exit code N'
- [ ] #2 main() extracts the requested code from the error before defaulting to FAILURE
- [ ] #3 Ctrl-C / pipe-break paths can exit with 130 / 141 from both Ok and Err arms
<!-- AC:END -->
