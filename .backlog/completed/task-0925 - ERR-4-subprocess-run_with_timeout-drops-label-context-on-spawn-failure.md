---
id: TASK-0925
title: 'ERR-4: subprocess::run_with_timeout drops label context on spawn failure'
status: Done
assignee: []
created_date: '2026-05-02 15:11'
updated_date: '2026-05-02 16:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:190`

**What**: `cmd.spawn()?` on line 190 propagates the raw `io::Error` via `From<io::Error> for RunError`. Unlike `RunError::Timeout`, the resulting `RunError::Io` carries no `label` or program name, so a spawn failure (e.g. `cargo` not on PATH) renders only as `No such file or directory (os error 2)` with no indication of which subprocess failed. The function already takes `label: &str` and uses it in the timeout path; the spawn path silently drops it.

**Why it matters**: Operators investigating a failed `cargo metadata` / `cargo update` invocation see a context-free errno message and must guess which subprocess emitted it. Affects every cargo-invoking data provider in the Rust extensions that surface `RunError::Io` to logs/UI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Spawn failure is wrapped with label (and ideally program name) so the rendered error names the failing operation, e.g. "cargo metadata: failed to spawn cargo: No such file or directory"
- [ ] #2 RunError::Io either gains a label field or the spawn path constructs an error that includes the label while preserving source() to the original io::Error
- [ ] #3 Existing RunError::Timeout formatting unchanged; new test asserts spawn-failure error string contains the caller-supplied label
- [ ] #4 ops verify and ops qa pass with no new clippy warnings
<!-- AC:END -->
