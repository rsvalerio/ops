---
id: TASK-1763
title: >-
  DUP-1: the write-manifest / build-context / deserialize-identity preamble is
  repeated in 16 lib.rs tests
status: Triage
assignee: []
created_date: '2026-08-27 11:20'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:443-896` (test module)

**What**: Sixteen of the module's tests open with the same five-statement block, differing only in the TOML body:

```rust
let dir = tempfile::tempdir().unwrap();
std::fs::write(dir.path().join("pyproject.toml"), r#"..."#).unwrap();
let provider = PythonIdentityProvider;
let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
let id: ProjectIdentity = serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
```

Counted with `grep -c "Context::test_context"` → 16, and `grep -c "tempfile::tempdir().unwrap()"` → 16. Tests at lines 443, 466, 538, 570, 588, 612, 634, 660, 689, 721, 750, 780, 808, 825, 849, 874. A single `fn identity_from(toml: &str) -> ProjectIdentity` helper (with a variant that also takes extra files to write, for the `uv.lock` and `.git/config` cases at lines 570 and 874) collapses each test to its manifest body plus its assertion.

`units.rs` in the same crate already extracts a `write` helper (`units.rs:143-148`) for the smaller version of this, so the pattern is established — `lib.rs` just never adopted it.

**Why it matters**: roughly 80 lines of the test module are boilerplate, which buries the one line per test that actually states the contract. It is also a change-amplifier: `provide`'s signature or the `Context` construction shape moving means editing sixteen sites, and the copy-paste has already produced inconsistency — two tests build the manifest with concatenated `\n` string literals (lines 471, 694) while the rest use raw strings.

**Note**: TASK-1736 filed the narrower "identical `write()` helper duplicated across lib.rs and units.rs" version of this against another about crate; this is the same class in the Python crate, at larger scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test helper takes a pyproject.toml body and returns the deserialized ProjectIdentity, and every applicable test uses it
- [ ] #2 The two tests needing extra files (uv.lock, .git/config) are covered by a helper variant rather than reverting to the inline form
- [ ] #3 Manifest bodies use one consistent literal style
- [ ] #4 The test suite passes with no change in what is asserted
<!-- AC:END -->
