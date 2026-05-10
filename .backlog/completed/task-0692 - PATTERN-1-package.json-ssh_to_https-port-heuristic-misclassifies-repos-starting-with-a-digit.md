---
id: TASK-0692
title: >-
  PATTERN-1: package.json ssh_to_https port heuristic misclassifies repos
  starting with a digit
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:16'
updated_date: '2026-04-30 11:14'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:181-193`

**What**: The scp-form heuristic `path.starts_with(|c: char| c.is_ascii_digit())` to distinguish port (host:22/path) from scp-form (host:owner/repo) misclassifies legitimate scp-form paths whose first segment starts with a digit (e.g. `git@host:42-cool-repo/x.git` → `https://host:42-cool-repo/x` instead of `https://host/42-cool-repo/x`).

**Why it matters**: Fringe but real (numeric repo prefixes exist on GitHub: e.g. 2024-archive); silent URL corruption renders an unreachable repo link in the About card.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Tighten the port heuristic to 'all digits up to the next /' rather than 'first char is digit'
- [x] #2 Test: ssh://git@github.com:42-archive/x.git → https://github.com/42-archive/x
<!-- AC:END -->
