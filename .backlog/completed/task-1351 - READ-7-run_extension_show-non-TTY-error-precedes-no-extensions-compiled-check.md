---
id: TASK-1351
title: 'READ-7: run_extension_show non-TTY error precedes no-extensions-compiled check'
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 16:48'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:254-263`

**What**: `run_extension_show_with_tty_check` checks `is_tty()` before `compiled.is_empty()`. When a user pipes `ops extension show` (no name) in an environment with no extensions compiled in, they see "extension show requires an interactive terminal (or pass a name)" — which suggests the fix is to attach a TTY or pass a name. The real problem ("no extensions compiled in") is reachable only by re-running interactively, where they then hit the second bail with the actual diagnosis.

```rust
if !is_tty() {
    anyhow::bail!("extension show requires an interactive terminal (or pass a name)");
}

if compiled.is_empty() {
    anyhow::bail!("no extensions compiled in");
}
```

**Why it matters**: The TTY check is a precondition for the interactive picker, but the picker is *only* useful when there is something to pick. Reordering to check `compiled.is_empty()` first surfaces the irrecoverable condition (binary built without extension features) immediately, regardless of TTY state. The interactive-terminal error then accurately means "we have things to show you, but need a terminal."
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 compiled.is_empty() is checked before is_tty() so a binary with no compiled-in extensions reports the real cause regardless of TTY
- [ ] #2 existing test run_extension_show_no_tty_returns_error still passes (default features include extensions)
- [ ] #3 a new test covers non-TTY + no-extensions and asserts the no-extensions diagnosis wins
<!-- AC:END -->
