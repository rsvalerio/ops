---
id: TASK-0910
title: >-
  SEC-33: read_origin_url has no byte cap; oversized .git/config slurped into
  memory
status: Triage
assignee: []
created_date: '2026-05-02 10:11'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:15`

**What**: read_origin_url calls std::fs::read_to_string on the git config path with no size cap. A multi-GB or symlink-to-/dev/zero .git/config will be loaded into a single allocation. TASK-0831 SEC-33 covers about/manifest_io readers but does not cover this path.

**Why it matters**: An adversarial repository (cloned for inspection) can OOM or stall the CLI through an oversized .git/config when the user runs ops in or against the clone.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_origin_url enforces a documented byte cap via File::open + Read::take, returning None with tracing::warn! when exceeded
- [ ] #2 Test with a synthetic oversized config file proves the helper bails without reading past the cap
<!-- AC:END -->
