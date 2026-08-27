---
id: TASK-1833
title: >-
  DUP-5: config-checkers hand-rolls is_root_euid with unsafe + a libc dev-dep
  the workspace already avoids
status: Triage
assignee: []
created_date: '2026-08-27 15:21'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions/config-checkers/src/lib.rs
  - extensions/config-checkers/Cargo.toml
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:424-452` (`unreadable_file_is_reported_as_failed`) and `extensions/config-checkers/Cargo.toml:21-22`

**What**: the crate reimplements a root-EUID probe that already exists in the workspace as a documented, purpose-built helper — and pays for the reimplementation with an `unsafe` block and a dependency:

```rust
// extensions/config-checkers/src/lib.rs:434-435
// SAFETY: `libc::geteuid` is always safe to call.
let is_root = unsafe { libc::geteuid() } == 0;
```

```toml
# extensions/config-checkers/Cargo.toml
[target.'cfg(unix)'.dev-dependencies]
libc = { workspace = true }
```

`ops_core::test_utils::is_root_euid()` (`crates/core/src/test_utils.rs:538-567`) is the canonical version. It was added for exactly this scenario — its doc comment says so:

> TEST-19 (TASK-1033): true when the current effective UID is 0 on Unix. Tests that rely on DAC permission denial (`chmod 0o000` + assert read fails) silently invert their assertion when run as root [...] Callers should `if is_root_euid() { return; }` at the top of the test

It also **deliberately avoids the dependency this crate just added**, in a comment addressed to precisely this decision:

> Avoid pulling in a libc dep just for one syscall: declare the FFI signature locally.

Two existing callers already use it (`crates/core/src/stack/mod.rs:280`, `crates/core/src/text.rs:662`), and it handles the `#[cfg(not(unix))]` arm that the local copy does not. The helper is behind the `test-support` feature, and taking it as a dev-dependency is the established workspace idiom — 19 member crates already write `ops-core = { workspace = true, features = ["test-support"] }` (e.g. `crates/theme/Cargo.toml:15`, `extensions-rust/deps/Cargo.toml:19`). `ops-core` is already a normal dependency of this crate.

**Why it matters**: DUP-5 (shared logic belongs in the one shared helper) with a READ-6 consistency angle and a real dependency-hygiene cost. Concretely: (1) `libc` is now in the crate's dev dependency graph for four lines of code the workspace explicitly decided not to take a dependency for; (2) this is the only `unsafe` block in `extensions/config-checkers`, so the crate carries an FFI/`unsafe` audit surface it does not need; (3) the local copy has no non-Unix arm, so the pattern does not port; (4) the guard is placed *after* the assertions' setup rather than at the top of the test as the helper's contract asks, which is the kind of drift a second copy always accumulates.

**Fix shape**: add `ops-core = { workspace = true, features = ["test-support"] }` to `[dev-dependencies]`, replace the block with `if ops_core::test_utils::is_root_euid() { return; }`, and drop the `[target.'cfg(unix)'.dev-dependencies] libc` entry. Keep the inline comment explaining why the guard is mandatory, as the helper's doc asks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 unreadable_file_is_reported_as_failed uses ops_core::test_utils::is_root_euid() instead of a local libc::geteuid call
- [ ] #2 The libc dev-dependency is removed from extensions/config-checkers/Cargo.toml and the crate contains no unsafe blocks
- [ ] #3 ops-core is taken as a dev-dependency with the test-support feature, matching the existing workspace convention
<!-- AC:END -->
