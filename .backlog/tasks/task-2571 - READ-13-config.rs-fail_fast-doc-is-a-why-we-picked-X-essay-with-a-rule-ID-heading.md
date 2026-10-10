---
id: TASK-2571
title: 'READ-13: config.rs fail_fast doc is a why-we-picked-X essay with a rule-ID heading'
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
  - extensions/hook-common/src/config.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/hook-common/src/config.rs:ensure_config_command docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/config.rs:20-41`

**What**: The `ensure_config_command` doc carries a three-part '# fail_fast policy (PATTERN-1)' section. Parts 1-2 ('Why true is the default', 'Why the knob is intentionally not exposed' — the latter arguing about what adding a field would ripple into the impl_hook_wrappers! macro and both hook crates) are a design-decision essay about choices already made. Only part 3 ('How operators flip it to false') and the '# Errors' section describe behavior a caller needs. The PATTERN-1 heading is the same rule-ID self-report prefix stripped from crates/theme in e5b5443f.

**Why it matters**: READ-13 — 'why we picked X over Y' essays are process artifacts: meaningless to a caller using the function and stale on the next change while looking authoritative. The operator override path and the reinstall-preservation guarantee are the end-state content worth keeping.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The fail_fast section states the end-state contract (hardcoded true, how to override, override preserved across reinstalls) without the design-rationale essay or rule-ID heading
- [ ] #2 cargo test -p ops-hook-common passes
<!-- AC:END -->
