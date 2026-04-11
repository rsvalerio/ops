---
id: TASK-0009
title: No startup validation of ExecCommandSpec fields
status: Done
assignee: []
created_date: '2026-04-10 12:00:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-security
  - RS
  - SEC-30
  - medium
  - crate-core
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/config/loader.rs:34-53`
**Anchor**: `load_config`
**Impact**: Config fields are deserialized from `.ops.toml`, global config, `.ops.d/`, and `OPS__` env vars without validation. Invalid values like an empty `program` string, `timeout_secs = 0` (immediate timeout), or non-existent `cwd` paths are only caught at command execution time, producing confusing errors instead of failing fast at startup.

**Notes**:
SEC-30 requires validating configuration at startup and failing fast on insecure settings. Current behavior:
- `ExecCommandSpec.program` can be an empty string — `Command::new("")` will fail at execution with an OS error
- `ExecCommandSpec.timeout_secs` can be 0 — `Duration::from_secs(0)` creates a zero-duration timeout that immediately races against the child process, almost certainly killing it instantly
- `ExecCommandSpec.cwd` can be a non-existent path — `Command::current_dir()` fails at execution time
- No validation in `load_config()` or `CommandRunner::new()`

Fix: Add a `validate()` method to `ExecCommandSpec` (or on `Config` after loading) that checks:
- `program` is non-empty
- `timeout_secs` is either `None` or `> 0`
- `cwd` exists on disk (if set)
Call it in `load_config()` or `CommandRunner::new()` and surface errors before any command runs.

**OWASP**: A05 (Security Misconfiguration)
<!-- SECTION:DESCRIPTION:END -->
