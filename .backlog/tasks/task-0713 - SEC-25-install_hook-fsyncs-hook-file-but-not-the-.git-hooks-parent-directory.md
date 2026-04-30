---
id: TASK-0713
title: 'SEC-25: install_hook fsyncs hook file but not the .git/hooks parent directory'
status: To Do
assignee:
  - TASK-0739
created_date: '2026-04-30 05:30'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:59` and `:148`

**What**: `write_new_hook` and `write_temp_hook` both call `file.sync_all()` on the hook file, but neither fsyncs the parent `.git/hooks/` directory. POSIX requires fsyncing the parent directory after creating or renaming a file to ensure the directory entry hits disk; without it, a power loss between the install and the next git invocation can leave the hook absent (create_new) or pointing at the temp inode (rename) even though the file content is durable.

**Why it matters**: This is the same gap closed for `atomic_write` by TASK-0340. Hook installation is the moment `ops install` advertises "your hook is now active"; a crash that drops the directory entry silently disables the pre-commit hook without surfacing an error on the next boot. The cost (one extra File::open + sync_all on the parent) is negligible relative to the rest of the install path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After the create_new + sync_all in write_new_hook, fsync the parent .git/hooks directory
- [ ] #2 After the rename in upgrade_legacy_hook, fsync the parent .git/hooks directory so the rename hits disk
- [ ] #3 Match the pattern adopted by ops_core atomic_write (TASK-0340) for consistency
<!-- AC:END -->
