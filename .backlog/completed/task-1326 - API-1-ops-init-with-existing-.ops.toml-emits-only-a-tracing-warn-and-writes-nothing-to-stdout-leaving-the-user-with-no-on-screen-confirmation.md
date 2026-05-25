---
id: TASK-1326
title: >-
  API-1: ops init with existing .ops.toml emits only a tracing::warn and writes
  nothing to stdout, leaving the user with no on-screen confirmation
status: Done
assignee:
  - TASK-1386
created_date: '2026-05-11 20:58'
updated_date: '2026-05-12 23:44'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/init_cmd.rs:29-42`

**What**: When `ops init` (without `--force`) is invoked in a directory that already contains `.ops.toml`, `run_init_to` matches the `ErrorKind::AlreadyExists` arm and returns `Ok(())` after emitting a single `tracing::warn!("ops.toml already exists; not overwriting (use --force to overwrite)")`. The function then **skips** the writer-bound stdout messages on lines 50-59 entirely.

The user-facing consequence:

```text
$ ops init           # .ops.toml is present
$ echo $?
0
```

With the default subscriber configuration the tracing event renders to *stderr* with a structured prefix (`<timestamp> WARN ops::init_cmd: ops.toml already exists ...`). Two failure modes follow:
1. Users who redirect stderr (`ops init 2>/dev/null`) — common when scripting `ops init` from a wrapper — see nothing at all. No stdout output, exit 0. They reasonably conclude the file was created or updated.
2. Even without redirection, users expect command-line tools to mirror the *outcome* (created / unchanged / failed) on stdout the same way `ops init` does on the success path. The success path writes one of three explicit `writeln!` lines (50-59) to the configured writer; the AlreadyExists path writes zero, breaking the symmetry. The `run_init_to(writer)` test seam is unreachable for this case.

This is the same shape of bug as TASK-1287 ("Extension load failure double-emits warn (tracing + UI)") in reverse — there the warn was duplicated; here it's *only* on the tracing channel, with no UI/stdout mirror.

**Why it matters**:
1. Silent no-op on a write command violates POSIX-style expectations. `ops init` is documented as creating `.ops.toml`; the user should be told plainly when it did not.
2. The behaviour is invisible to test coverage: `run_init_no_overwrite_without_force` (init_cmd.rs:192-197) verifies the file is unchanged but not that the user receives any notification, so a regression that silently drops the tracing event entirely would still pass.
3. `ops init --force` over an existing file does write to stdout ("Created .ops.toml..."), so the user gets inconsistent feedback for the two paths.

Fix: in the AlreadyExists arm, mirror the warn through the writer (e.g. `writeln!(w, ".ops.toml already exists; pass --force to overwrite")`) before returning. Optionally also use `ops_core::ui::warn` so it goes through the same UI channel as other operator-visible diagnostics. Add a test asserting the rendered stdout (via `run_init_to(..., &mut buf)`) contains the "already exists" hint.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops init over an existing .ops.toml writes a visible 'already exists; pass --force' line through the test-injectable writer, not only via tracing
- [x] #2 Unit test exercises run_init_to and asserts the user-visible text against a Vec<u8> buffer, mirroring the existing run_init_to_output_message_no_flags test for the happy path
- [x] #3 Documented contract: tracing::warn remains for structured-log consumers but is no longer the only user-facing signal
<!-- AC:END -->
