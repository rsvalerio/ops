---
id: TASK-0560
title: >-
  PATTERN-1: is_component_in_list uses open-ended prefix match against rustup
  component lines
status: Triage
assignee: []
created_date: '2026-04-29 05:03'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:167-172`

**What**: is_component_in_list strips -preview from the search term, then does line.trim().starts_with of base+dash. The asymmetry between stripping -preview from the search term but not from listed lines makes the matcher hard to reason about, and the open-ended prefix could match siblings like base-foo-triple even when -preview was the only intended widening.

**Why it matters**: A user expecting auto-install to honor -preview listings or to never-match unrelated base-foo siblings cannot tell which is which from the implementation; the test surface only covers the exact-match path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Match against the listed line target-triple boundary explicitly (split on -arch-vendor-) instead of an open-ended prefix
- [ ] #2 Add tests covering clippy-preview-aarch64-apple-darwin listed when caller searches for clippy, and ensure unrelated siblings do not match
<!-- AC:END -->
