---
id: TASK-2191
title: 'READ-13: ops-about-go docs narrate past bugs and task IDs instead of describing the current behaviour'
status: Done
assignee: []
created_date: '2026-09-08 07:14'
updated_date: '2026-09-10 19:31'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2248'
modified_files:
  - extensions-go/about/src/lib.rs
  - extensions-go/about/src/go_mod.rs
  - extensions-go/about/src/go_syntax.rs
  - extensions-go/about/src/go_work.rs
  - extensions-go/about/src/modules.rs
priority: low
ordinal: 104000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: all five modules — `extensions-go/about/src/lib.rs`, `go_mod.rs`, `go_syntax.rs`, `go_work.rs`, `modules.rs`

**What**: Nearly every doc comment and inline comment in this crate is written as a changelog entry rather than a description of the end state. A sample:

- `go_mod.rs:18` — "PATTERN-1 (TASK-1727): ... Before this, `module (` fell through to the prefix matcher and set the module name to the literal `\"(\"`."
- `go_mod.rs:120` — "PATTERN-1 (TASK-1727): cmd/go *requires* quoting ... so none of the `./`, `../`, `/` prefix arms below matched and the local replace was dropped."
- `go_mod.rs:159` — a nine-line rationale on `looks_like_module_version` narrating the previous `v<digit> + contains('.')` heuristic and everything it accepted.
- `go_syntax.rs:44-60` — the `is_block_opener` body carries three stacked TASK-numbered paragraphs explaining what the "prior shape" returned.
- `modules.rs:44-58` — a fifteen-line inline comment inside `ProjectUnit::new(...)` covering TASK-1085, the `""` vs `"."` sentinel history, and a cross-stack invariant.
- `modules.rs:75-135` — `unit_from_use_dir`'s ~35 lines of code are interleaved with ~35 lines of TASK-1208 / TASK-1027 / TASK-1721 rationale describing three superseded implementations.
- `lib.rs:63` and `go_work.rs:8` carry the same shape at module level.

Test docs repeat it: most `#[test]` fns are prefixed with "TASK-NNNN: the previous X did Y".

**Why it matters**: The comments describe code that no longer exists, so a reader must reconstruct the current rule from a list of things that used to be wrong. It also makes the real invariants (Go's quoting grammar, the `..`-past-a-real-segment traversal policy, the `"."` root sentinel) hard to find — they are buried inside migration prose. Git history and the backlog already record why each change happened.

This is the Go-stack instance of the pattern already filed as TASK-2136 (run-before-commit), TASK-2147 (run-before-push) and TASK-2169 (text-fixers); apply the same rewrite policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Doc and inline comments state the rule the code enforces now, without narrating superseded implementations
- [x] #2 TASK-NNNN references are removed from doc comments, or reduced to a single trailing reference where a regression test genuinely needs one
- [x] #3 The invariants currently buried in the prose (Go token quoting, the traversal policy, the "." root-module sentinel, go.work precedence) survive the rewrite as explicit statements
- [x] #4 Test names or one-line docs carry the behaviour under test rather than the bug that motivated it; no test is deleted

<!-- AC:END -->
