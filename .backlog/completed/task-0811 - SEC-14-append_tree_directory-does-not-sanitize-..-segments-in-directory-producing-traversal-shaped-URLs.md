---
id: TASK-0811
title: >-
  SEC-14: append_tree_directory does not sanitize .. segments in directory,
  producing traversal-shaped URLs
status: Done
assignee:
  - TASK-0827
created_date: '2026-05-01 06:02'
updated_date: '2026-05-01 10:50'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:223-234`

**What**: An adversarial package.json with repository.directory of ../../../../etc/passwd produces https://github.com/o/r/tree/HEAD/../../../../etc/passwd. The function only strips ./ prefix, normalises backslash, and trims slashes — .. components inside the path are passed through unchanged.

**Why it matters**: The output URL is rendered into the About card and likely emitted into HTML/markdown contexts. SEC-14 path-traversal-shape applies to URL construction over user input.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Reject or strip path components equal to .. after normalising slashes; document the policy in the function doc
- [x] #2 Tests for directory ../foo, directory a/../b, and directory /absolute (already begins with /)
- [x] #3 Existing valid-monorepo tests stay green
<!-- AC:END -->
