---
id: TASK-0895
title: >-
  PERF-1: AboutOptions::from_ref still clones each String, contradicting its
  stated purpose
status: Done
assignee: []
created_date: '2026-05-02 09:47'
updated_date: '2026-05-02 11:12'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions/about/src/lib.rs:83

**What**: from_ref accepts Option<&[String]> and immediately does visible_fields.map(|f| f.to_vec()), which clones every String. The CLI now allocates a new Vec instead of reusing the one already in Config — same allocation cost as before, plus an extra slice indirection. The doc/intent (per TASK-0763) is to avoid cloning config.about.fields, which the implementation does not actually achieve.

**Why it matters**: The avoid-cloning justification at the call site (crates/cli/src/subcommands.rs:47) is misleading. Future maintainers may believe the clone has been removed when it has not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change AboutOptions.visible_fields to Cow<'_, [String]> or Arc<[String]> so from_ref truly avoids cloning, OR
- [ ] #2 Remove from_ref and revert the call site (the savings are illusory), OR
- [ ] #3 Switch Config.about.fields to Arc<[String]> so the clone is genuinely O(1) and from_ref becomes unnecessary
- [ ] #4 Update the rustdoc on the chosen API to honestly describe its allocation behavior
<!-- AC:END -->
