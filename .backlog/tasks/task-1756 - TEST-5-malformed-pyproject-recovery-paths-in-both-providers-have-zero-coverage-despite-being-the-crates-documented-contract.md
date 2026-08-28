---
id: TASK-1756
title: >-
  TEST-5: malformed-pyproject recovery paths in both providers have zero
  coverage despite being the crate's documented contract
status: To Do
assignee:
  - TASK-1992
created_date: '2026-08-27 11:18'
updated_date: '2026-08-28 14:11'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
  - extensions-python/about/src/units.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:188-203` (`parse_pyproject` error arm) and `extensions-python/about/src/units.rs:60-73` (`read_workspace_members` error arm)

**What**: The crate-level doc comment (`lib.rs:8-10`) states the contract explicitly:

> Parse and read errors fall back to defaults; non-NotFound read errors and parse errors are reported via `tracing` (`debug!` / `warn!`) so a malformed manifest does not silently look like a missing one (TASK-0394).

No test in either module ever feeds a syntactically invalid `pyproject.toml` to a provider. All 16 identity tests and all 6 units tests write well-formed TOML. Uncovered behaviour:

- `parse_pyproject` → `tracing::warn!(path, error, recovery = "default-identity", ...)` then `None`, so `unwrap_or_default()` at `lib.rs:85` yields an empty `Pyproject` and the identity falls back to the directory name.
- `read_workspace_members` → `tracing::warn!(path, error, ...)` then `Vec::new()`, so `collect_units` returns no units.

Neither the fallback value nor the warn record (its presence, its `path` field, or the `recovery = "default-identity"` key added by ERR-7 / TASK-0974) is asserted anywhere.

**Why it matters**: the warn is the *only* thing distinguishing "no manifest" from "broken manifest" — the whole point of TASK-0394 and TASK-0974. Both are silently deletable today: removing the `tracing::warn!` calls, or flipping the recovery from `None`/`Vec::new()` to a panic-free but wrong value, leaves the suite green. The crate ships a `WarnCounter`-style harness for exactly this in `ops_about::test_support`, so the seam exists and is simply unused here.

Also uncovered on the same axis: `units.rs` `collect_units` never sees a *member* `pyproject.toml` that fails to parse, so the `parse_package_metadata` warn-and-default path (`units.rs:98-108`) is likewise untested from this crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test writes a syntactically invalid pyproject.toml and asserts PythonIdentityProvider::provide still succeeds with the directory-name fallback identity (no version, no stack_detail)
- [ ] #2 A test writes a syntactically invalid root pyproject.toml and asserts collect_units returns an empty Vec
- [ ] #3 A test writes a valid root manifest with a workspace member whose own pyproject.toml is invalid, and asserts the unit still appears with the format_unit_name directory fallback
- [ ] #4 At least one of the above asserts the tracing warn is actually emitted (via ops_about::test_support), including the manifest path and the recovery field
<!-- AC:END -->
