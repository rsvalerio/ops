---
id: TASK-2433
title: 'SEC-14: read_optional_text follows a symlinked manifest out of the workspace root'
status: Triage
assignee: []
created_date: '2026-10-04 15:51'
labels:
  - code-review-rust
  - SEC
dependencies: []
modified_files:
  - extensions/about/src/manifest_io.rs
  - extensions/about/src/manifest_cache.rs
  - extensions-go/about/src/go_mod.rs
  - extensions-go/about/src/go_work.rs
  - extensions-node/about/src/units.rs
  - extensions-java/about/src/maven/pom.rs
  - extensions-terraform/about/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/manifest_io.rs:read_optional_text` (callers: `extensions/about/src/manifest_cache.rs`, `extensions-go/about/src/go_mod.rs`, `extensions-go/about/src/go_work.rs`, `extensions-node/about/src/units.rs`, `extensions-java/about/src/maven/pom.rs`, `extensions-terraform/about/src/lib.rs`)

**What**: `read_optional_text` opens its path with a symlink-following open. TASK-2398 closed the workspace-member path (members now go through a containment check plus `ops_core::text::read_capped_to_string`), and TASK-2399 made the helper reject non-regular files without blocking, but a root-level manifest that is itself a symlink (`package.json -> /some/other/file`, `go.mod`, `pom.xml`, `versions.tf`) is still read and its fields surfaced in `ops about` output. `ops_core::text::read_capped_to_string` already refuses a symlink at every component; switching needs each caller to hand in a canonical root (tests that pass a raw `tempfile::tempdir()` fail on macOS where `/var` is a symlink), which is why it was not done inside TASK-2421.

**Why it matters**: AGENTS.md states `ops` must not follow symlinks out of the workspace. Impact is bounded to manifest fields from a file the user can already read, hence low.

**Origin**: discovered during TASK-2421 while fixing TASK-2398 / TASK-2399.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A manifest read through read_optional_text that resolves outside the workspace root via a symlink is refused with a warn
- [ ] #2 Regression test with a root-level manifest symlinked outside the root
<!-- AC:END -->
