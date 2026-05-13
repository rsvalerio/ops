---
id: TASK-1430
title: >-
  API-1: CommandSpec untagged+deny_unknown_fields produces misleading 'missing
  field' errors on Exec-side typos
status: To Do
assignee:
  - TASK-1456
created_date: '2026-05-13 18:23'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - API
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/commands.rs:17`

**What**: `#[serde(untagged)]` on `CommandSpec` combined with `deny_unknown_fields` on each variant means a typo like `progam = "echo"` (Exec intent) is attempted as Composite first, fails, and serde returns the *Composite* error ("missing field commands") instead of the Exec one.

**Why it matters**: Confusing config errors at load time; users see an error for a field they never tried to set. A custom Deserialize that disambiguates by key presence (or a tagged enum) returns the user-relevant error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Custom Deserialize picks Exec vs Composite by key presence and reports the variant-relevant error
- [ ] #2 Regression test pinning the improved error message for a typo'd Exec field and a typo'd Composite field
<!-- AC:END -->
