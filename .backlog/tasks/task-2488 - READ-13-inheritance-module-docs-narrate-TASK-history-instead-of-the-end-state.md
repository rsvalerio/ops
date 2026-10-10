---
id: TASK-2488
title: 'READ-13: inheritance module docs narrate TASK history instead of the end state'
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
  - extensions-rust/cargo-toml/src/inheritance.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/cargo-toml/src/inheritance.rs:inheritance'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:117`

**What**: Doc comments across inheritance.rs open with rule/task provenance tags and narrate past states. Locations:
- inheritance.rs:117-121 — resolve_vec_field doc narrates "TASK-0961: WorkspacePackage::keywords/categories are plain Vec<String> ... so an absent workspace keywords table is indistinguishable from keywords = []".
- inheritance.rs:147-160 — resolve_publish doc section "Fail-closed rule (SEC-31 / TASK-1789)" narrates "the exact signal loss TASK-1196 introduced Option<bool> to prevent (cargo itself hard-errors on this manifest shape)".
- inheritance.rs:222-227 — resolve_from_simple_dep doc narrates "DUP-7 / TASK-1804: restating the nine default fields here made DetailedDepSpec exhaustively constructed in three places".
- inheritance.rs:260-264 — resolve_from_detailed_dep doc narrates "DUP-7 / TASK-1804: unlike resolve_from_simple_dep, this constructor stays exhaustive on purpose ...".
- inheritance.rs:301 — merge_features comment opens "PERF-2 (TASK-0807): feature lists are typically tiny ...".
- inheritance.rs:318-320, 332-334 — test doc comments carry "TASK-0385:" / "TASK-0961:" tags narrating the behaviour being pinned.

**Why it matters**: READ-13 — documentation describes the end state, not the journey. The precedence rationale itself (empty workspace vec does not substitute; exhaustive literal is a compile-time guard; linear scan beats hashing for tiny lists) is enduring and must stay; the TASK tags and how-it-used-to-be framing are process artifacts. The project has been stripping exactly this shape from sibling crates.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Docs in inheritance.rs (including test doc comments) carry no RULE-ID/TASK-XXXX references and no narration of previous implementations
- [ ] #2 The durable rationale (empty-vec no-substitute rule, fail-closed publish resolution, exhaustive-constructor guard, linear-scan choice) remains as present-tense statements
<!-- AC:END -->
