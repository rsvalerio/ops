---
id: TASK-1536
title: >-
  DUP-3: cargo-update tests.rs duplicates BufWriter+MakeWriter tracing-capture
  scaffold across two tests
status: To Do
assignee:
  - TASK-1575
created_date: '2026-05-19 09:54'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - dup
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/tests.rs:160-181, 552-570`

**What**: The `BufWriter(Arc<Mutex<Vec<u8>>>)` newtype with hand-rolled `io::Write` and `tracing_subscriber::fmt::MakeWriter` impls is defined inline twice — once inside `arrow_drift_and_extra_tokens_warn_fires_with_expected_entries` and once inside `parse_skips_two_token_updating_registry_form_no_warn`. The two copies are byte-identical except for the surrounding imports (one uses `use std::io::Write; use std::sync::{Arc, Mutex};` at the function top, the other uses fully-qualified paths inline). The same scaffold is also tracked in the wider workspace as TASK-1494 (deps crate, duplicated 3×) — confirming this is a recurring sub-pattern, not local.

**Why it matters**: Each new format-drift / no-warn test invented after this point will paste the scaffold a third time. Lifting it into a `test-support` helper (or a `mod test_log_capture` inside `tests.rs`) gives each test ~5 lines of setup instead of ~20, removes one source of drift if the `MakeWriter` API moves, and makes the intent of each test — what was logged — visible without the capture boilerplate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 BufWriter+MakeWriter scaffold is defined in exactly one place inside the tests module (helper fn, sub-module, or test-support item) and reused by both call sites.
- [ ] #2 Both arrow_drift_and_extra_tokens_warn_fires_with_expected_entries and parse_skips_two_token_updating_registry_form_no_warn drive their assertions through the shared helper, with no remaining inline BufWriter definitions.
- [ ] #3 Tests still assert the same WARN-line invariants (format-drift / trailing-tokens / no-warn) and continue to pass.
<!-- AC:END -->
