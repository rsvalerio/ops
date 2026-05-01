---
id: TASK-0139
title: 'READ-1: writeln!(String, ..).unwrap() obscures infallibility in help.rs'
status: Done
assignee: []
created_date: '2026-04-22 21:16'
updated_date: '2026-04-23 15:09'
labels:
  - rust-code-review
  - read
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/help.rs:130, 140`

**What**: `writeln!` to a `String` uses `.unwrap()`. Writing to a String (via fmt::Write) is infallible; the unwrap suggests potential failure where there is none.

**Why it matters**: Misleads readers into thinking this can fail; clutters the code. Using `write!`/`writeln!` on `String` should be paired with either a short `let _ =` comment or prefer the `.push_str(&format!(..))` equivalent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Remove .unwrap() from writeln! on String (the fmt::Write impl on String is infallible)
- [ ] #2 Or add a brief comment noting the infallibility if unwrap is kept
<!-- AC:END -->
