---
id: TASK-1736
title: >-
  DUP-1: identical write() test helper duplicated verbatim in lib.rs and
  units.rs
status: Triage
assignee: []
created_date: '2026-08-27 11:12'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-node/about/src/lib.rs
  - extensions-node/about/src/units.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/lib.rs:122-127` and `extensions-node/about/src/units.rs:359-364`

**What**: Both `#[cfg(test)]` modules define byte-for-byte the same helper:

```rust
fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}
```

Six identical lines, two copies, one crate — over the DUP-1 5-line threshold.

**Why it matters**: This crate already has an established shared home for exactly this kind of helper. `ops_about::test_support` exists specifically because "each provider grew its own ... test that re-proved the same property" (DUP-3 / TASK-0985), it is already a dev-dependency with the `test-support` feature enabled in `Cargo.toml`, and both of these test modules already call into it for `assert_debug_escapes_control_chars` (`package_json.rs:203`, `units.rs:373`). The duplication is therefore a drift surface with a ready fix: a future tightening — propagating the `create_dir_all` error, switching to `expect` with a setup message (TEST-22), handling a path with no parent — has to land twice or the copies diverge silently.

**Notes (out of scope for this task, informational)**: the same helper also appears verbatim in `extensions-terraform/about/src/lib.rs` and `extensions-python/about/src/units.rs`. Those are other crates and other reviewers' scope; hoisting to `ops_about::test_support` would retire all four copies at once, so prefer that landing site over a crate-local `mod test_util`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The write() test helper is defined once and both lib.rs and units.rs test modules use that single definition
- [ ] #2 Preferred landing site is ops_about::test_support (already a test-support dev-dependency of this crate) so the sibling about crates can adopt it too
- [ ] #3 cargo test -p ops-about-node passes unchanged
<!-- AC:END -->
