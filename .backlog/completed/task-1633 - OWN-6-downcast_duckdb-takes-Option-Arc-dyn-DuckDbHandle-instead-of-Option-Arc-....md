---
id: TASK-1633
title: >-
  OWN-6: downcast_duckdb takes &Option<Arc<dyn DuckDbHandle>> instead of
  Option<&Arc<...>>
status: Done
assignee:
  - TASK-1640
created_date: '2026-05-22 07:17'
updated_date: '2026-05-22 13:43'
labels:
  - code-review-rust
  - ownership
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`extensions/duckdb/src/lib.rs:25\`

**What**: \`fn downcast_duckdb(handle: &Option<Arc<dyn ops_extension::DuckDbHandle>>) -> Option<&DuckDb>\` accepts a reference-to-Option, then immediately calls \`.as_ref()\` to convert it into \`Option<&Arc<...>>\`. Per OWN-6, the conventional Rust idiom is to accept the already-converted form so callers don't have to construct an \`Option\` reference at the call site.

Both call sites (\`try_provide_from_db\` line 45 cloning the Arc, \`get_db\` line 53) already have a clean \`&ctx.db\` form, so the change is cosmetic but it aligns the helper with idiomatic Rust and removes the redundant \`.as_ref()\` from inside the helper.

**Why it matters**: OWN-6 is a Low-severity readability/idiom issue. The fix is mechanical and improves API ergonomics: callers can pass \`ctx.db.as_ref()\` (which they already implicitly do via the function body), and the helper signature matches similar helpers across the workspace.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 downcast_duckdb signature reads Option<&Arc<dyn DuckDbHandle>>
- [ ] #2 try_provide_from_db and get_db updated to pass already-borrowed Option
- [ ] #3 All existing tests still pass (duck_db_provider_*, etc)
<!-- AC:END -->
