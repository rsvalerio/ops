---
id: TASK-1055
title: >-
  PERF-3: text::manifest_max_bytes re-reads OPS_MANIFEST_MAX_BYTES env var on
  every read_capped_to_string call
status: Done
assignee: []
created_date: '2026-05-07 21:03'
updated_date: '2026-05-08 04:18'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:20-26`

**What**: `manifest_max_bytes()` calls `std::env::var(MANIFEST_MAX_BYTES_ENV)` on every invocation, and the function is called from `read_capped_to_string` on every manifest read (`Cargo.toml`, `go.mod`, `gradle.properties`, `requirements.txt`, ...). Stack auto-detection plus the about/identity providers can hit this dozens of times per `ops` invocation. `std::env::var` on Linux/macOS takes a global env-table lock (`environ`), so under any contended workload this both serialises the readers and pays a syscall-style cost per call.

**Comparison**: This is exactly the shape that was fixed for the runner's identical knob in TASK-0542 (`OPS_OUTPUT_BYTE_CAP` cached behind a `OnceLock<usize>` after the first lookup, with a one-shot warn on bad values per TASK-0840). `manifest_max_bytes` ships the older per-call `std::env::var` shape even though the env value is process-global and constant for a run.

**Why it matters**: per-call `std::env::var` on a 32-way parallel detection probe (`Stack::detect` walks ancestors + `for_each_trimmed_line` reads several manifests) takes the env lock for every probe. It also means a misconfigured `OPS_MANIFEST_MAX_BYTES=foo` falls back silently with no operator feedback (TASK-0840's parallel issue, never closed for this knob).

**Severity rationale**: Low — manifest reads are not the dominant per-spawn hot path the way capture buffers were, but the pattern is identical to a known-fixed PERF-3 elsewhere in the workspace.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cache the resolved cap behind a OnceLock<u64> mirroring crates/runner/src/command/results.rs::output_byte_cap,Factor out a pure parser parse_manifest_max_bytes(Option<&str>) -> (u64, Option<String>) so the warn-on-bad-value path is unit-testable without poking the OnceLock,Emit a one-shot tracing::warn! on unparseable / zero values inside the OnceLock initialiser,Update the doc on manifest_max_bytes to reflect the cache-once semantics (existing test that overrides via env will need to spawn a subprocess or run before the first read)
<!-- AC:END -->
