---
id: TASK-1581
title: >-
  API-3: tools public install_cargo_tool / install_rustup_component lack
  configurable timeout
status: Done
assignee:
  - TASK-1638
created_date: '2026-05-21 22:45'
updated_date: '2026-05-22 13:15'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/install.rs:12-14` and `:131-133`

**What**: The crate exposes `install_cargo_tool(name, package)` and `install_rustup_component(component, toolchain)` as the only public install entry points. Both hardcode `DEFAULT_INSTALL_TIMEOUT` (600s) by delegating to `*_with_timeout` variants kept `pub(crate)`. Downstream binaries that need a different deadline (e.g. faster CI, longer for embedded toolchains) cannot pass one without forking the crate or reaching into the private module.

**Why it matters**: API-3 (least-power principle): public APIs should accept the parameters callers may reasonably need. The internal variants already exist; promoting them — or accepting `impl Into<Option<Duration>>` — gives callers the lever without a breaking change. A wrapper crate hitting a slow registry currently has to wait the full 10 minutes or break out into raw `Command::new(resolve_cargo_bin)` without the validation/spawn scaffolding.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 install_cargo_tool / install_rustup_component (or new sibling functions) accept an explicit Duration timeout
- [x] #2 DEFAULT_INSTALL_TIMEOUT remains the default when no timeout is supplied
- [x] #3 existing call sites continue to compile with no source changes
<!-- AC:END -->
