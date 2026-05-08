---
id: TASK-1157
title: >-
  DUP-3: BufWriter+MakeWriter tracing-capture harness duplicated across four
  modules
status: To Do
assignee:
  - TASK-1265
created_date: '2026-05-08 07:44'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:684`

**What**: The exact same `BufWriter(Arc<Mutex<Vec<u8>>>)` + Write + MakeWriter shim appears verbatim in at least four places: `extensions-rust/about/src/query.rs:684-700` (twice — also at :774-790), `extensions-rust/about/src/coverage_provider.rs:184-202`, `extensions-rust/metadata/src/ingestor.rs:444-460`. Each copy is ~17 lines.

**Why it matters**: DUP-3 threshold (3+ occurrences) hit. Style drift across copies leads to inconsistent log capture in tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move BufWriter (or equivalent tracing capture wrapper) into a shared dev helper crate or test_support module under extensions-rust/
- [ ] #2 Replace per-file copies with a single use of the shared helper
<!-- AC:END -->
