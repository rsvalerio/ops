---
id: TASK-2569
title: 'READ-13: git.rs docs narrate fix history and carry rule-ID self-report prefixes'
status: To Do
assignee: []
created_date: '2026-10-10 15:42'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/hook-common/src/git.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/hook-common/src/git.rs:git module docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:311-321,657-661,739-743`; rule-ID prefixes at `13,31,187,238,276,394,430,531,592,631,657,737,763,815,847`

**What**: Documentation narrates the journey that produced the code rather than the end state: the `resolve_absolute_gitdir` doc ends with 'Realistic impact of the old shape was bounded by filesystem permissions ... The asymmetry is what made it a finding: two spellings of the same input got very different scrutiny' (311-321); test docs open with 'Before the fix the absolute branch returned the target verbatim and the hook installer wrote an executable script into it' (739-743) and narrate the pre-fix unfloored-anchor behaviour (657-661). Separately, 15 `///` docs open with review-rule self-report prefixes (`/// SEC-14: ...`, `/// SEC-33: ...`, `/// ERR-7: ...`) — a process artifact meaningless to a reader using the API. The same prefixes were stripped from crates/theme in e5b5443f (prose kept, tag dropped), a cleanup this crate has not yet received.

**Why it matters**: READ-13 — 'why we picked X over Y' essays and self-reports of which guidelines a change followed are process artifacts: they go stale on the next change while looking authoritative. The enduring containment contract (what is accepted, what is refused, why the anchor is floored) is the part worth keeping.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 History narration (before-the-fix sentences, old-shape impact, what-made-it-a-finding) is removed; the end-state containment rules stay
- [ ] #2 No doc comment in the file opens with a rule-ID prefix
- [ ] #3 cargo test -p ops-hook-common passes
<!-- AC:END -->
