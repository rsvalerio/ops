---
id: TASK-0162
title: >-
  SEC-22: exec spawn failures leak full io::Error paths through StepFailed
  message to stderr/UI
status: Done
assignee: []
created_date: '2026-04-22 21:23'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - SEC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:367-378`

**What**: When `cmd.output().await` returns `Err(e)`, the code does `let msg = e.to_string();` and passes that string verbatim into `RunnerEvent::StepFailed::message` and `StepResult::failure`. `io::Error::to_string()` on a spawn failure includes the full resolved path of the program and the spec cwd (e.g. `No such file or directory (os error 2): /home/alice/.cargo/bin/nonexistent`), which the progress display then prints to stderr / writes to the TAP file. In a CI tail that gets shared publicly, this leaks the developer's username/home path.

**Why it matters**: SEC-22 ("no fingerprinting surface / internal-path details in user-facing responses"). Low impact for a local task runner but trivial to mitigate: strip the absolute path prefix and replace with the relative spec.program, or only log the full error at `debug!` level and surface a shorter "failed to spawn `cargo`: No such file or directory" to the UI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 StepFailed message for spawn-failure IO error does not contain absolute user paths
<!-- AC:END -->
