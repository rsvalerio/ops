---
id: TASK-2167
title: 'API-5: the #[must_use] on run_trailing_whitespace/run_end_of_file_fixer cannot enforce what its message promises - the FixerReport is dropped silently after `?`'
status: To Do
assignee: []
created_date: '2026-09-08 07:06'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions/text-fixers/src/runner.rs
  - extensions/text-fixers/src/report.rs
priority: low
ordinal: 80000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/runner.rs:20`, `extensions/text-fixers/src/runner.rs:38`

**What**: both entry points carry

```rust
#[must_use = "the FixerReport drives the process exit code; ignoring it defeats the fixer"]
pub fn run_trailing_whitespace(opts: &FixerOptions, writer: &mut dyn Write) -> anyhow::Result<FixerReport>
```

The return type is `Result`, which is already `#[must_use]`, so the attribute
only changes the diagnostic text for the case that was already caught. The
case the message describes — running the fixer for its side effects and
discarding the report — compiles clean:

```rust
run_trailing_whitespace(&opts, &mut w)?;        // no lint
let _ = run_trailing_whitespace(&opts, &mut w)?; // no lint
```

`must_use` on the *function* does not propagate through `?`, and `FixerReport`
itself (`report.rs:63`) derives only `Debug, Default` with no `#[must_use]`.
`FixerReport::changed` and `FixerReport::failed` are `#[must_use]`, but those
only fire once someone has already decided to call them.

**Why it matters**: the attribute reads as a guarantee and is documented as
one. `crates/cli/src/subcommands.rs:329-346` is the only caller that maps
`changed() || failed()` onto `ExitCode::FAILURE`; the next caller added can
drop the report — and with it the non-zero exit — with no compiler complaint.
That is precisely the fail-open the message warns about: the fixer rewrites
files, prints its lines, and the process exits 0, so the pre-commit driver
reads the tree as clean.

Exact twin of TASK-2135 in `ops-config-checkers`; the two should be fixed the
same way. The lint that reaches the case is `#[must_use]` on the *type*, which
survives `?` because it attaches to the value rather than to the call.

<!-- scan confidence: verified — both attributes and the CLI caller read in full -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 FixerReport is annotated #[must_use] with a message naming the exit-code consequence, so a discarded report warns even after ?
- [ ] #2 The now-redundant function-level #[must_use] attributes are removed, or kept only with a message describing what they actually catch
- [ ] #3 A compile-fail check or a documented manual verification confirms that run_trailing_whitespace(&opts, &mut w)?; produces an unused_must_use warning
- [ ] #4 Resolved consistently with TASK-2135 in config-checkers
<!-- AC:END -->
