---
id: TASK-1031
title: 'DUP-1: ui::tests::render reimplements emit''s line-split and sanitise pipeline'
status: Done
assignee: []
created_date: '2026-05-07 20:23'
updated_date: '2026-05-07 23:11'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/ui.rs:73-85` (test) duplicates `crates/core/src/ui.rs:35-50` (production `emit`)

**What**: The `tests::render` helper rebuilds production `emit`'s exact loop body — split-on-`\n`, `sanitise_line` per segment, `prefix = if first { "" } else { "  " }`, `writeln!("ops: {level}: {prefix}{buf}")` — into a parallel `String`-writing copy so the SEC-21 assertions can inspect the bytes without touching stderr. The two implementations are byte-for-byte identical except for the writer (`stderr` vs `String`).

**Why it matters**: When `emit` evolves (a new control byte gets escaped, the continuation-prefix grows from two spaces to four, a per-line truncation cap is added under SEC-21 follow-ups, etc.) the test helper will silently keep emitting the *old* output shape. The SEC-21 regression tests would then assert that the *test renderer* still neutralises ANSI / forged `ops:` prefixes — not that production `emit` does — yielding a green run with broken sanitisation. Test-helper drift from production logic is exactly DUP-1's "duplicated grammar" footgun.

Refactor: extract the body of `emit` into `fn emit_to<W: Write>(level: &str, message: &str, w: &mut W)`. Production `emit` calls it with `&mut std::io::stderr().lock()`; the tests call it with a `&mut Vec<u8>` (or `&mut String` via `Cursor`), eliminating the second copy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a writer-generic helper from `emit` and route both production stderr emission and the test `render` helper through it
- [ ] #2 Existing SEC-21 assertions in `ui::tests` keep passing without copy-paste of the line/sanitise loop
<!-- AC:END -->
