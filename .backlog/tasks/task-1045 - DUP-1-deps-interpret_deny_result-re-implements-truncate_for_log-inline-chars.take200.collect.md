---
id: TASK-1045
title: >-
  DUP-1: deps interpret_deny_result re-implements truncate_for_log inline
  (chars().take(200).collect())
status: Done
assignee: []
created_date: '2026-05-07 20:53'
updated_date: '2026-05-07 23:18'
labels:
  - code-review-rust
  - dup
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:418, 442` (`interpret_deny_result`)

**What**: The same file already defines `truncate_for_log` (parse.rs:296):

```rust
fn truncate_for_log(s: &str) -> String {
    const MAX: usize = 200;
    if s.len() <= MAX { s.to_string() } else {
        let mut end = MAX;
        while !s.is_char_boundary(end) { end -= 1; }
        format!("{}…", &s[..end])
    }
}
```

It is used in `interpret_upgrade_output` (`parse.rs:66`) and `decode_diagnostic` (`parse.rs:504, 519, 539, 581, 604`). However the two `Some(1)` and `Some(other)` arms of `interpret_deny_result` open-code the truncation:

```rust
stderr.chars().take(200).collect::<String>()
```

Two divergences from the helper:

1. The inline form takes 200 *chars* (variable byte count), whereas `truncate_for_log` takes 200 *bytes* and clamps to a char boundary. For all-ASCII stderr they happen to match; for localised cargo-deny diagnostics the inline form leaks a longer log line than the helper.
2. The inline form drops the `…` suffix, so an operator cannot tell the difference between "stderr was 199 chars total" and "stderr was 50 KB and we cut it".

Both inline call sites should route through `truncate_for_log` for parity with `interpret_upgrade_output` (which sits one function away in the same file and already uses it).

**Why it matters**: DUP-1 — three slightly-different truncators in one module. A future tweak to the truncation policy (cap at 500 chars, change the ellipsis, sanitise control chars) would have to find and patch every copy; a missed copy silently regresses one of the call sites.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both Some(1) and Some(other) arms of interpret_deny_result render stderr through truncate_for_log instead of inlining chars().take(200).collect()
- [ ] #2 No 'chars().take(200).collect()' calls remain in deps/src/parse.rs outside truncate_for_log itself
<!-- AC:END -->
