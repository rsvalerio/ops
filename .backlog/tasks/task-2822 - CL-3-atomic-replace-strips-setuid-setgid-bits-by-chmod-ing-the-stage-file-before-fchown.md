---
id: TASK-2822
title: 'CL-3: atomic::replace strips setuid/setgid bits by chmod-ing the stage file before fchown'
status: Done
assignee: []
created_date: '2026-11-18 18:49'
updated_date: '2026-11-18 21:38'
labels:
  - code-review-rust
  - CL
dependencies: []
parent_task_id: 'TASK-2859'
modified_files:
  - extensions/text-fixers/src/atomic.rs
priority: low
ordinal: 1000
dedup_key: 'CL-3:extensions/text-fixers/src/atomic.rs:preserve_attributes'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/atomic.rs:184-203` (`preserve_attributes`)

**What**: `preserve_attributes` calls `staged.set_permissions(original.permissions())` first and `fchown(staged, uid, gid)` second. On Linux, `chown`/`fchown` on a non-directory always sets `ATTR_KILL_SUID | ATTR_KILL_SGID`, even when the uid/gid are unchanged and even for an unprivileged owner, so the S_ISUID / S_ISGID bits just copied by `set_permissions` are cleared again. Verified locally: a file chmod'ed `06755` reads back `0755` after `fchown(fd, getuid(), getgid())`.

The module doc (`atomic.rs:55-56`) and `replace`'s doc promise "Mode, uid and gid *are* preserved"; for a setuid/setgid file (e.g. a setgid script or a file in a shared-group workflow) the fixer silently changes the mode. The implicit assumption — that chmod survives a later chown — is undocumented and false. No rule covers this exactly; CL-3 (implicit assumptions) is the closest.

**Why it matters**: A fixer that rewrites files on a pre-commit path changes a security-relevant mode bit without reporting it, contradicting its own contract. Low frequency (text files rarely carry suid/sgid), but silent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 preserve_attributes performs fchown before set_permissions so the final mode equals the original mode, including S_ISUID/S_ISGID
- [x] #2 A unix test rewrites a file with mode 02755 (or 04755) owned by the running user and asserts the mode survives

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.31 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
