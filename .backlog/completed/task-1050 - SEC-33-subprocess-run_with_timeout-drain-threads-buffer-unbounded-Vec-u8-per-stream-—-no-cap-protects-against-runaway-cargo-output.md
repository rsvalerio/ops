---
id: TASK-1050
title: >-
  SEC-33: subprocess::run_with_timeout drain threads buffer unbounded Vec<u8>
  per stream — no cap protects against runaway cargo output
status: Done
assignee: []
created_date: '2026-05-07 21:02'
updated_date: '2026-05-07 23:21'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:244-257`

**What**: `run_with_timeout` spawns two drain threads that each call `read_to_end(&mut buf)` on the child's stdout/stderr `Vec<u8>`, with no upper bound on buffer growth. `cargo metadata`, `cargo update`, `cargo deny`, and `cargo llvm-cov` callers in extensions-rust all flow through this path. A misbehaving child (or a workspace large enough that `cargo metadata` JSON balloons past expectations) can grow the in-memory buffer without limit before `read_to_end` returns.

**Comparison**: the runner's parallel exec path went through exactly this fix in TASK-0764 / TASK-0515 — `command::exec::read_capped` plus `OPS_OUTPUT_BYTE_CAP` keep peak RSS bounded per stream. `subprocess::run_with_timeout` ships the older unbounded shape even though it serves the same role for the cargo-invoking data providers (which are the noisiest sources of subprocess stdout in the project: `cargo metadata --format-version=1` on a large workspace is regularly tens of MiB).

**Why it matters**: SEC-33 explicitly calls out "Bound resource consumption on untrusted input … timeouts on operations processing external data — prevents DoS via unbounded allocations." Trust model: `ops` runs in user-controlled CWD, and a malicious workspace can trivially craft a `Cargo.toml` / `build.rs` that emits arbitrarily large stdout. The wall-clock timeout (`MAX_TIMEOUT_SECS = 3600`) does not bound memory, only wall time, so the helper happily allocates 1 GiB+ of stdout buffer over a 30 s window.

**Severity rationale**: Low because the existing callers (cargo subcommands the user already trusts) rarely emit pathological output in practice; the structural gap is what's load-bearing. Aligns with the SEC-33 sweep that produced TASK-0926 (Cargo.toml readers) and TASK-0932 (text::for_each_trimmed_line).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Apply a per-stream byte cap (mirroring command::exec::read_capped) inside the drain threads,Surface dropped byte counts via the existing tracing breadcrumbs so callers can detect truncation,Add a regression test that pumps a >cap stream and asserts captured bytes are bounded,Document the cap and any env override (e.g. OPS_SUBPROCESS_OUTPUT_BYTE_CAP) on run_with_timeout
<!-- AC:END -->
