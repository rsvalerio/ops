---
id: TASK-2482
title: 'READ-13: types module docs narrate TASK history instead of the end state'
status: To Do
assignee: []
created_date: '2026-10-10 15:30'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - read
dependencies: []
parent_task_id: 'TASK-2615'
modified_files:
  - extensions-rust/cargo-toml/src/types.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/cargo-toml/src/types.rs:types'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/types.rs:52`

**What**: Item docs in types.rs carry rule/task provenance tags and before/after narration. Locations:
- types.rs:52-57 — `dev_dependencies` field doc narrates "READ-6 / TASK-1798: rename, not alias ... With alias the JSON emitted by DataProvider::provide carried the Rust name, so a consumer reading the documented key silently got nothing."
- types.rs:183-197 — the `InheritableField` doc section "Absence is a state, not a sentinel (ERR-6 / TASK-1793)" narrates "Before TASK-1793 the default was Value(T::default()), so package_version returned Some("") ... That is truthy in an Option-based fallback chain ...", describing consumer bugs a past default produced.
- types.rs:294-303 — `is_publishable` doc narrates "API / TASK-1196: previously returned bool and matched Inherited { .. } as true, silently flipping the safe default ...".

**Why it matters**: READ-13 — documentation describes the end state, not the journey. The invariant explanations (absent-vs-empty distinction, unresolved-Inherited-maps-to-None) are enduring and must stay; the TASK tags and prior-implementation narration around them are process artifacts that go stale while looking authoritative. The project has been stripping exactly this shape from sibling crates.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Docs on CargoToml fields, InheritableField, and PublishSpec::is_publishable state the current semantics with no RULE-ID/TASK-XXXX references and no narration of previous behaviour
- [ ] #2 The durable invariants (Absent vs empty-string vs Inherited semantics; unresolved Inherited returning None from is_publishable) remain documented in present tense
<!-- AC:END -->
