---
id: TASK-0802
title: >-
  PERF-3: format_report uses out.push_str(&format!(...)) repeatedly, allocating
  an intermediate String per push
status: Done
assignee:
  - TASK-0821
created_date: '2026-05-01 06:01'
updated_date: '2026-05-01 06:45'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:53-99`

**What**: Throughout format_report, format_upgrade_section, format_advisories, format_bans_summary, format_deny_section, every line is built via out.push_str(&format!("...")). Each call allocates a fresh String only to copy it into out.

**Why it matters**: write!(out, ...) (with out: &mut String and std::fmt::Write in scope) writes directly into the destination, eliminating the intermediate allocation. For a workspace with hundreds of advisories/upgrades, this is hundreds of avoidable allocations per render.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace out.push_str(format!(...)) with write!(&mut out, ...) (importing std::fmt::Write)
- [ ] #2 Apply consistently across format_report, format_upgrade_section, format_advisories, format_bans_summary, format_deny_section
- [ ] #3 Output bytes unchanged
<!-- AC:END -->
