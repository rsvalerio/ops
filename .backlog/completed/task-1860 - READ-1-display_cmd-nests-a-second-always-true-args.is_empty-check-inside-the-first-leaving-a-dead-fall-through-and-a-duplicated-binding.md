---
id: TASK-1860
title: >-
  READ-1: display_cmd nests a second, always-true args.is_empty() check inside
  the first, leaving a dead fall-through and a duplicated binding
status: Done
assignee:
  - TASK-1983
created_date: '2026-08-27 15:29'
updated_date: '2026-08-28 23:53'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - crates/core/src/config/commands.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/commands.rs:247-261` (`ExecCommandSpec::display_cmd`)

**What**:

```rust
    pub fn display_cmd(&self) -> Cow<'_, str> {
        if self.args.is_empty() {
            let program = self.display_program.as_deref().unwrap_or(&self.program);
            if self.args.is_empty() {
                return shell_quote(program);
            }
        }
        let program = self.display_program.as_deref().unwrap_or(&self.program);
        Cow::Owned(format!(
            "{} {}",
            shell_quote(program),
            join_shell_quoted(&self.args)
        ))
    }
```

`self.args` is not touched between the two checks, so the inner `if` is unconditionally true whenever the outer one is — the outer block always returns, and the implied "outer true, inner false" fall-through into the `format!` is unreachable. The `let program = self.display_program.as_deref().unwrap_or(&self.program);` line is also written twice, once in each branch.

Behaviour is correct today (the two tests `exec_spec_display_cmd_no_args` and `exec_spec_display_cmd_prefers_display_program` both pass), so this is a readability defect, not a bug.

**Why it matters**: READ-1. `display_cmd` is the SEC-21 audit render — the string an operator reads in `--dry-run` output before greenlighting a `.ops.toml` they did not write — so it is a function that gets re-read under scrutiny. The shape as written invites a reader to reason about a fall-through path that cannot happen, and the duplicated `program` binding is the kind of near-miss that survives a refactor in one branch and not the other. Collapsing to a single `let program = …` followed by one `if self.args.is_empty() { return shell_quote(program); }` says exactly what the function does.

Neither clippy nor the compiler flags it: the inner condition is not a constant, so `unreachable_code` does not fire and no restriction lint covers the pattern.

<!-- scan confidence: verified by reading commands.rs:247-261 -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 display_cmd binds program once and has a single args.is_empty() check, with no unreachable branch
- [x] #2 The existing display_cmd tests (no-args, display_program override, metacharacter quoting) pass unchanged
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
`display_cmd` now binds `program` once and has a single `args.is_empty()` early return; the nested always-true check and the duplicated binding are gone. Behaviour unchanged — `exec_spec_display_cmd_no_args`, `exec_spec_display_cmd_prefers_display_program` and `exec_spec_display_cmd_quotes_metacharacters` pass unmodified.
<!-- SECTION:NOTES:END -->
