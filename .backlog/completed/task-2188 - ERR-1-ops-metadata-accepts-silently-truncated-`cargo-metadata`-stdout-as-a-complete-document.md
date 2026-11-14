---
id: TASK-2188
title: 'ERR-1: ops-metadata accepts silently-truncated `cargo metadata` stdout as a complete document'
status: Done
assignee: []
created_date: '2026-09-08 07:14'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - error-handling
dependencies: []
parent_task_id: 'TASK-2234'
modified_files:
  - extensions-rust/metadata/src/ingestor.rs
  - extensions-rust/metadata/src/lib.rs
priority: high
ordinal: 101000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:53`, `extensions-rust/metadata/src/lib.rs:429`

**What**: `run_cargo_metadata` goes through `ops_core::subprocess::run_cargo` →
`run_with_timeout` → `read_capped`, which bounds each stream at
`output_byte_cap()` (`DEFAULT_OUTPUT_BYTE_CAP` = **4 MiB**, override
`OPS_OUTPUT_BYTE_CAP`). Truncation is **not** an error: `collect_drain`
(`crates/core/src/subprocess/drain.rs:134-150`) returns `Ok(buf)` with the
trailing bytes discarded and only a `tracing::warn!` breadcrumb.

`ops-metadata` treats that buffer as authoritative on both paths:

- `MetadataIngestor::collect` calls `check_metadata_output(&output)` — which
  only inspects `output.status` — and then `dir.write_atomic(METADATA_JSON,
  &output.stdout)`. A truncated JSON document is staged and handed to
  `read_json_auto`.
- `provide_via_cargo_metadata` (`lib.rs:437`) calls
  `serde_json::from_slice(&output.stdout)` on the same possibly-truncated
  buffer.

Neither call site inspects the truncation signal, so the failure surfaces as a
DuckDB `"metadata_raw create"` parse error or `"parsing cargo metadata stdout"`
serde error with no mention of `OPS_OUTPUT_BYTE_CAP` — or, worse, as a document
that `read_json_auto` accepts in a degraded shape.

**Why it matters**: the crate's own `METADATA_MAX_BYTES_DEFAULT` doc
(`lib.rs:61-73`) states "10+ MiB cargo-metadata output is possible" and sets a
**64 MiB** cap on that premise. That cap is unreachable: the upstream subprocess
cap fires at 4 MiB first, so on any workspace whose `cargo metadata` output
exceeds 4 MiB the SEC-33 machinery in this crate never runs and the ingest
either fails with a misattributed parse error or records incomplete workspace
data as ground truth (`data_sources.checksum` then certifies the truncated
bytes). Measured on this repo `cargo metadata --format-version 1 --locked` is
1.8 MB — under the cap today, but well within a factor of 3 of it.

<!-- scan confidence: verified — read_capped/collect_drain semantics confirmed at crates/core/src/subprocess/drain.rs:34-150 -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_cargo_metadata (or its callers) detects that stdout was capped and fails with an error naming OPS_OUTPUT_BYTE_CAP and the observed/kept byte counts, instead of parsing a truncated document
- [ ] #2 MetadataIngestor::collect does not stage a truncated metadata.json, and provide_via_cargo_metadata does not serde-parse a truncated buffer
- [ ] #3 The relationship between OPS_OUTPUT_BYTE_CAP (4 MiB) and OPS_METADATA_MAX_BYTES (64 MiB) is reconciled: either the metadata path raises the subprocess cap to its own ceiling, or METADATA_MAX_BYTES_DEFAULT's doc stops claiming a 64 MiB budget it cannot reach
- [ ] #4 A test drives a cargo-metadata stdout larger than the resolved output cap and asserts the error names the cap and its env var
<!-- AC:END -->
