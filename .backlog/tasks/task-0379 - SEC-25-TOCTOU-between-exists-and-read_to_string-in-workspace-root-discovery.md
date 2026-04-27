---
id: TASK-0379
title: 'SEC-25: TOCTOU between exists() and read_to_string in workspace-root discovery'
status: Done
assignee:
  - TASK-0416
created_date: '2026-04-26 09:38'
updated_date: '2026-04-27 08:28'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:267`

**What**: find_workspace_root calls cargo_toml.exists() then later fs::read_to_string(&cargo_toml). A concurrent rename or symlink swap can mislead the resolver. More importantly, current.parent() recursion is unbounded and never canonicalizes: a symlink loop can cause loop to walk forever.

**Why it matters**: TOCTOU is low-impact for a CLI but the unbounded ancestor walk via symlinks is a hardening gap.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Canonicalize start before walking ancestors; cap depth (e.g. 64) as defensive bound
- [x] #2 Add a test for symlink loops / very deep paths
<!-- AC:END -->
