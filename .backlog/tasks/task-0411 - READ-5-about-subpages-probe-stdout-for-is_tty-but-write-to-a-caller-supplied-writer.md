---
id: TASK-0411
title: >-
  READ-5: about subpages probe stdout for is_tty but write to a caller-supplied
  writer
status: To Do
assignee:
  - TASK-0418
created_date: '2026-04-26 09:53'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/units.rs:50`, `extensions/about/src/coverage.rs:95`, `extensions/about/src/deps.rs:50`, `extensions/about/src/cards.rs:65` (via render_card), `extensions/about/src/lib.rs:89`

**What**: Every `run_about_*_with(..., writer)` entry point computes `let is_tty = std::io::stdout().is_terminal();` and then forwards `is_tty` into card / coverage / deps rendering that writes into `writer`, not stdout. The decision to emit ANSI escapes is therefore based on the wrong file descriptor: a caller that hands in a `Vec<u8>` while stdout happens to be a real terminal will receive ANSI-styled content; conversely a caller writing to a TTY of its own while stdout is redirected will get unstyled text.

**Why it matters**: (1) Test buffers pick up ANSI escape sequences when the test binary is run interactively, leaking into substring assertions and producing flaky-on-TTY behavior. (2) A future caller routing about output to a non-stdout TTY (pager, log writer) loses styling without an opt-in. The fix is to derive `is_tty` from the writer (e.g., accept a `&dyn Write + IsTerminal`-style probe, take an explicit `is_tty` argument, or always pass the writer through a styling helper that decides per-fd). Today the same bug is repeated in five subpages.

<!-- scan confidence: high; verified by reading every run_about_*_with site -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 is_tty (or styling decision) is derived from the writer, not stdout, in every run_about_*_with entry point
- [ ] #2 test exercises a Vec<u8> writer and asserts no ANSI escapes are emitted regardless of stdout state
<!-- AC:END -->
