---
id: TASK-1040
title: 'ERR-1: atomic_write skips parent-directory fsync for bare-filename paths'
status: Done
assignee: []
created_date: '2026-05-07 20:52'
updated_date: '2026-05-08 06:28'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`crates/core/src/config/edit.rs:91-180\`

**What**: \`atomic_write\` resolves the parent directory via \`path.parent().unwrap_or_else(|| Path::new(\".\"))\`. For a \`Path::new(\"foo.toml\")\` (a bare filename, no directory component), \`Path::parent()\` returns \`Some(\"\")\` — not \`None\` — so the \`unwrap_or_else\` branch is NOT taken. The empty-path is then used for two operations: (1) \`parent.join(tmp_name)\` which silently produces just the tmp_name, fine; (2) the \`#[cfg(unix)]\` block at line 172 calls \`std::fs::File::open(parent)\` with an empty path, which fails with \`ENOENT\`. The error is swallowed by the outer \`if let Ok(dir) = ...\`, so the parent-directory fsync is silently skipped — defeating the documented crash-safety guarantee on ext4 for bare-filename writes.

The companion \`init_cmd::write_init\` at line 78-82 already handles this case correctly by remapping empty parent to \`Path::new(\".\")\`. \`atomic_write\` should mirror that.

Repro: \`atomic_write(Path::new(\"foo.toml\"), b\"x\")\` from a process whose cwd is on ext4 — the rename succeeds, but the dir entry is not fsync-d, so a power loss between rename and the next sync(2) can lose the new file.

**Why it matters**: the comment at line 162-170 explicitly documents the parent fsync as the only signal that crash-safety is currently broken. Silently no-oping it for bare-filename paths means callers that relied on \`atomic_write\` for the \`.ops.toml\` path (which IS bare in production: \`PathBuf::from(\".ops.toml\")\` from \`init_cmd.rs:21\`) silently lose the guarantee. The asymmetry with \`init_cmd::write_init\`'s no-force branch — which DOES handle this — is the loud bug: the same crash-safety story should apply on both branches.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 atomic_write remaps empty parent path to Path::new(".") before opening for fsync
- [x] #2 regression test exercises atomic_write with a bare-filename path and asserts the fsync codepath was reached (e.g. via tracing capture or by checking the parent was openable)
<!-- AC:END -->
