---
id: TASK-1853
title: >-
  SEC-33: open_refusing_symlinks blocks forever on a FIFO planted at a manifest
  or .ops.toml path
status: Triage
assignee: []
created_date: '2026-08-27 15:27'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/core/src/text.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:152-168` (`open_refusing_symlinks`), reached from `read_capped_to_string`, `for_each_trimmed_line`, and `crates/core/src/config/loader/mod.rs:104` (`read_capped_toml_file_with`)

**What**: The open guards against symlinks and nothing else:

```rust
pub(crate) fn open_refusing_symlinks(path: &Path) -> std::io::Result<std::fs::File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
```

`O_NOFOLLOW` rejects a symlink at the final component. It says nothing about **file type**. Opening a FIFO `O_RDONLY` without `O_NONBLOCK` blocks inside `open(2)` until a writer appears — and there is no timeout anywhere on this path.

Every downstream defence is downstream of the open, so none of them run:

```rust
    let mut file = open_refusing_symlinks(path).map_err(|e| with_path(&e, path))?;
    let mut buf = String::new();
    let limit = cap.saturating_add(1);
    (&mut file).take(limit).read_to_string(&mut buf)...
```

The `SEC-33` byte cap and `MANIFEST_MAX_BYTES`'s `take(cap + 1)` never execute — the process never returns from `open`.

**Why it matters**: the module doc (text.rs:8-15) states the threat model explicitly — *"an adversarial repository could otherwise force unbounded allocations via an oversized or `/dev/zero`-symlinked manifest"*. A FIFO is that same attack with a strictly worse outcome: not a bounded allocation but a **permanent hang, with no error and no diagnostic**. `mkfifo go.mod` (or `package.json`, `requirements.txt`, `Cargo.toml`, `.ops.toml`) in a hostile checkout, a shared CI workspace, or any scratch directory a malicious `postinstall` can write to wedges `ops` until the CI job's own timeout kills the runner. The manifest readers in the `about` extensions call `read_capped_to_string` on repo-supplied paths on every invocation.

Fix direction: add `libc::O_NONBLOCK` to `custom_flags` and then `fstat` the returned descriptor and reject anything that is not `S_IFREG` (also closes `/dev/zero`, `/dev/random`, and character devices generally, which the current guard only blocks when they are reached *via a symlink*). Clear `O_NONBLOCK` after the type check if the subsequent read needs blocking semantics.

<!-- scan confidence: verified by reading text.rs:152-168, and reproduced with a probe binary calling ops_core::text::read_capped_to_string on a mkfifo'd go.mod — the call hung until `timeout 10` killed it (exit 124) -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 open_refusing_symlinks cannot block indefinitely: the open is non-blocking and the descriptor's type is checked before any read
- [ ] #2 A path that is a FIFO, character device, block device, or socket is refused with a stable named error rather than opened
- [ ] #3 A Unix regression test mkfifo's a manifest path and asserts read_capped_to_string returns an Err promptly rather than blocking
- [ ] #4 The rustdoc states the boundary the primitive enforces (regular files only, no symlink at the final component) rather than naming the O_NOFOLLOW flag alone
<!-- AC:END -->
