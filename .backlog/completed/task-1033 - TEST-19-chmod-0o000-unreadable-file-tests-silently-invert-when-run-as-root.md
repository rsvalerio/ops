---
id: TASK-1033
title: 'TEST-19: chmod 0o000 unreadable-file tests silently invert when run as root'
status: Done
assignee: []
created_date: '2026-05-07 20:24'
updated_date: '2026-05-08 06:29'
labels:
  - code-review-rust
  - test
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `crates/core/src/text.rs:301-318` (`for_each_trimmed_line_unreadable_file_returns_none`)
- `crates/core/src/stack.rs:328-388` (`extension_walk_per_entry_error_logs_debug_breadcrumb`)
- `crates/core/src/config/edit.rs:332-343` (`atomic_write_preserves_restrictive_destination_perms` — also susceptible: root can write 0o600 files, but the *cap* is consistent there; the read-side tests are the real hazard)

**What**: Two `#[cfg(unix)]` regression tests assume DAC permission bits gate the read syscall:

1. `for_each_trimmed_line_unreadable_file_returns_none` — chmods a file to `0o000` and asserts `for_each_trimmed_line(...) == None`. As root (UID 0) the kernel skips the DAC check; the read succeeds, the callback runs, and the test would fail with "expected None, got Some(())".
2. `extension_walk_per_entry_error_logs_debug_breadcrumb` — chmods a sibling dir to `0o000` to provoke a per-entry IO error. As root the dir is readable, no per-entry error is synthesised, and the test passes for the wrong reason (it claims to verify the error-path breadcrumb but actually only verifies that `Stack::detect` still returns `Terraform`). The test's own comment acknowledges it can't deterministically force the error path.

**Why it matters**: Container CI (Docker default UID is root, GitHub Actions self-hosted runners often run privileged builds, `cargo test` inside a rootful devcontainer) silently inverts the assertion in case (1) and emits a green-but-meaningless result in case (2). The failure mode is "test starts failing the day a CI image switches base"; the cure is a UID guard or an alternative mechanism (open the file then `set_permissions` on a closed handle, or use `/proc/self/fd/X` style — or just gate on `geteuid() != 0` and skip).

**Calibration**: TEST-19 ("Use tempfile::tempdir() / no hardcoded paths / isolated state per test"). Both tests use tempdir but the *isolation contract* breaks when the caller's UID grants superuser bypass. Same pattern is documented in `flakiness-patterns.md` under host-environment-dependence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add an early skip (`if nix::unistd::geteuid().is_root() { return; }` or equivalent) or `#[cfg_attr(target_os = "linux", ignore = "...")]` guard on tests that depend on DAC permission denial
- [x] #2 Document the rationale inline so future contributors do not strip the guard
- [x] #3 Audit the rest of the workspace for the same chmod-0o000 + assert-failure pattern and apply the same guard or restructure the test
<!-- AC:END -->
