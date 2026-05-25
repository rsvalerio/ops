---
id: TASK-1515
title: >-
  FN-4: deps format_upgrade_section takes bool is_breaking flag with three
  downstream branches
status: Done
assignee:
  - TASK-1645
created_date: '2026-05-19 07:27'
updated_date: '2026-05-25 17:41'
labels:
  - code-review-rust
  - fn
  - pattern
  - read
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:152-206` (`format_upgrade_section`)

**What**: `format_upgrade_section(out, title, entries, is_breaking: bool)` takes a boolean flag that drives three distinct behaviours: (1) whether to compute `latest_width` (line 170), (2) which `writeln!` shape to emit per row — with or without the `(latest …)` suffix (lines 176-198), and (3) which advice string to print (line 200). Call sites at lines 93-105 pass `false` for compatible and `true` for breaking upgrades.

A bare bool at the call boundary is classic boolean-blindness: a reader at the call site sees `false, true` and must walk to the callee to learn what mode that selects. A typed enum (e.g. `enum UpgradeKind { Compatible, Breaking }`) makes both call sites self-documenting and makes adding a third upgrade class (yanked? security-pinned?) a `match` exhaustiveness error rather than a silent `if !is_breaking` mismatch.

**Why it matters**: FN-4 (boolean parameter steering multi-branch behaviour). Adjacent rules: PATTERN-1 (use the type system to encode states, not flags) and READ-5 (call-site readability).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 format_upgrade_section's is_breaking: bool replaced with a typed enum (UpgradeKind or similar) covering Compatible and Breaking variants
- [ ] #2 Both call sites in format_report (lines 93-105) pass the enum variants explicitly; no bool flag remains in the signature
- [ ] #3 Existing format_report_with_upgrades / format_report_with_breaking_upgrades_shows_advice tests continue to pass without change in expected output
<!-- AC:END -->
