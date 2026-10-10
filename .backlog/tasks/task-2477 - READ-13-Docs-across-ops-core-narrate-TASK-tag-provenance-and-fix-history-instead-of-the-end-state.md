---
id: TASK-2477
title: 'READ-13: Docs across ops-core narrate TASK-tag provenance and fix history instead of the end state'
status: To Do
assignee: []
created_date: '2026-10-10 15:29'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2610'
modified_files:
  - crates/core/src/expand.rs
  - crates/core/src/text.rs
  - crates/core/src/config/loader/mod.rs
  - crates/core/src/config/edit.rs
  - crates/core/src/stack/mod.rs
  - crates/core/src/project_identity/card.rs
  - crates/core/src/output.rs
  - crates/core/src/subprocess/drain.rs
  - crates/core/src/config/commands.rs
  - crates/core/src/subprocess/mod.rs
  - crates/core/src/stack/detect.rs
  - crates/core/src/config/root.rs
  - crates/core/src/config/extend.rs
  - crates/core/src/ui.rs
  - crates/core/src/table.rs
  - crates/core/src/subprocess/cap.rs
  - crates/core/src/config/loader/conf_d.rs
  - crates/core/src/style.rs
  - crates/core/src/config/sections.rs
  - crates/core/src/config/merge.rs
  - crates/core/src/config/loader/env.rs
  - crates/core/src/project_identity/format.rs
  - crates/core/src/project_identity.rs
  - crates/core/src/config/loader/global.rs
  - crates/core/src/config/theme_types.rs
  - crates/core/src/sync.rs
  - crates/core/src/paths.rs
  - crates/core/src/config/mod.rs
  - crates/core/src/bounded_read.rs
  - crates/core/src/config/clone.rs
  - crates/core/src/stack/metadata.rs
  - crates/core/src/config/strategy.rs
  - crates/core/src/config/overlay.rs
  - crates/core/src/config/locked.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/core/src/expand.rs:module docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: production docs across `crates/core/src` — TASK-tag first-occurrence anchors and counts per file: `expand.rs:12` (59 occurrences), `text.rs:8` (43), `config/loader/mod.rs:3` (40), `config/edit.rs:26` (31), `stack/mod.rs:21` (27), `project_identity/card.rs:19` (23), `output.rs:15` (23), `subprocess/drain.rs:1` (21), `config/commands.rs:3` (21), `subprocess/mod.rs:13` (19), `stack/detect.rs:3` (17), `config/root.rs:22` (16), `config/extend.rs:14` (15), `ui.rs:16` (14), `table.rs:28` (11), `subprocess/cap.rs:1` (11), `config/loader/conf_d.rs:1` (11), `style.rs:3` (10), `config/sections.rs:50` (10), `config/merge.rs:23` (9), `config/loader/env.rs:1` (8), `project_identity/format.rs:10` (7), `project_identity.rs:104` (7), `config/loader/global.rs:146` (6), `config/theme_types.rs:175` (5), `sync.rs:1` (4), `paths.rs:6` (4), `config/mod.rs:18` (4), `bounded_read.rs:4` (4), `config/clone.rs:4` (3), `stack/metadata.rs:3` (2), `config/strategy.rs:2` (1), `config/overlay.rs:3` (1), `config/locked.rs:2` (1) — ~488 tagged passages in production regions (test modules under `config/tests/`, `project_identity/tests.rs`, and `test_utils.rs` excluded from this count).

**What**: Doc comments describe how the code got here rather than what it does. Nearly every module and item opens with a rule/TASK prefix — `SEC-33 (TASK-0932): default cap on manifest-style file reads` (text.rs:8), `ARCH-1 / TASK-1185: extracted from the monolithic stack.rs` (stack/metadata.rs:3), `API-9 / TASK-0858: #[non_exhaustive] mirrors ProjectIdentity` (project_identity.rs:104), `TRAIT-1 / TASK-1435: derives Debug and Clone` (card.rs:19), `ERR-1 / TASK-1040: Path::parent() returns Some("")` (config/edit.rs:220) — and many narrate past states and removed predecessors: "the prior per-call std::env::var lookup contended" (text.rs:21), "A hand-picked 'most-abused' subset was here before" (text.rs:570), "the already-closed TASK-1332 pattern" (card.rs:168), "which until TASK-1849 was the only config section nothing validated at all" (theme_types.rs:320). `text.rs:183-207` is a full "Decision: no root-anchored variant" design essay. One case escapes docs into an operator-facing runtime message: the `tracing::warn!` at `text.rs:399` embeds `ARCH-2 / TASK-2038:` verbatim in the message string an operator reads when a manifest read is refused.

**Why it matters**: The rule-of-thumb for READ-13: if the text would be deleted verbatim by someone who joined after the decision, it belongs in the PR description or an ADR. The TASK/rule prefixes are meaningless outside the review backlog, and the "used to / pre-fix / until TASK-N" passages go stale as the code moves on. Keep the enduring invariants (the threat models, the refusal surfaces, the cap contracts) and delete the provenance. The repo is already doing this cleanup wave by wave (commits `e5b5443f` docs(theme), `1d9bc644` docs(about)); ops-core is the largest remaining surface. For `text.rs:399` the fix is not just deletion — the operator-facing message should lead with the refusal and the remedy, not the backlog reference.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Item and module docs describe current behaviour and invariants only; TASK/rule-id prefixes, 'used to'/'previously'/'pre-fix' passages, and design-decision essays are removed
- [ ] #2 The tracing::warn! message at text.rs:399 no longer embeds backlog references; it states the refusal and the operator remedy
<!-- AC:END -->
