---
id: TASK-1821
title: >-
  DUP-3: cargo-deny severity classification is duplicated across the gate and
  the renderer, so the two can silently disagree
status: To Do
assignee:
  - TASK-1997
created_date: '2026-08-27 11:33'
updated_date: '2026-08-28 14:13'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-rust/deps/src/lib.rs
  - extensions-rust/deps/src/format.rs
  - extensions-rust/deps/src/parse/deny.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:211-226` (`has_issues::is_actionable`) and `extensions-rust/deps/src/format.rs:42-92` (`SeverityClass`)

**What**: the crate carries the same piece of domain knowledge — "which cargo-deny severity strings exist, and which are benign" — in two independent places:

```rust
// lib.rs (the gate that decides the process exit code)
match severity {
    "error" => true,
    "warning" => !relax_warning,
    "note" | "help" | "info" => false,
    other => { tracing::warn!(...); true }
}

// format.rs (the renderer that decides icon, colour and ReportStatus)
match severity {
    "error" => Self::Error,
    "warning" => Self::Warning,
    "note" | "help" | "info" => Self::Info,
    _ => Self::Unknown,
}
```

A third site, `parse/deny.rs:215` (`MISSING_SEVERITY_SENTINEL`), produces a value both classifiers must route to their fail-closed arm — the sentinel's own doc comment has to explain how `has_issues` handles it, which is exactly the coupling that this duplication creates.

The two lists agree today, and both TASK-0601 and TASK-0602 had to be filed and fixed *separately* for the same underlying change. The next cargo-deny severity addition requires editing both again, and forgetting one produces the worst outcome available: a report rendered with a red `?` and an `Error` row status while `has_issues` returns `false` and `ops deps` exits 0 (or the reverse — a green-looking report on a non-zero exit).

`SeverityClass` is already the richer, better-tested abstraction (`format/row_tests.rs` pins its icon/status contract). The gate should classify through it rather than re-deriving the same partition from raw strings.

**Why it matters**: the severity partition is the single safety-relevant fact in this crate — it decides whether a supply-chain finding fails CI. Encoding it twice means a drift between the two copies is *undetectable* from either side: each module's own tests still pass, and the disagreement only shows up as a report whose visible status contradicts the exit code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The set of known cargo-deny severity strings and their benign/actionable partition is defined exactly once in the crate
- [ ] #2 has_issues classifies through that single definition instead of its own match on &str
- [ ] #3 The bans-only 'warning is informational' relaxation is preserved and still expressed at the call site, not baked into the shared classifier
- [ ] #4 Unknown severities still fail closed in the gate and still render as SeverityClass::Unknown / ReportStatus::Error
- [ ] #5 A test asserts the gate and the renderer agree for every known severity plus MISSING_SEVERITY_SENTINEL and an unknown value, so a future one-sided edit fails
- [ ] #6 Existing has_issues and SeverityClass tests still pass unchanged
<!-- AC:END -->
