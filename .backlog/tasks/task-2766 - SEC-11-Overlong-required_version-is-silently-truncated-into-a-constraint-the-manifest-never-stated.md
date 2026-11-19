---
id: TASK-2766
title: 'SEC-11: Overlong required_version is silently truncated into a constraint the manifest never stated'
status: To Do
assignee: []
created_date: '2026-11-18 18:43'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2856'
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-11:extensions-terraform/about/src/lib.rs:sanitize_required_version'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:717` (`sanitize_required_version`, lines 708-728; cap at `REQUIRED_VERSION_MAX_LEN`, line 213)

**What**: When a `required_version` value exceeds 64 characters, `sanitize_required_version` keeps the first 64 characters and returns them as the value. No ellipsis or other marker is added. The `stack_detail` the operator sees (`Terraform <value>`) therefore looks like a complete constraint but can be cut mid-token: `!= 1.5.3` becomes `!= 1.5`, and `< 2.0.0` becomes `< 2`. The result is a constraint that differs from the one in the manifest, with nothing on screen to say so. The truncation is only reported through a `warn` log.

The function's own doc comment rejects this approach for control characters: "Stripping would silently splice the attacker-controlled tail onto the legitimate prefix; dropping surfaces the field as missing." The length cap applies the opposite policy to the same untrusted value. The test `extract_required_version_caps_overlong_value` pins the truncating behaviour. The function also calls `value.chars().count()` twice (lines 717 and 721).

**Why it matters**: SEC-11 asks for untrusted input to be validated at the boundary, and this code instead alters it in place. `ops about` runs in checkouts the operator did not audit, so an over-cap value is either abuse or a malformed manifest. Showing a plausible but wrong version constraint misleads more than showing nothing. The renderer already truncates to terminal width with a visible ellipsis (`ops_about::text_util::truncate_to_width`), so cutting the value here adds no protection, only a silent change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An over-cap required_version is either dropped (returns None with the existing warn) or rendered with a visible truncation marker, never as an unmarked prefix
- [ ] #2 The REQUIRED_VERSION_MAX_LEN and sanitize_required_version docs state the chosen policy consistently with the control-character drop policy
- [ ] #3 extract_required_version_caps_overlong_value is updated to assert the new behaviour, and the char count is computed once
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.20 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
