---
id: TASK-1811
title: >-
  SEC-25: the metadata size gate is advisory — --tracked follows a symlinked
  device and fs::read is unbounded
status: Done
assignee:
  - TASK-2004
created_date: '2026-08-27 11:32'
updated_date: '2026-08-28 22:25'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/config-checkers/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:243-283` (`run_checker`, the `std::fs::metadata` gate and the `std::fs::read` that follows)

**What**: the size cap is implemented as a `metadata()` check followed by an independent, uncapped `std::fs::read`:

```rust
match std::fs::metadata(&path) {
    Ok(md) if md.len() > opts.max_bytes => { ...skip... }
    Ok(_) => {}
    ...
}
let bytes = match std::fs::read(&path) { ... };
```

Two ways past it, both reachable:

1. **Non-regular files (`--tracked` only).** `std::fs::metadata` follows symlinks and the code never asks whether the target is a regular file. In walk mode this is harmless — `ops_text_fixers::discovery::walk` filters on `entry.file_type().is_file()` with `follow_links` off — but the `--tracked` path (`discovery::tracked_files`, `extensions/text-fixers/src/discovery.rs:78-99`) does no file-type filtering at all: it splits `git ls-files -z` and `root.join(rel)`s each entry. Git tracks symlinks (mode 120000), so a committed `evil.json -> /dev/zero` is handed straight to this loop. Verified in a scratch repo:

   ```
   $ git ls-files            ->  evil.json
   $ stat evil.json          ->  size 0, ischr True, isreg False
   ```

   `md.len()` is `0`, so `0 > max_bytes` is false and the gate passes. `std::fs::read` on `/dev/zero` then reads until an EOF that never arrives — unbounded allocation until the OOM killer or an allocation abort. A symlink to a FIFO blocks the process forever instead, with no timeout anywhere on the path. Either way one committed symlink hangs or kills `ops check-json --tracked` / `ops check-yaml --tracked` on any machine that runs it, including CI on an untrusted branch.

   (Cross-crate cause: `ops-text-fixers`'s `tracked_files` does not filter file types the way its own `walk` does. The fix still belongs here — this crate reads and parses the paths, so it owns the precondition, and hardening only `run_checker` fixes it for both discovery modes.)

2. **TOCTOU (SEC-25 proper).** Even for a regular file, `metadata()` and `read()` are two independent syscalls against a path. A file that measures under the cap at check time can be replaced or extended before the read, and `fs::read` has no ceiling of its own — it grows the `Vec` to whatever the file now is. The `metadata` result is used as an authorisation decision and then discarded.

**Why it matters**: SEC-25 / SEC-33. The documented purpose of `DEFAULT_MAX_BYTES` (`lib.rs:36-42`) is to stop "an allocator/parser DoS on CI runners and pre-commit hosts", and the check as written does not enforce that invariant on the read it guards. A cap that can be walked past by a tracked symlink is not a cap.

**Fix shape**: make the size limit a property of the read rather than of a prior stat. Open the file once and bound it: `let f = File::open(&path)?;` then `f.metadata()` on the *handle* (no second path resolution), reject `!md.is_file()` explicitly, and read through `f.take(max_bytes + 1)` so the byte ceiling holds regardless of what `metadata` said. `.take(max_bytes + 1)` also gives a free, correct oversize detection: if the read yields `max_bytes + 1` bytes, it is over the cap. A symlink pointing outside the repo root is worth rejecting in the same guard.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 run_checker rejects non-regular files (devices, FIFOs, directories) before reading them, so a tracked symlink to /dev/zero or a FIFO can neither hang nor exhaust memory
- [x] #2 The byte cap is enforced on the read itself (bounded reader / Read::take) rather than only on a prior metadata() call, closing the TOCTOU window
- [x] #3 The file is opened once and its metadata taken from the handle, not resolved twice by path
- [x] #4 A regression test covers a non-regular tracked entry (unix-gated symlink to a character device or FIFO) and asserts the checker neither hangs nor allocates unboundedly
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in wave TASK-2004, in the new `runner.rs`.

`open_regular_file` opens the path once and takes the authoritative type and
size from the *handle*; `read_bounded` then reads through
`file.take(max_bytes + 1)` and re-checks the byte count actually read, so the
cap is a property of the read and the metadata/read TOCTOU window is closed.
Non-regular files are skipped, not read. A path that has vanished between
discovery and read is skipped rather than failed (see TASK-1813).

AC#3 — substitution recorded. One stat by path survives, deliberately, *before*
`File::open`: opening a FIFO blocks in `open(2)` until a writer appears, so a
tracked symlink to one would hang the checker before any handle-based check
could run — which AC#1/#4 forbid. That stat authorises nothing; it is a
liveness guard only, and both the type and the size decisions that gate the
read come from the open handle. Doing it single-resolution would need
`O_NONBLOCK` via `custom_flags`, i.e. a `libc` dependency the workspace (and
TASK-1833, in this same wave) explicitly avoids.

AC#4: `tests::tracked_symlink_to_a_character_device_is_skipped_not_read` stages
a symlink to `/dev/zero` in a scratch git repo and asserts it is skipped with
"not a regular file", scanned 0 / skipped 1. A FIFO is not constructed
separately — `mkfifo` needs the same `libc` dep — but it takes the identical
`!md.is_file()` branch, one step earlier.
<!-- SECTION:NOTES:END -->
