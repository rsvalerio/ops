---
id: TASK-1882
title: >-
  SEC-25: the new-hook path writes pre-commit in place, so a failed or
  interrupted install leaves a truncated hook git will happily run
status: To Do
assignee:
  - TASK-2008
created_date: '2026-08-27 15:33'
updated_date: '2026-08-28 14:16'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/hook-common/src/install.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:41-75` (`install_hook` -> `write_new_hook`)

**What**: The *upgrade* path (`upgrade_legacy_hook`) is carefully crash-safe: it stages the payload in a randomised `tempfile::NamedTempFile`, fsyncs it, rechecks, then `persist`s it with an atomic `rename(2)` (SEC-25 / TASK-1210). The *create* path is not. `install_hook` opens the real hook path with `OpenOptions::create_new(true)` and `write_new_hook` then writes the script directly into that live path:

```rust
Ok(file) => write_new_hook(file, &hook_path, config, w),
...
file.write_all(config.hook_script.as_bytes()).context("failed to write hook")?;
file.sync_all().context("failed to fsync hook")?;
drop(file);
set_hook_executable(hook_path)?;
```

If `write_all`, `sync_all`, or `set_hook_executable` fails (ENOSPC, EIO, EDQUOT, a full `/tmp`-style `.git` on a quota'd volume) or the process is killed mid-write, the function returns `Err` — or never returns — and **leaves the partially written file at `.git/hooks/<hook>` with no cleanup**. `write_new_hook`'s own comment ("if the system crashes between install and the next git invocation, fsync prevents a zero-byte hook") only covers a crash *after* a successful write; it does nothing for a failure *during* one.

Two consequences, both bad for a hook crate:

1. **Silently disabled gate (fail-open).** A zero-length file is a valid executable that exits 0, so git runs it and every subsequent `git commit` passes with no checks and no diagnostic. A truncated script (`#!/usr/bin/env bash\nexec ops run-before-com`) is the opposite failure — it blocks every commit with a shell error the user cannot attribute to ops.
2. **Wedged reinstall.** On the next `ops <hook>-install`, `handle_existing_hook` reads the truncated content, finds no legacy marker in it, and bails with *"a pre-commit hook already exists ... and was not installed by ops (first line: ...). Remove it manually"* — telling the operator that ops's own half-written artefact is a foreign user hook.

The fix already exists in the same file: route the create path through the same stage-and-rename that `upgrade_legacy_hook` uses (write payload to a randomised sibling, fsync, chmod, `persist`), or at minimum remove the partial file on the error path the way `upgrade_legacy_hook` does with its `inspect_err(|_| { let _ = std::fs::remove_file(&tmp_path); })`.

**Why it matters**: the whole point of the pre-commit/pre-push extensions is that the gate is either present and enforcing or absent and obviously absent. A partially written hook is a third state the installer creates by itself, and its most likely shape (empty file) is the one that reports success to git on every commit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 install_hook's create path stages the hook payload in a temp sibling and renames it into place atomically, so .git/hooks/<hook> is never observable in a partially written state
- [ ] #2 If any step of the create path fails, no file is left at the hook path (the staged temp file is removed and the error is propagated with context)
- [ ] #3 A test simulates a write failure (or interruption) during first-time install and asserts that no file — in particular no empty file — remains at .git/hooks/pre-commit afterwards
- [ ] #4 A test asserts an empty or truncated pre-existing hook file is not reported to the operator as a foreign user-authored hook
<!-- AC:END -->
