---
id: TASK-0733
title: >-
  PERF-1: format_error_tail decodes the entire stderr buffer to scan for the
  last N lines
status: To Do
assignee:
  - TASK-0741
created_date: '2026-04-30 05:50'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/output.rs:24-37` (`format_error_tail`)

**What**: `format_error_tail(stderr: &[u8], n)` first calls `String::from_utf8_lossy(stderr).into_owned()` (allocating the *whole* stderr decoded as UTF-8), then iterates `.lines()` and keeps only the last `n` in a bounded ring. So a 4 MiB stderr (the configured `OPS_OUTPUT_BYTE_CAP`) is fully copied just to surface 5 lines.

`String::from_utf8_lossy` on a `Cow::Owned` 4 MiB allocation is the costly step here; the subsequent `lines()` iteration is fine. The fix is to scan from the end: split on `\n` byte-wise, keep the last `n` segments, then lossy-decode only those. PERF-1 explicitly calls out "premature .collect()" and "unnecessary intermediate collections"; this is exactly that pattern in bytes form.

Final `ring.into_iter().collect::<Vec<_>>().join("\n")` is also an unnecessary collect — `Itertools::join` or a manual loop avoids the intermediate Vec.

**Why it matters**: This is invoked by `ErrorDetailRenderer::extract_stderr_tail` on every failed step. Under `cargo test` failures with multi-MB stderr output, the spike copies the entire buffer just to render 5 lines.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 format_error_tail no longer decodes the entire stderr buffer; it locates the last N \n-separated segments via byte search and decodes only those
- [ ] #2 ring.into_iter().collect::<Vec<_>>().join("\n") is replaced with an allocation-free join (or a single String build with reserve)
- [ ] #3 Existing tests (and a new large-buffer regression test) still pass
<!-- AC:END -->
