---
id: TASK-0808
title: >-
  READ-5: load_coverage swallows DataIngestor::load result via _load_result then
  drops it
status: Done
assignee:
  - TASK-0822
created_date: '2026-05-01 06:02'
updated_date: '2026-05-01 07:02'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:280-286`

**What**: load_coverage accepts data_dir and db, calls init_schema, then `let _load_result = ingestor.load(data_dir, db)?;` and returns Ok(()). The LoadResult carries record_count and source_name — exactly the diagnostic the rest of the system uses to confirm a load actually happened. Discarding it via _load_result makes the public function indistinguishable from a no-op load.

**Why it matters**: load_coverage is the public entry point downstream tooling calls; throwing away the structured load report means callers cannot tell if 0 rows ingested vs N rows ingested. Sister-tasks TASK-0606 and TASK-0651 are exactly about making record_count an enforceable health signal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either propagate LoadResult from the public function (signature change) or assert/log record_count before discarding so a 0-row load surfaces in tracing
- [ ] #2 If the design intent is fire-and-forget, document it explicitly with a comment and link to the contract
- [ ] #3 Add a test that loads zero records and verifies the chosen path emits a tracing::warn or fails as appropriate
<!-- AC:END -->
