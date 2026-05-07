---
id: TASK-1038
title: >-
  ERR-2: tools/install install_tool routes System+rustup_component path through
  cargo (silent semantic mismatch)
status: Done
assignee: []
created_date: '2026-05-07 20:25'
updated_date: '2026-05-07 23:30'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/install.rs:118-137`

**What**: `install_tool` first installs the rustup component (when present), then dispatches on `spec.source()`. The `ToolSource::System` arm bails with "system tools cannot be auto-installed" *unless* `spec.rustup_component()` is Some — in which case the function returns Ok early (the rustup component install above happened first, and the System arm short-circuits the bail). That's the documented intent.

But the `ToolSource::Cargo` arm always calls `install_cargo_tool(name, spec.package())` regardless of whether a rustup component install has already happened. If a tool spec lists *both* a Cargo source and a rustup_component (e.g. a hypothetical 'clippy' wrapper packaged as a cargo subcommand and also as a rustup component), the function will install the rustup component AND the cargo binary — silently producing two installations where the operator's intent was probably one or the other.

**Why it matters**: Low. Today no entry in the default ToolSpec config has both fields set, but `ToolSpec` allows it and there's no validation. Documented behaviour for the both-set case should be either: warn and prefer one path, or fail closed at config-parse time.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ToolSpec configurations that set both rustup_component and ToolSource::Cargo are explicitly handled (rejected at parse, or one path chosen with a warn)
- [ ] #2 Test pins the chosen behaviour
<!-- AC:END -->
