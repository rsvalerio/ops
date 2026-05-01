---
id: TASK-0296
title: 'ASYNC-6: cargo subprocess callsites in extensions-rust lack timeouts'
status: Done
assignee:
  - TASK-0299
created_date: '2026-04-23 16:54'
updated_date: '2026-04-23 18:21'
labels:
  - rust-code-review
  - async
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: Multiple `std::process::Command::new("cargo")...output()` callsites run long-lived / network-touching cargo invocations with no timeout wrapper:

- `extensions-rust/metadata/src/lib.rs:26` — `cargo metadata`
- `extensions-rust/test-coverage/src/lib.rs:87` — `cargo llvm-cov`
- `extensions-rust/cargo-update/src/lib.rs:52` — `cargo update --dry-run`
- `extensions-rust/deps/src/parse.rs:15` — `cargo upgrade --dry-run`
- `extensions-rust/deps/src/parse.rs:122` — `cargo deny check`

All use blocking `std::process::Command::output()` with no bounded wait.

**Why it matters**: ASYNC-6 / SEC-33 require bounded resource consumption on external calls. Network-dependent cargo operations (registry fetch, advisory DB update, dependency resolution) can hang indefinitely on slow or misconfigured networks, and `ops about` / `ops deps` then appear frozen with no way to diagnose short of Ctrl+C.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each of the 5 callsites wraps the subprocess with a bounded timeout (or moves to an async runtime with tokio::time::timeout)
- [x] #2 Timeout expiration surfaces an explicit error to the caller (not a silent hang / zero output)
- [x] #3 Timeout default is configurable (env or config) and documented; sensible default chosen per operation
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added ops_core::subprocess::{run_with_timeout, default_timeout, RunError, TimeoutError}. Per-op defaults: cargo metadata=120s, cargo llvm-cov=900s, cargo update=120s, cargo upgrade=180s, cargo deny=240s. OPS_SUBPROCESS_TIMEOUT_SECS env var overrides any default. Timeout kills the child and surfaces RunError::Timeout (distinct from IO errors). All 5 callsites migrated.
<!-- SECTION:NOTES:END -->
