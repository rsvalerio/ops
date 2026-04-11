---
id: TASK-0015
title: merge_env_vars silently swallows malformed OPS__ environment variable errors
status: Done
assignee: []
created_date: '2026-04-10 20:30:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-idioms
  - EFF
  - ERR-1
  - medium
  - crate-core
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/config/loader.rs:18-31`
**Anchor**: `fn merge_env_vars`
**Impact**: Both `config_crate::Config::builder().build()` and `merged.try_deserialize::<ConfigOverlay>()` errors are silently discarded via nested `if let Ok(...)`. When a user sets a malformed `OPS__` environment variable (e.g., `OPS__OUTPUT__COLUMNS=notanumber`), the override is silently ignored with no warning or error. The user expects their env var to take effect (env vars are layer 5 in the documented config merge order) but gets the default value instead.

**Notes**:
ERR-1: "handle or propagate, never both" — here the choice is neither. The errors are discarded without logging. Compare with `read_config_file` (same file, line 55-78) which logs `tracing::warn!` for read errors and `tracing::error!` for parse errors.

Fix: Add `tracing::warn!` for both error paths so users get feedback when their env var overrides fail to apply:
```rust
match env_config {
    Ok(merged) => match merged.try_deserialize::<ConfigOverlay>() {
        Ok(env_overlay) => merge_config(config, &env_overlay),
        Err(e) => tracing::warn!(error = %e, "failed to deserialize OPS__ env config"),
    },
    Err(e) => tracing::warn!(error = %e, "failed to build OPS__ env config"),
}
```
<!-- SECTION:DESCRIPTION:END -->
