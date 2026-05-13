---
id: TASK-1397
title: 'PERF-5: style_gated always allocates String when color disabled'
status: To Do
assignee:
  - TASK-1458
created_date: '2026-05-13 18:09'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/style.rs:58-64`

**What**: The color-disabled branch of `style_gated` returns `s.to_string()`, forcing a heap allocation for every render even though the input `&str` would suffice verbatim. Each `cyan(...)`/`grey(...)` call site pays this on every invocation.

**Why it matters**: Color-disabled CI / piped output is the common production case. Returning `Cow<'_, str>` (or taking `&str` and returning it unchanged) would skip the copy at zero risk and consolidate the gating logic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 style_gated avoids allocating when ANSI color is disabled
- [ ] #2 All callers compile and behave identically with the new return type
<!-- AC:END -->
