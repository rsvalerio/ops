---
id: TASK-2118
title: 'TEST-32: collect_drops_remote_url_with_control_bytes asserts on a defaulted empty string, so two of its three assertions cannot fail'
status: To Do
assignee: []
created_date: '2026-09-08 06:54'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2241'
modified_files:
  - extensions/git/src/provider.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/provider.rs:227-249`

**What**: The test does

```rust
let url = info.remote_url.clone().unwrap_or_default();
assert!(!url.contains('\n'), ...);
assert!(!url.contains('\u{1b}'), ...);
assert!(info.remote_url.is_none(), ...);
```

The final assertion establishes that `remote_url` is `None`, which means `url` is always `""` and the two preceding assertions are tautologies — they would still pass if the leak-detection logic were deleted. If the code ever regressed to *returning* a poisoned URL, the third assertion is the only one doing work, and the first two would fire only incidentally.

**Why it matters**: The test names the SEC-2 / TASK-1102 control-byte guarantee, so it is the artifact a future reader trusts when changing `RedactedUrl::redact` or the `GitInfo::collect` fallback. Assertions that cannot fail overstate the coverage and hide that the "no raw newline / no ANSI escape in the emitted value" property is not actually exercised anywhere on this path.

<!-- Reviewer note: either drop the two vacuous assertions and keep the is_none() check, or (better) split into two tests — one pinning the drop, one feeding a control-byte value through a path where a value *is* emitted, so the no-leak property is asserted against a non-empty string. -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No assertion in the test is satisfied trivially by the unwrap_or_default fallback
- [ ] #2 The control-byte no-leak property is asserted against a value that is actually emitted, or the test is scoped explicitly to the drop behaviour
<!-- AC:END -->
