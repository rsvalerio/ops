---
id: TASK-2094
title: 'UNSAFE-10: runner unsafe blocks (termios, killpg) have no Miri coverage in CI'
status: Done
assignee:
  - TASK-2245
created_date: '2026-09-07 22:59'
updated_date: '2026-09-08 15:59'
labels:
  - code-review-rust
  - unsafe
dependencies: []
modified_files:
  - crates/runner/src/terminal.rs
  - crates/runner/src/command/process_group.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/terminal.rs:71-98,143-147`; `crates/runner/src/command/process_group.rs:115,169-177`

**What**: The crate's four unsafe sites all carry specific, high-quality SAFETY comments (MaybeUninit + tcgetattr/tcsetattr on STDERR_FILENO; killpg with the pid-reservation argument). But nothing ever executes this code under Miri: ci.yml runs fmt / check / clippy / nextest / doctests / cargo-deny only. The MaybeUninit assume_init pattern in EchoGuard::disable_echo is exactly the class of bug (partially-initialised struct read after a failed call) Miri exists to catch — its correctness currently rests on the ret != 0 branch ordering alone.

**Why it matters**: UNSAFE-10 — where unsafe is justified, a Miri run is one of the three checks the rule requires. Miri cannot execute libc tcsetattr/killpg semantics faithfully, so the realistic scope is narrow: run the guard-construction paths under `cargo +nightly miri test` with the unsupported syscalls either short-circuited (the existing non-TTY branch already covers the early return) or the limitation written down where the next reviewer will find it. Either outcome satisfies the rule; silence does not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CI runs a Miri job over the runner crate (nightly, ), or the unsafe modules' docs record explicitly why Miri cannot execute these libc paths and what covers them instead
- [x] #2 The EchoGuard MaybeUninit/assume_init path is exercised by whichever mechanism is chosen (Miri or documented exemption)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-2245. (1) ci.yml miri job runs the runner EchoGuard construction path (`disable_echo` test) — locally verified passing under Miri: stderr is a pipe under the harness, so the non-TTY early-return branch executes. (2) terminal.rs module docs now record the UNSAFE-10 exemption for the MaybeUninit/tcgetattr/tcsetattr path: Miri has no shims for foreign termios calls and cannot provide a real TTY on stderr; substitute evidence is the per-block SAFETY prose (assume_init conditional on the POSIX ret==0 check) and the TEST-5 manual PTY testing. (3) process_group.rs module docs record the same for killpg, with the pid-reservation SAFETY argument and the cancellation tests under the ordinary Test job as the substitute evidence.
<!-- SECTION:NOTES:END -->
