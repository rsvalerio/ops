---
id: TASK-0776
title: >-
  PERF-3: finish_bar allocates an owned String inside finish_with_message even
  though caller already owns the rendered line
status: Triage
assignee: []
created_date: '2026-05-01 05:56'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:174-177`

**What**: finish_bar takes line: &str, then calls bar.finish_with_message(line.to_string()). Every call site already owns or has just constructed the line as a String — taking &str then re-allocating loses the move opportunity.

**Why it matters**: Per-step finalize on every plan, plus orphan finalization on cancellation. Switching to impl Into<Cow<'static, str>> (indicatif's actual signature) lets the caller hand over an owned String without a copy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change finish_bar to accept String (or impl Into<Cow<'static, str>>) and update call sites to pass owned strings already constructed
- [ ] #2 Keep the write_non_tty(line) mirror call by borrowing back from the owned value
- [ ] #3 Pin behaviour with the existing display tests
<!-- AC:END -->
