---
id: TASK-0549
title: >-
  DUP-1: format_language_stats_section recomputes loc_pct, ignoring
  LanguageStat::loc_pct/files_pct
status: Triage
assignee: []
created_date: '2026-04-29 05:02'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/code.rs:46`

**What**: format_language_stats_section derives the percentage from stat.loc / total_loc even though LanguageStat already carries authoritative loc_pct and files_pct populated by query_project_languages (which uses DuckDB full-table denominator). The render-side recomputation re-derives loc_pct from the post-`>= 0.1` filtered subset, so cards including only a few languages display percentages summing to 100% even though tiny languages were elided. files_pct is dropped entirely.

**Why it matters**: Two sources of truth for the same percentage; rendered numbers can disagree with stored ones the moment the filter trims sub-0.1% languages.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Renderer uses stat.loc_pct directly (and either renders or drops files_pct consistently)
- [ ] #2 A test exercises a multi-language case where the upstream loc_pct and the post-filter recompute differ and confirms the rendered value matches the stored one
<!-- AC:END -->
