---
id: TASK-0852
title: >-
  CL-3: terraform find_required_version fallback scan depends on filesystem
  read_dir ordering
status: Triage
assignee: []
created_date: '2026-05-02 09:17'
labels:
  - code-review-rust
  - complexity
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:83-94`

**What**: When the named candidates miss, the function scans read_dir(root) and returns the first required_version found. read_dir ordering is platform-dependent (ext4 != APFS != Windows), so two operators on the same project can see different rendered Terraform versions when the constraint differs across files.

**Why it matters**: Non-determinism in user-visible output. Less serious than a correctness bug, but the variance is a real footgun if a repo has conflicting required_version constraints.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Sort the directory entries by filename before scanning, or collect all matches and pick a documented winner (most restrictive, alphabetical first, etc.)
- [ ] #2 Add a test that creates two .tf files with different required_version strings and asserts a stable result
<!-- AC:END -->
