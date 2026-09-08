---
id: TASK-2087
title: >-
  UNSAFE-10: production unsafe FFI in ops-core is never exercised under Miri (no
  Miri job in CI)
status: To Do
assignee:
  - TASK-2245
created_date: '2026-09-07 22:58'
updated_date: '2026-09-08 10:58'
labels:
  - code-review-rust
  - unsafe
dependencies: []
modified_files:
  - crates/core/src/text.rs
  - crates/core/src/config/edit.rs
  - .github/workflows/ci.yml
priority: medium
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:247-460`; `crates/core/src/config/edit.rs:268-271`; `.github/workflows/ci.yml`

**What**: `ops-core` carries production `unsafe` in two places, both correctly justified as FFI/platform calls and both with per-block `// SAFETY:` prose (UNSAFE-10 check 1 passes):
- `text.rs` `unix_open` module: `libc::openat`, `libc::fstat` + `MaybeUninit::assume_init`, `libc::fcntl` F_GETFL/F_SETFL, `OwnedFd::from_raw_fd`, `File::from_raw_fd` — the SEC-25/TASK-2038 symlink-refusing component walk.
- `config/edit.rs` `build_tmp_basename`: `OsStr::from_encoded_bytes_unchecked` after stripping one leading ASCII `.`.

However, UNSAFE-10 check 2 — the code is exercised under Miri (`cargo +nightly miri test`) — is absent: `.github/workflows/ci.yml` runs only fmt, cargo check (all-features/all-targets), clippy `-D warnings`, an MSRV job, and cargo deny. No workflow file, justfile target, or `.cargo/config` mentions Miri anywhere in the repo.

**Why it matters**: UNSAFE-10 — Miri catches aliasing, alignment, initialization, and provenance violations that pass every ordinary test. The `fstat` + `assume_init` and `from_raw_fd` ownership-transfer patterns are exactly the class of mistake Miri is designed to surface, and a regression in the SAFETY invariants (e.g. a future edit to the component walk reordering `clear_nonblock` before the type check, or a double-close via a cloned fd) would land silently with the current CI matrix. The crate otherwise enforces its lint policy mechanically (workspace pedantic/nursery deny, `clippy.toml` msrv check) — the unsafe-evidence check is the one discipline with no mechanical gate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Miri job (e.g. `cargo +nightly miri test -p ops-core`, possibly restricted to the `text::unix_open` and `edit::build_tmp_basename` tests if full-crate Miri hits unsupported libc operations) runs in CI, or the repo documents why Miri cannot cover this unsafe and what evidence substitutes for it
- [ ] #2 If Miri cannot run the libc FFI paths on the CI runner, the job at minimum covers the pure-memory unsafe (`OsStr::from_encoded_bytes_unchecked`) and the fd-ownership transfer logic via a Miri-friendly seam, with the exclusion documented in the workflow
<!-- AC:END -->
