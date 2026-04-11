---
id: TASK-0030
title: "Extract load_config + build_data_registry pattern repeated in 3 match arms"
status: Triage
assignee: []
created_date: '2026-04-09 20:30:00'
labels: [rust-code-duplication, CD, DUP-3, low, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/main.rs:128-129`, `crates/cli/src/main.rs:136-137`, `crates/cli/src/main.rs:146-148`
**Anchor**: `fn run` (About, Deps, Dashboard match arms)
**Impact**: The 2-line pattern `load_config_and_cwd()` followed by `build_data_registry()` is repeated in 3 consecutive match arms. If the data context setup changes (e.g., adding workspace root validation or caching), all three arms need updating.

**Notes**:
Each arm starts with:
```rust
let (config, cwd) = load_config_and_cwd()?;
let registry = crate::registry::build_data_registry(&config, &cwd)?;
```

Then diverges into the specific subcommand call. `build_data_registry` was already extracted to reduce boilerplate (per its DUP-003 comment), so the remaining 2-line pattern is small.

Fix: extract `fn load_data_context() -> anyhow::Result<(Config, PathBuf, DataRegistry)>` to combine both steps. Low priority since the residual duplication is minimal and well-contained within a single function.
