---
id: TASK-2553
title: 'READ-13: ops-git Cargo.toml comment carries DUP-3 / TASK-2014 provenance tags on the dev-dependency rationale'
status: To Do
assignee: []
created_date: '2026-10-10 15:39'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/git/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/git/Cargo.toml:[dev-dependencies]'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/Cargo.toml:19-21`

**What**: The `[dev-dependencies]` comment opens with `# DUP-3 / TASK-2014:` before explaining that the config tests' tracing-capture harness lives in `ops_core::test_utils` behind `test-support`. The rule-ID + task-ID prefix is a self-report of which guideline a past change followed — a process artifact, not a description of the manifest's end state.

**Why it matters**: READ-13: self-reports of which guidelines a change followed are journey narration; they go stale while looking authoritative, and the backlog IDs are unresolvable for a reader of the manifest. The explanation itself is durable and worth keeping (it says why `ops-core` re-appears with the `test-support` feature). Recent commits stripped exactly this pattern from sibling crates (docs(about)/docs(theme)/docs(sqlite) task-provenance removals), so this is residual. Note the location is a manifest comment rather than a /// block — flagged under the same rule on the strength of the project's own cleanup direction; reject at triage if deemed out of scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Comment states the end-state fact (config tests use the ops_core::test_utils tracing-capture harness behind test-support) with no DUP-/TASK- identifier prefix
- [ ] #2 cargo check -p ops-git --all-targets still resolves dev-dependencies unchanged
<!-- AC:END -->
