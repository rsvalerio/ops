---
id: TASK-2096
title: >-
  TEST-33: ConfigurableTheme::new performs stderr I/O via
  warn_on_running_template_overhead with no injection point
status: To Do
assignee:
  - TASK-2241
created_date: '2026-09-07 22:59'
updated_date: '2026-09-08 10:56'
labels:
  - code-review-rust
  - testability
dependencies: []
modified_files:
  - crates/theme/src/configurable.rs
priority: low
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:107`

**What**: `ConfigurableTheme::new` calls `warn_on_running_template_overhead` (same file, line 423), which writes directly to the operator's stderr through `ops_core::ui::warn` when the theme's `running_template_overhead` is below the derived lower bound. The constructor is therefore side-effectful I/O: every construction path — including `resolve_theme` and `resolve_theme_owned`, and any test or library embedding that builds a theme to inspect it — can emit a warning line to stderr, and a caller that resolves a theme more than once gets the same warning repeated. There is no way to construct a theme quietly or to capture the diagnostic programmatically.

**Why it matters**: the validation itself is good (READ-5 / TASK-1971); the delivery channel is the problem. A constructor that writes to stderr cannot be used in a captured-output context without noise, and the diagnostic is only reachable as a side effect rather than as a value. Alternatives that keep the check: return the warning from a `validate()` method the caller renders; collect diagnostics into a `Vec<String>` field exposed by a getter; or emit the warning from the runner at theme-resolution time (where other config warnings presumably already surface) instead of from the constructor.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ConfigurableTheme::new performs no I/O; the running_template_overhead diagnostic is returned, collected, or emitted at resolution time by the caller
- [ ] #2 A test constructs a misconfigured theme and asserts the diagnostic is observable without reading stderr
<!-- AC:END -->
