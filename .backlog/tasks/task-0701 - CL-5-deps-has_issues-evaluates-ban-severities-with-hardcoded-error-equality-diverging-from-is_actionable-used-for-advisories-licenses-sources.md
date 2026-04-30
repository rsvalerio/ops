---
id: TASK-0701
title: >-
  CL-5: deps has_issues evaluates ban severities with hardcoded 'error'
  equality, diverging from is_actionable used for advisories/licenses/sources
status: Done
assignee:
  - TASK-0738
created_date: '2026-04-30 05:27'
updated_date: '2026-04-30 18:36'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:237`

**What**: `has_issues` runs `is_actionable(&e.severity)` on advisories, licenses, and sources but checks bans with `e.severity == "error"` only. The doc comment immediately above (line 198) explains that ban warnings (duplicate-crate notices) are intentionally informational, but the hardcoded equality also treats every *unknown* severity as benign — exactly the failure mode that motivated TASK-0601 for the other categories. A future cargo-deny severity (e.g. `critical`) on a banned-crate diagnostic would silently pass CI while the same severity on an advisory would fail it.

**Why it matters**: CI-gating rule diverges across the four `DenyResult` categories without a stated reason, so a single schema-drift event surfaces inconsistently — the supply-chain gate is the wrong place for unknown-equals-benign defaults. Either reuse `is_actionable` and explicitly filter the documented "duplicate" code, or document why bans alone fail-open for unknown severities.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 has_issues evaluates ban severities through the same is_actionable predicate (or a deliberately documented variant) used for the other DenyResult categories
- [ ] #2 unknown ban severities fail-closed (or trigger an explicit tracing::warn) the same way TASK-0601 does for advisories/licenses/sources
- [ ] #3 test pins the chosen behaviour against a 'critical'-severity ban entry
<!-- AC:END -->
