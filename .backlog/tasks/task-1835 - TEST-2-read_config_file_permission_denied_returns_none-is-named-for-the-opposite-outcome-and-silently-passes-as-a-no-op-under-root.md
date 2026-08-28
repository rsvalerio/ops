---
id: TASK-1835
title: >-
  TEST-2: read_config_file_permission_denied_returns_none is named for the
  opposite outcome and silently passes as a no-op under root
status: To Do
assignee:
  - TASK-1983
created_date: '2026-08-27 15:22'
updated_date: '2026-08-28 14:09'
labels:
  - code-review-rust
  - testing
dependencies: []
modified_files:
  - crates/core/src/config/tests/validate_tests.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/tests/validate_tests.rs:315-330`

**What**: Two defects in one test.

1. **The name states the opposite of the assertion.** The test is called `read_config_file_permission_denied_returns_none`, but the body asserts the error case:

```rust
let result = read_config_file(&path);
assert!(result.is_err(), "permission denied should return Err");
```

`read_config_file` distinguishes `Ok(None)` (file absent — asserted by the sibling `read_config_file_missing_returns_ok_none`) from `Err` (unreadable). The name claims EACCES collapses into the absent case, which is the exact fail-open behaviour the `Err` return exists to prevent. A reader scanning test names to learn the contract learns the wrong one, and anyone "fixing" the code to match the name would reintroduce a silent config-drop.

2. **It cannot fail in a privileged sandbox.** `chmod 0o000` is bypassed by `CAP_DAC_OVERRIDE`, so in a root container — the default for Docker-based CI — `read_config_file` succeeds and `assert!(result.is_err())` fails, or (if the read is refused for some other reason) passes for a reason unrelated to permissions. The rest of this crate already handles that: `edit.rs::sync_parent_dir_warns_when_parent_open_fails` (crates/core/src/config/edit.rs:816-824) explicitly guards its assertion with a comment — *"A privileged sandbox (CI running as root, fakeroot, etc.) can bypass DAC and the open will actually succeed"* — and only asserts when the OS actually denied. This test has no such guard.

The restore is also unconditional-on-success only in the sense that it runs after the assertion:

```rust
let result = read_config_file(&path);
assert!(result.is_err(), "permission denied should return Err");

let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
```

A failing assertion panics before the chmod-back, leaving a 0o000 file that can make the `TempDir` teardown noisy. The sibling tests in `edit.rs` deliberately restore *before* asserting for this reason.

**Why it matters**: TEST-2 / TEST-18. This is the only test covering the fail-closed behaviour of the config reader on an unreadable file — the branch that decides whether a config the user cannot read is silently ignored or loudly refused. A test whose name documents the wrong contract and which no-ops under the most common CI topology provides no protection for it.

<!-- scan confidence: verified by reading validate_tests.rs:315-330 and comparing against the root-guard idiom already used at crates/core/src/config/edit.rs:816-824 -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The test is renamed to state the outcome it actually asserts (e.g. read_config_file_permission_denied_returns_err)
- [ ] #2 The test detects the privileged-sandbox case the way edit.rs::sync_parent_dir_warns_when_parent_open_fails does, and either skips or asserts the real behaviour rather than failing when DAC is bypassed
- [ ] #3 Permissions are restored before the assertion so a failure cannot leave a 0o000 file behind for TempDir teardown
<!-- AC:END -->
