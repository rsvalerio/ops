---
id: TASK-0747
title: 'PERF-1: parse_spec re-parses each color spec on every styled segment'
status: Triage
assignee: []
created_date: '2026-05-01 05:52'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style.rs:20-46, 108-113`

**What**: `apply_style_gated` calls `parse_spec(spec)` per invocation. StepLineTheme::render calls apply_style 3-4 times per step always with the same constant strings. Spec is split/filtered/mapped and a Vec<&'static str> is allocated per call.

**Why it matters**: Spec is owned by ThemeConfig (immutable for the run); parsed SGR prefix can be precomputed once at construction.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ConfigurableTheme precomputes the SGR prefix string per color slot at construction
- [ ] #2 apply_style accepts a precomputed prefix (or uses a OnceLock per slot)
- [ ] #3 Existing color/styling tests pass without modification
<!-- AC:END -->
