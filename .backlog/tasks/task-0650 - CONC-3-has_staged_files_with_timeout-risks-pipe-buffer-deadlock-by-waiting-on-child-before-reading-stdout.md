---
id: TASK-0650
title: >-
  CONC-3: has_staged_files_with_timeout risks pipe-buffer deadlock by waiting on
  child before reading stdout
status: Done
assignee:
  - TASK-0735
created_date: '2026-04-30 04:54'
updated_date: '2026-04-30 06:12'
labels:
  - code-review-rust
  - conc
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions/run-before-commit/src/lib.rs:108-167

**What**: has_staged_files_with_timeout spawns 'git diff --cached --name-only --diff-filter=ACMR' with Stdio::piped() for stdout and stderr (lines 111-113), then enters a try_wait polling loop (lines 127-148) BEFORE any reader drains the pipes. Only after the child exits does wait_with_output (line 151) read the stdout/stderr buffers.

If git diff --cached --name-only produces more output than the OS pipe buffer (typically 64 KiB on Linux, 16 KiB on macOS), git will block on write waiting for a reader that does not exist until after the timeout fires. In a large monorepo with thousands of staged files, the path list trivially exceeds 64 KiB. The visible symptom is a commit that always hits the bounded-wait timeout and bails with HasStagedFilesError::Timeout even when git is healthy.

**Why it matters**: This is the primary pre-commit gate. A false-positive Timeout makes the hook unusable for large repos and surfaces as 'pre-commit hook bailed; commits blocked' to the developer. The bounded-wait fix from TASK-0589 actually exposed this latent deadlock by replacing the previous unbounded wait_with_output (which was slow but correct) with a try_wait poll that does not drain the pipe.

**Repro**: stage many thousands of files then run ops run-before-commit. Or simulate via a fake git script that emits ~2 MB of stdout and trivially fills a 64 KiB pipe.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Drain stdout/stderr concurrently with the timeout wait — e.g. spawn reader threads on child.stdout.take() and child.stderr.take() and join them in the timeout/exit path, or move to async I/O via tokio::process
- [x] #2 Add a regression test that pipes >128 KiB of synthetic stdout from a fake git binary and confirms the call returns true within a small multiple of the configured timeout
- [x] #3 Consider whether --quiet --exit-code (which suppresses output and signals via exit status) would let us avoid reading stdout entirely; if so, switch to that and document the trade-off
<!-- AC:END -->
