---
id: TASK-1000
title: >-
  SEC-25: create_ingest_dir DirBuilder with mode 0o700 stamps every intermediate
  parent restrictively
status: Triage
assignee: []
created_date: '2026-05-04 22:02'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:102-117`

**What**: `create_ingest_dir` builds the data dir with `DirBuilder::new().recursive(true).mode(0o700).create(data_dir)`. With `recursive(true)`, that 0o700 mode is also applied to every intermediate directory created during the call, not only the final ingest dir. When `target/ops/data.duckdb.ingest` is created in a fresh workspace, the helper will create `target/ops` (and possibly `target/`) with mode 0o700 — the same overly-restrictive permissions that were chosen specifically for the ingest dir.

**Why it matters**:
- `target/` is canonically 0o755 in cargo / build-system convention; tightening it to 0o700 breaks downstream tooling that expects `target/release/binary` to be readable by other users in CI runners or shared dev machines (matrix testing rigs, build caches mounted into containers as a non-root user, package-build chroots).
- The TASK-0787 contract said "the ingest dir holds workspace-root sidecars and JSON staging files that the database trusts on load" — only the ingest dir needs hardening. The hardening should be scoped to the leaf, not the path.
- Conversely, when an intermediate dir already exists at 0o755, the `recursive(true)` builder leaves it alone — so a fresh workspace and a workspace where `target/` already exists end up with different permissions trees. The asymmetry is silent.

**Reproduction sketch**: in a test that calls `create_ingest_dir(tmp.join("a/b/c.ingest"))`, assert `metadata(tmp.join("a")).permissions().mode() & 0o777 == 0o700`. The current code passes that assertion; the fix should be to create only the final segment with 0o700 (e.g., create_dir_all the parent first at default umask, then DirBuilder the leaf at 0o700).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 create_ingest_dir creates intermediate parents with the platform-default umask and only the final ingest-dir leaf with mode 0o700.
- [ ] #2 Test asserts that an intermediate parent (e.g., target/ops) is NOT 0o700 after a fresh create when umask defaults to 022.
- [ ] #3 Existing TASK-0787 invariant on the ingest leaf still holds (fresh + pre-existing leaf both end at 0o700).
<!-- AC:END -->
