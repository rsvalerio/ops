---
id: TASK-1494
title: >-
  DUP-3: BufWriter+MakeWriter tracing-capture scaffold duplicated 3x inside
  deps/src/tests.rs
status: To Do
assignee:
  - TASK-1574
created_date: '2026-05-18 17:28'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/tests.rs:149-197`, `:355-409`, `:1437-1486`

**What**: Three independent tests in `deps/src/tests.rs` each open-code the same `BufWriter(Arc<Mutex<Vec<u8>>>)` + `impl MakeWriter` + `tracing_subscriber::fmt()` wiring used to capture `tracing::warn!` / `debug!` output:

- `parse_upgrade_table_warns_on_missing_separator` (lines 149–197)
- `interpret_upgrade_output_bails_on_unrecognised_header_with_separator` (lines 355–409)
- `parse_deny_output_skips_malformed_json_with_tracing` (lines 1437–1486)

Each instance re-declares an identical local `BufWriter` struct, identical `Write` impl, and identical `MakeWriter` impl. The third instance has slightly different formatter knobs (`with_ansi(false)` / `Level::DEBUG`), but the scaffold is mechanically the same.

Workspace-wide DUP-3 / DUP-1 work (TASK-1157, TASK-1279, TASK-1311, TASK-1429) has already factored the same scaffold elsewhere in the tree, so the rule is established and the consolidation target likely exists. The deps crate should either consume that shared helper or factor its own `mod test_support` inside the file.

**Why it matters**: Three independent copies of the same tracing-capture harness in a single file is a maintenance hazard — a fix or knob (e.g. switching to `tracing-test`, capturing structured fields, escaping ANSI) has to be replicated three times, and the file-local instances drift (the third already has `with_ansi(false)` while the first two do not).

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All three sites share one BufWriter+MakeWriter helper (file-local mod or workspace test-support crate)
- [ ] #2 Formatter knobs that legitimately differ (level, ansi) become parameters of the helper, not separate copies
- [ ] #3 Existing assertions still pass; tests remain serial-safe where they were before
<!-- AC:END -->
