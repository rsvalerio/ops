---
id: TASK-0898
title: >-
  SEC-25: atomic_write loses restrictive perms on existing files via default
  umask
status: Done
assignee: []
created_date: '2026-05-02 10:08'
updated_date: '2026-05-02 14:46'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:98`

**What**: OpenOptions::new().write().create_new().open() does not set explicit permissions, so the temp file inherits the process umask (typically 0644). Atomically renaming over an existing .ops.toml that the user previously chmod'd to 0600 silently widens its ACL to 0644.

**Why it matters**: Repeated `ops about setup` / theme edits silently strip restrictive permissions the user set on .ops.toml, exposing config to other local users on shared hosts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 atomic_write stat()s the destination (when it exists) and applies the same mode bits to the temp file via OpenOptions::mode on Unix
- [x] #2 When destination is absent, default to 0o600 to avoid leaking through umask
- [x] #3 Add a unit test that pre-creates a file at 0o600, calls atomic_write, and asserts mode is preserved
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
On Unix, atomic_write now stat()s the destination and applies its mode bits to the temp file via OpenOptionsExt::mode; defaults to 0o600 when the destination is absent so umask cannot leak. Two regression tests pin both branches: atomic_write_preserves_restrictive_destination_perms (existing 0o600 survives a replace) and atomic_write_defaults_new_file_to_0600 (no-destination case).
<!-- SECTION:NOTES:END -->
