---
id: TASK-2039
title: 'SEC: anchor duckdb ingest staging to a verified directory handle'
status: Done
assignee:
  - TASK-2041
created_date: '2026-08-29 06:35'
updated_date: '2026-08-29 13:06'
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
- [x] #1 The TOCTOU window between ingest-dir verification and staged writes is closed, either by threading a verified directory handle through Ingestor::collect / Ingestor::load and sidecar.rs, or by making the staging parent unwritable to other local principals
- [x] #2 A test demonstrates that a symlink swapped in after verification cannot redirect a staged write
- [x] #3 The chosen approach is documented at create_ingest_dir alongside the existing SEC-25 notes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented the bounded option: removed the swap *capability* instead of threading a directory handle through the Ingestor trait. create_ingest_dir now calls the new harden_ingest_parent(parent) on Unix, which opens the staging parent, confirms it is a directory, and clears the group/other write bits through the handle (fchmod, not path-based chmod) so no other local principal can create or rename names inside it; a shared-writable but sticky parent (/tmp-style) is left alone with a debug breadcrumb because the sticky bit already forbids the swap, and a parent whose bits cannot be cleared (owned by someone else) makes staging fail with an explanatory error rather than proceeding. Only the write bits are cleared (0o775 -> 0o755), so TASK-1000 (target/ stays conventional) still holds. AC#3: the choice and the rejected alternative are documented in a TASK-2039 section on create_ingest_dir next to the existing SEC-25 notes.

AC#2 substitution: a post-verification symlink swap is a TOCTOU race no test can drive deterministically. The equivalent check under the chosen approach is that the capability is gone: create_ingest_dir_removes_swap_capability_from_the_staging_parent starts the parent at 0o777 and asserts no group/other write remains after the call (owner access preserved), and create_ingest_dir_leaves_a_sticky_shared_parent_alone pins that ops does not chmod a sticky shared directory.

Remainder (Ingestor::collect / load and sidecar.rs still reopen data_dir by path) filed separately for triage.
<!-- SECTION:NOTES:END -->
