---
id: TASK-2587
title: 'READ-13: tokei tests narrate task history (used-to/they-now essays, TASK tags) instead of the end state'
status: To Do
assignee: []
created_date: '2026-10-10 15:46'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2626'
modified_files:
  - extensions/tokei/src/tests.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/tokei/src/tests.rs:mod-tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/tests.rs` — module doc `:14-17`; inline comments at `:179`, `:210-211`, `:227`, `:265`, `:378`, `:405`, `:546-548`, `:565`, `:613-615`, `:620`, `:639`, `:669-672`, `:680`, `:712`, `:737`, `:781-788`, `:796`, `:853-859`, `:888`

<!-- scan confidence: candidates to inspect -->

**What**: Test docs and comments carry backlog provenance rather than behavioral rationale. Candidates:
- `:14-17` module doc: "TEST-18 (TASK-1977): five tests used to scan the live crate directory without that gate... They now build a fixture_project tree" — history of a past fix.
- `:613-615`: "`load_tokei` was removed (DUP-1, TASK-0226): it duplicated the TokeiIngestor::load path... These tests now exercise the ingestor directly" — narrates a removed symbol.
- `:669-672`: "TEST-1 (TASK-1978): the single-entry-point invariant used to live here as a test with a fully commented-out body... It is now a compile_fail doctest".
- `:781-788`: "SQLite port note: the former tokei_files_create_sql_with_real_json test asserted the shape of the DuckDB builder... That builder is gone".
- `:853-859`, `:888`: "TASK-2156 correction: ...", "AC #3 / TASK-2156: ..." — acceptance-criteria provenance.
- Repeated provenance tags on otherwise-good behavioral comments: `CL-3 (TASK-1974)` at :179, :210; `SEC-33, TASK-1970` at :227; `CL-3 / TASK-2153` at :265; `ERR-2, TASK-1972` at :378, :405, :546-548; `SEC-25 / TASK-2054` at :565, :620, :639, :680, :712, :737, :796.

**Why it matters**: READ-13 — the task/rule IDs and used-to/they-now narration go stale and belong in commit history; the behavioral rationale (why the fixture is deterministic, why the ingestor is driven directly) is the part worth keeping, stated as present-tense fact. Matches the cleanup already applied to lib.rs (TASK-2415) and to sibling crates (TASK-2136, TASK-2147).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test comments state the behavior each test pins, without TASK-/AC-#/rule-tag prefixes or narration of removed tests and past states
- [ ] #2 The isolation-policy section of the module doc survives as policy; its TASK-1977 history paragraph does not
<!-- AC:END -->
