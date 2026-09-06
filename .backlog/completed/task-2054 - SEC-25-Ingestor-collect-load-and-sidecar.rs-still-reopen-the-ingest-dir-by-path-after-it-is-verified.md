---
id: TASK-2054
title: >-
  SEC-25: Ingestor::collect / load and sidecar.rs still reopen the ingest dir by
  path after it is verified
status: Done
assignee:
  - TASK-2063
created_date: '2026-08-29 13:07'
updated_date: '2026-08-29 18:08'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/duckdb/src/ingestor.rs
  - extensions/duckdb/src/sql/ingest/dir.rs
  - extensions/duckdb/src/sql/ingest/orchestrator.rs
  - extensions/duckdb/src/sql/ingest/sidecar.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs`, `extensions/duckdb/src/sql/ingest/dir.rs`, `extensions/duckdb/src/sql/ingest/orchestrator.rs`, `extensions/duckdb/src/sql/ingest/sidecar.rs`

**What**: TASK-2039 closed the verify-then-write TOCTOU window by removing the *capability* to swap the ingest dir — `create_ingest_dir` now calls `harden_ingest_parent`, which clears group/other write on the staging parent so no other local principal can create or rename names in it. The structural half of the finding is untouched: `create_ingest_dir` still verifies through an open handle and drops it, `provide_via_ingestor` still passes a plain `&Path` to `Ingestor::collect` / `Ingestor::load`, and `sidecar.rs` still joins onto `data_dir` by path, so every staged write re-resolves the directory by name.

Two cases the parent-hardening does not cover:

- A shared-writable but **sticky** parent (a `/tmp`-style staging area) is deliberately left alone — the sticky bit stops others renaming our name, but the reopen is still by path.
- An attacker running as the **same uid** (a compromised build script, another tool in the same session) is unaffected by any mode.

**Why it matters**: the remaining exposure is small and mode-gated, but the durable fix is structural — thread a verified `Dir`-like handle (`cap-std`, or `openat`-based `*at` syscalls) through the `Ingestor` trait and `sidecar.rs` so staged writes are relative to the descriptor that was verified, never to a name resolved again. That is a trait-signature change across every ingestor and was out of scope for TASK-2039's wave.

**Origin**: discovered during TASK-2041 while fixing TASK-2039.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Ingestor::collect and Ingestor::load receive a verified directory handle (or *at-style anchored writer) instead of a bare &Path, and sidecar.rs stages through the same anchor
- [x] #2 A test shows a staged write cannot be redirected by replacing the ingest dir name after verification, without relying on the parent's mode
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-2063 / SEC-25: added `IngestDir` (extensions/duckdb/src/sql/ingest/dir.rs) — a verified directory descriptor (O_DIRECTORY|O_NOFOLLOW, dev/ino-checked against the lstat taken during hardening) whose write_atomic / open_read / rename / remove_file / checksum are anchored on *at(2) syscalls (openat/renameat/unlinkat). `DataIngestor::collect` and `::load` now take `&IngestDir` instead of `&Path`; `SidecarIngestorConfig::collect_sidecar` / `load_with_sidecar`, the whole of sidecar.rs (write/read/remove), the cleanup rename-to-.done + unlink path, MetadataIngestor and the public `load_coverage` all stage through it. `provide_via_ingestor` opens the anchor once and holds it across collect→load. Path is still handed out (IngestDir::path / entry_path) for DuckDB read_json_auto (path-only API) and provenance labels — reads and labels, never writes; documented on the type. Anchoring is Unix-only, matching the existing platform split; non-Unix keeps the by-name behaviour plus the symlink/reparse rejection. AC#2 covered by dir.rs::staged_write_is_not_redirected_by_swapping_the_ingest_dir_name, which renames the verified dir aside and symlinks an attacker dir over the path with the parent left writable throughout, so it depends on no mode. Extra: entry names are validated as single path components, and a symlinked ingest dir is refused by IngestDir::open.
<!-- SECTION:NOTES:END -->
