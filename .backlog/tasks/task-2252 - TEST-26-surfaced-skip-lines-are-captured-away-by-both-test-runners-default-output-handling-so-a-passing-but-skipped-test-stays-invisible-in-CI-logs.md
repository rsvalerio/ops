---
id: TASK-2252
title: >-
  TEST-26: surfaced skip: lines are captured away by both test runners' default
  output handling, so a passing-but-skipped test stays invisible in CI logs
status: Triage
assignee: []
created_date: '2026-09-08 16:13'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - .github/workflows/ci.yml
priority: low
ordinal: 158000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `.github/workflows/ci.yml:174`

**What**: TASK-2126 / TASK-2166 added `ops_core::test_utils::skip_precondition`, which prints a `skip: <fixture>: <reason>` line whenever a test bails on an unmet precondition (no git on PATH, root EUID, missing /dev/zero). But the gating pipeline runs `cargo nextest run`, which captures per-test stdout/stderr and shows them only for failing tests; plain `cargo test` behaves the same via libtest capture. So in exactly the environments where the guards trip (root containers, git-less sandboxes), the surfaced skip is written into a captured buffer and never reaches the log a green run leaves behind.

**Why it matters**: the AC of both tasks was "a reader of CI output can see the assertion did not run". In gating CI (ubuntu-latest, non-root, git present) the guards do not trip, so the assertions genuinely run there — but any other pipeline running this suite as root still gets an indistinguishable green. The surfacing mechanism is correct at the source and lost at the sink.

**Origin**: discovered during TASK-2237 while fixing TASK-2126/TASK-2166. Candidate sinks: a nextest profile with `--success-output` scoped to these tests, a final "N skips surfaced" summary counter, or documenting `--nocapture` for container runs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A run where a skip_precondition line was printed is distinguishable in the gating CI log without also enabling full success-output noise for the whole suite
<!-- AC:END -->
