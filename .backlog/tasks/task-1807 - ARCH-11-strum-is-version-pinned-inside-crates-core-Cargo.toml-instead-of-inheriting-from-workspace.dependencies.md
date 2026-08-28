---
id: TASK-1807
title: >-
  ARCH-11: strum is version-pinned inside crates/core/Cargo.toml instead of
  inheriting from [workspace.dependencies]
status: To Do
assignee:
  - TASK-1983
created_date: '2026-08-27 11:30'
updated_date: '2026-08-28 14:08'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - crates/core/Cargo.toml
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/Cargo.toml:19`

**What**: Every other third-party dependency in `crates/core/Cargo.toml` inherits its version from the workspace (`serde = { workspace = true }`, `toml = { workspace = true }`, `anyhow = { workspace = true }`, `libc`, `tempfile`, `proptest`, `serial_test`, … — 12 of 14). Two do not:

```toml
config = "0.15"                                   # see the SEC-27 finding — unused, remove
strum = { version = "0.28", features = ["derive"] }
```

`strum` is not present in the root `[workspace.dependencies]` table at all, so its version and feature set live only here. It is a real dependency — `stack/mod.rs` derives `EnumString`, `IntoStaticStr`, `EnumIter`, `VariantNames` from it and `stack/detect.rs` imports `IntoEnumIterator` — so unlike `config` it must be kept, just relocated.

**Why it matters**: ARCH-11 — the workspace's stated policy is single-point version control so a CVE bump or a major-version migration is one edit. Today `ops-core` is the only consumer of `strum`, so nothing has drifted yet; the moment a second crate needs `strum` (any new stack-enum or extension registry is the obvious candidate) the natural move is to add another local pin, and the two silently diverge. Centralising now costs one line and removes the failure mode entirely. Note this is a *consistency* finding, not a live drift: `cargo tree` shows exactly one `strum` version in the workspace today.

<!-- scan confidence: verified — root Cargo.toml [workspace.dependencies] contains no strum entry -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 strum (with its derive feature) is declared once in the root [workspace.dependencies] table and crates/core/Cargo.toml inherits it with { workspace = true }
- [ ] #2 No third-party dependency in crates/core/Cargo.toml carries an inline version string after the change
- [ ] #3 cargo build --all-targets --workspace and cargo clippy --all-targets --workspace -- -D warnings pass, and Cargo.lock shows no version change for strum
<!-- AC:END -->
