---
id: TASK-1540
title: >-
  DUP-4: cargo-metadata JSON fixture re-inlined 12+ times across
  metadata/src/tests.rs and ingestor.rs tests
status: To Do
assignee:
  - TASK-1576
created_date: '2026-05-19 15:24'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/tests.rs:10-78` (plus ~12 re-inlined copies), `extensions-rust/metadata/src/ingestor.rs:208-256, 294-353, 394-430, 478-537`

**What**: The cargo-metadata JSON skeleton (`packages`, `workspace_members`, `workspace_default_members`, `resolve`, `target_directory`, `version`, `workspace_root`, `metadata`) is open-coded as a `serde_json::json!` literal in many test bodies:
- `tests.rs:10-78` is `sample_metadata()` — the proper shared fixture.
- `tests.rs:186-201, 213-228, 236-251, 266-271, 490-496, 501-505, 513-528, 537-552, 562-577, 590-609, 636-653, 669-686, 702-719, 727-744, 752-790, 805-823` re-inline near-identical packaging JSON instead of building on the shared fixture.
- `ingestor.rs:208-256, 294-353, 394-430 (via sample_obj), 478-537` inline another ~30-field JSON each.

Each block restates the same 15-20 boilerplate fields (`source`, `features`, `manifest_path`, `metadata`, `publish`, `authors`, `categories`, `keywords`, `readme`, `repository`, `homepage`, `documentation`, `edition`, `links`, `default_run`, `rust_version`, `license`, `license_file`, `description`) that the test under question does not exercise.

**Why it matters**: A schema-shape change in cargo metadata (or in this crate's parsing assumptions) requires updating ~15 inline JSON blobs across two files. Reviewers cannot tell at a glance which fields each test actually cares about — the variation is buried in a wall of constant boilerplate. A `fn sample_pkg(name, version, id) -> Value` + `fn sample_metadata_with_pkgs(pkgs) -> Value` test helper (or a builder) would shrink each test body to its semantically-meaningful overrides and make regression coverage scope visible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 metadata/src/tests.rs and ingestor.rs share a single test-helper module (e.g. sample_pkg / sample_metadata_with) that constructs cargo-metadata JSON fixtures
- [ ] #2 Each individual test body retains only the fields it specifically exercises; boilerplate fields (license, repository, homepage, etc.) live in the helper
- [ ] #3 All existing tests still pass without weakening their assertions
<!-- AC:END -->
