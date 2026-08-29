---
id: TASK-2039
title: 'SEC: anchor duckdb ingest staging to a verified directory handle'
status: Triage
assignee: []
created_date: '2026-08-29 06:35'
labels:
  - security
  - duckdb
dependencies: []
type: enhancement
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
CodeRabbit (PR #40) flagged that `create_ingest_dir` verifies the ingest dir through an open handle (lstat + dev/ino match + fchmod) and then **drops** that handle. `provide_via_ingestor` afterwards passes the plain `&Path` to `Ingestor::collect` and `Ingestor::load`, which reopen `data_dir` by path. Between the verification and each staged write there is a TOCTOU window: a co-tenant who can create names in the parent directory can swap the verified directory for a symlink, and staged JSON the database later trusts on load lands wherever they point.

The rejection half was closed in the review pass (a pre-existing symlink / reparse point is now refused on both the Unix and non-Unix branches via the shared `reject_untrusted_ingest_dir`, with tests). What remains is the handle-vs-path gap, which is not a review-pass-sized change: the `Ingestor` trait signature takes `&Path` for both `collect` and `load`, every ingestor implements it, and `sidecar.rs` joins onto `data_dir` by path as well.

Options to weigh:
- Thread a verified `Dir`-like handle (e.g. `cap-std`, or `openat`-based `*at` syscalls) through the `Ingestor` trait and `sidecar.rs`.
- Or make the staging *parent* private (0o700) so no untrusted principal can create or swap names inside it, which removes the swap capability without changing the trait.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The TOCTOU window between ingest-dir verification and staged writes is closed, either by threading a verified directory handle through Ingestor::collect / Ingestor::load and sidecar.rs, or by making the staging parent unwritable to other local principals
- [ ] #2 A test demonstrates that a symlink swapped in after verification cannot redirect a staged write
- [ ] #3 The chosen approach is documented at create_ingest_dir alongside the existing SEC-25 notes
<!-- AC:END -->
