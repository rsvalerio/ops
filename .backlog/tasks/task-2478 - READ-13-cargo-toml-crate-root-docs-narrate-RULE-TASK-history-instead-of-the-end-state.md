---
id: TASK-2478
title: 'READ-13: cargo-toml crate-root docs narrate RULE/TASK history instead of the end state'
status: Done
assignee: []
created_date: '2026-10-10 15:29'
updated_date: '2026-10-10 21:42'
labels:
  - code-review
  - read
dependencies: []
parent_task_id: 'TASK-2615'
modified_files:
  - extensions-rust/cargo-toml/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/cargo-toml/src/lib.rs:cargo-toml'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:75`

**What**: The crate-root docs carry process narration and rule/task provenance tags. Locations:
- lib.rs:75-94 — the `pub use` block comment narrates the ARCH-4/TASK-1795 re-export decision history and which items were "deliberately not re-exported" as a record of a past review decision.
- lib.rs:213-223 — `resolve_root` doc opens with "SEC-25 / TASK-2143: the data-provider path resolves its root with the strict ancestor walk", then contrasts it with what the lenient walk "differs on ... instead of being skipped".
- lib.rs:244-250 — `provide_typed` doc opens "PERF-1 / TASK-1195: produce the typed CargoToml directly, skipping the ... round-trip", narrating why the method was added.
- lib.rs:289-295 — `schema` doc opens "READ-6 / TASK-1798: every name below is a key that actually appears ..." and narrates "before it, the schema advertised dev-dependencies while serde emitted dev_dependencies".

**Why it matters**: READ-13 — documentation describes the end state, not the journey that produced it. RULE/TASK tags and before/after narration are process artifacts: meaningless to a reader using the API, stale on the next change, while looking authoritative. The project has been stripping exactly this shape (commits "strip task tags and history narration from docs and tests", "remove task provenance tags from module docs").

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Crate-root and item docs in lib.rs describe current behaviour only, with no RULE-ID/TASK-XXXX references and no narration of past implementations or review decisions
- [x] #2 Enduring semantics the docs carry today (e.g. why the strict walk is used, what the schema keys mean) are preserved as present-tense statements

<!-- AC:END -->
