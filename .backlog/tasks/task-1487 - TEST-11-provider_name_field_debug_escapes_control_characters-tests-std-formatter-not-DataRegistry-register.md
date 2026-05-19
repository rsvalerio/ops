---
id: TASK-1487
title: >-
  TEST-11: provider_name_field_debug_escapes_control_characters tests std
  formatter, not DataRegistry::register
status: To Do
assignee:
  - TASK-1579
created_date: '2026-05-18 06:27'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/tests.rs:559-565`

**What**: The test claims to pin SEC-21 / TASK-1226 (escape control characters in the `provider_name` field emitted by `DataRegistry::register`'s duplicate-insert breadcrumb). However, the test body never invokes `DataRegistry::register` or any production codepath — it just runs `format!("{name:?}", name = ...)` and asserts std's Debug formatter escapes `\n` and ESC.

```rust
fn provider_name_field_debug_escapes_control_characters() {
    let name = "stub\nFAKE_LOG\n\u{1b}[31m";
    let rendered = format!("{name:?}");
    assert!(!rendered.contains('\n'));
    assert!(!rendered.contains('\u{1b}'));
    assert!(rendered.contains("\\n"));
}
```

The test will continue to pass even if the production breadcrumb in `data.rs:191` is changed from `provider_name = ?name` (Debug) to `provider_name = %name` (Display), reintroducing the log-forgery vector SEC-21 was meant to close.

**Why it matters**: TEST-11 — the test asserts a tautology of std's Debug impl rather than the behaviour it claims to pin. It provides no regression protection for SEC-21.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Rewrite the test to drive DataRegistry::register with a forged provider name and use a tracing_subscriber test layer (or tracing-test) to capture the emitted event and assert the rendered field does not contain raw newline/ESC bytes
- [ ] #2 Alternatively, expose a small helper that formats the breadcrumb provider_name field and call it directly, so the test exercises the actual formatting choice rather than std Debug impl
- [ ] #3 The reworked test fails if the format specifier in data.rs:191 is flipped from ?name (Debug) to %name (Display)
<!-- AC:END -->
