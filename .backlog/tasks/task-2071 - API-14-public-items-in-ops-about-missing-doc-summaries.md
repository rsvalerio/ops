---
id: TASK-2071
title: 'API-14: public items in ops-about missing doc summaries'
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-07 22:55'
updated_date: '2026-09-08 10:59'
labels:
  - code-review-rust
  - api
dependencies: []
modified_files:
  - extensions/about/src/cards.rs
  - extensions/about/src/text_util.rs
  - extensions/about/src/deps.rs
  - extensions/about/src/code.rs
  - extensions/about/src/units.rs
  - extensions/about/src/coverage.rs
priority: low
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs:64,83`; `extensions/about/src/text_util.rs:11,321`; `extensions/about/src/deps.rs:14,53`; `extensions/about/src/code.rs:40`; `extensions/about/src/units.rs:15`; `extensions/about/src/coverage.rs:16`

**What**: <!-- scan confidence: candidates to inspect --> Several public items carry no doc summary at all, in a crate whose other public items are meticulously documented:

- `cards.rs`: `pub fn build_card_stats_line` (line 64), `pub fn render_card` (line 83)
- `text_util.rs`: `pub fn get_terminal_width` (line 11 — rationale is a comment inside the body, invisible in rustdoc), `pub fn tty_style` (line 321)
- `deps.rs`: `pub const PROJECT_DEPENDENCIES_PROVIDER` (line 14), `pub fn format_dependencies_section` (line 53)
- `code.rs`: `pub fn format_language_stats_section` (line 40)
- `units.rs`: `pub const PROJECT_UNITS_PROVIDER` (line 15)
- `coverage.rs`: `pub const PROJECT_COVERAGE_PROVIDER` (line 16)

**Why it matters**: API-14 — the summary sentence is mandatory on public items and is what rustdoc lifts into the module index. The `run_about_*` entry points all have `# Errors` sections and summaries; these render/format helpers and provider-name consts are the gaps.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every listed public item has a first-paragraph doc summary (~15 words) followed by a blank line
- [ ] #2 Provider-name consts document what provider supplies the data and what the subpage does when it is absent
<!-- AC:END -->
