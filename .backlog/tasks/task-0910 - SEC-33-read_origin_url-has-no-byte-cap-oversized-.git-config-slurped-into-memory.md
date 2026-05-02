---
id: TASK-0910
title: >-
  SEC-33: read_origin_url has no byte cap; oversized .git/config slurped into
  memory
status: Done
assignee: []
created_date: '2026-05-02 10:11'
updated_date: '2026-05-02 14:52'
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
- [x] #1 read_origin_url enforces a documented byte cap via File::open + Read::take, returning None with tracing::warn! when exceeded
- [x] #2 Test with a synthetic oversized config file proves the helper bails without reading past the cap
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
read_origin_url now caps the read at MAX_GIT_CONFIG_BYTES (4 MiB) via File::open + Read::take + read_to_string; oversized configs return None with a tracing::warn! and are never parsed. Mirrors the ops_about::manifest_io posture for project manifests. Added read_origin_url_bails_on_oversized_config test that pads a real header up past the cap and asserts None.
<!-- SECTION:NOTES:END -->
