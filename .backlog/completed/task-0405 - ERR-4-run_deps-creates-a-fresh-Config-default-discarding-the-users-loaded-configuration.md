---
id: TASK-0405
title: >-
  ERR-4: run_deps creates a fresh Config::default(), discarding the user's
  loaded configuration
status: Done
assignee:
  - TASK-0420
created_date: '2026-04-26 09:52'
updated_date: '2026-04-27 11:41'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:130-153`

**What**: `run_deps` builds its own context with a hardcoded default Config:

```rust
let cwd = std::env::current_dir()?;
let config = std::sync::Arc::new(ops_core::config::Config::default());
let mut ctx = Context::new(config, cwd);
```

Unlike sibling subcommands (`run_about`, `run_extension_show`) that load the user's `.ops.toml` via the shared `load_config_and_cwd` helper (see TASK-0207), this entry point ignores it entirely. Any `[deps]` settings, env, or theme that ends up on `Config` is invisible inside the data provider.

**Why it matters**: Configuration drift between `ops deps` and `ops about deps` (which goes through the proper config-loading path). Future config-driven knobs (custom timeouts, allowed licenses, extension-specific overrides) added to `Config` silently do nothing for this subcommand.

**Suggested**: route through `load_config_and_cwd` like the other CLI handlers, or document why this command intentionally ignores user config.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_deps either loads the user config like sibling handlers or has a documented justification for using Config::default()
- [ ] #2 If config is loaded, regression test confirms that config-driven settings reach the DepsProvider
<!-- AC:END -->
