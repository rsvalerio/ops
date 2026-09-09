---
id: TASK-2120
title: 'PERF-2: checksum_reader double-buffers - a 64 KiB BufReader wrapping reads that are themselves 64 KiB'
status: To Do
assignee: []
created_date: '2026-09-08 06:54'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - performance
dependencies: []
parent_task_id: 'TASK-2244'
modified_files:
  - extensions/duckdb/src/sql/ingest/dir.rs
priority: low
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest/dir.rs:791`

**What**: the streaming SHA-256 core behind `IngestDir::checksum` builds both a
`BufReader` and a read buffer at the same size:

```rust
let mut reader = BufReader::with_capacity(64 * 1024, source);
let mut hasher = Sha256::new();
let mut buf = vec![0u8; 64 * 1024];
loop { let n = reader.read(&mut buf)?; ... }
```

`BufReader::read` bypasses its own buffer entirely when the caller's slice is
at least as large as the internal buffer (it delegates straight to the inner
reader), so the `BufReader` never buffers anything here. Its only effect is a
second 64 KiB heap allocation per checksum call, plus a layer of indirection on
every read.

The doc comment above the function justifies the 64 KiB chunking for
multi-megabyte ingests (coverage, tokei) — that reasoning applies to `buf`, not
to the wrapper.

**Why it matters**: 64 KiB allocated and freed per checksum for no effect, on
the ingest path where the comment says large files are expected. Removing the
`BufReader` (reading directly from the `File` into `buf`) is strictly simpler
and preserves the streaming contract the two `checksum_*` tests pin.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 checksum_reader reads directly into the 64 KiB buffer without an equally-sized BufReader wrapper, or the wrapper's role is justified in a comment
- [ ] #2 checksum_streaming_matches_in_memory_for_large_input and checksum_is_deterministic still pass unchanged
<!-- AC:END -->
