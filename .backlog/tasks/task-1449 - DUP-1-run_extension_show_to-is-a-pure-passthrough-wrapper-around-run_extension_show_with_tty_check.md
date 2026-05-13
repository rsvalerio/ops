---
id: TASK-1449
title: >-
  DUP-1: run_extension_show_to is a pure passthrough wrapper around
  run_extension_show_with_tty_check
status: To Do
assignee:
  - TASK-1457
created_date: '2026-05-13 19:04'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:260-270` and `272-338`

**What**: `run_extension_show_to` (lines 260-270) is a bare delegating wrapper that immediately forwards every parameter to `run_extension_show_with_tty_check` (line 272) with no transformation. Both functions have identical signatures (`w`, `config`, `name: Option<&str>`, `is_tty: F`) and identical generic bounds (`F: FnOnce() -> bool`).

```rust
fn run_extension_show_to<F>(
    w: &mut dyn Write,
    config: &ops_core::config::Config,
    name: Option<&str>,
    is_tty: F,
) -> anyhow::Result<()>
where
    F: FnOnce() -> bool,
{
    run_extension_show_with_tty_check(w, config, name, is_tty)
}
```

The doc comment on `run_extension_show_to` claims it "delegates to `run_extension_show_with_tty_check` which contains the TTY/picker logic" — the entire body of `run_extension_show_to` is a single forwarding call.

**Why it matters**: Two functions with identical signatures and one being a one-line passthrough is dead code per DUP-1 (5+ line duplications consolidated, and single-line passthroughs that add no value). Callers either go through `run_extension_show_to` (which forwards) or directly to the inner — there is no behavioral difference. This is also visible as an unnecessary stack frame in error backtraces, and increases the cognitive footprint of a high-traffic file (833 LoC). The two functions should be unified: either keep only `run_extension_show_with_tty_check` and rename, or delete the inner and inline the body into `run_extension_show_to`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Consolidate run_extension_show_to and run_extension_show_with_tty_check into a single function with no passthrough wrapper
- [ ] #2 Update all call sites and tests to reference the single retained function
- [ ] #3 Ensure no behavioural change: TTY check, error messages, and stdout output remain byte-identical
<!-- AC:END -->
