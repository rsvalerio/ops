---
id: TASK-1501
title: >-
  API-3: cargo-toml blanket #[allow(dead_code)] on public impl blocks and
  constants
status: Done
assignee:
  - TASK-1641
created_date: '2026-05-18 18:04'
updated_date: '2026-05-25 16:13'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:108`, `:110`, `:143`; `extensions-rust/cargo-toml/src/types.rs:53`, `:225`, `:349`

**What**: `pub const DESCRIPTION`, `pub const SHORTNAME`, `CargoTomlExtension::with_root`, and entire public `impl` blocks (`impl CargoToml`, `impl PublishSpec`, `impl DepSpec`) are wrapped in `#[allow(dead_code)]`. These items are part of the public API — `dead_code` should not apply. The allow likely dates from when intra-workspace consumers were absent.

**Why it matters**: It silences `dead_code` warnings across whole impl blocks (~150 lines on `impl DepSpec` alone), hiding any *actually dead* method added later; it also signals to readers "these methods might not be needed" which is the opposite of the intent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Audit each #[allow(dead_code)] site; remove from items reachable through pub use or consumed by sibling crates
- [ ] #2 For methods only used in tests, gate behind #[cfg(any(test, feature = ...))] or document why they remain in the public API
- [ ] #3 cargo clippy --all-targets -- -D warnings remains clean
<!-- AC:END -->
