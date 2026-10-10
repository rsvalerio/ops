---
id: TASK-2462
title: 'READ-13: units.rs docs narrate deferred-migration and design-choice rationale'
status: To Do
assignee: []
created_date: '2026-10-10 15:27'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - readability
dependencies: []
parent_task_id: 'TASK-2621'
modified_files:
  - extensions-node/about/src/units.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-node/about/src/units.rs:units'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:57`, `extensions-node/about/src/units.rs:113`, `extensions-node/about/src/units.rs:226`

**What**: Doc comments narrate how the design was chosen rather than the end state:
- units.rs:226 (in the `parse_pnpm_workspace_yaml` doc): a deferred-migration note - a long-term fix delegating to a real YAML crate, with the current code framed as the until-then state.
- units.rs:57 (inline comment in `collect_units`): the per-stack PackageProbe placement is explained by comparing it to a rejected alternative (a parallel shim).
- units.rs:113 (comment in `workspace_member_globs`): narrates the cache-sharing arrangement against the identity provider site.

**Why it matters**: READ-13 - process artifacts go stale while looking authoritative; the enduring facts (single IO per manifest via the shared cache, unsupported YAML shapes logged at debug) are worth keeping, the journey text belongs in an ADR or PR description.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Doc comments state current behavior only; the deferred-migration sentence and the rejected-alternative comparisons are removed or moved to an ADR
- [ ] #2 cargo test -p ops-about-node passes unchanged
<!-- AC:END -->
