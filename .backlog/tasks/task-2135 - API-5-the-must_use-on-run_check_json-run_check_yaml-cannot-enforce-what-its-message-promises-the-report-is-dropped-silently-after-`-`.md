---
id: TASK-2135
title: 'API-5: the #[must_use] on run_check_json/run_check_yaml cannot enforce what its message promises - the report is dropped silently after `?`'
status: Done
assignee: []
created_date: '2026-09-08 06:56'
updated_date: '2026-09-10 15:59'
labels:
  - code-review-rust
  - api-design
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions/config-checkers/src/report.rs
  - extensions/config-checkers/src/runner.rs
priority: low
ordinal: 51000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/runner.rs:22`, `extensions/config-checkers/src/runner.rs:42`

**What**: both entry points carry

```rust
#[must_use = "the CheckerReport drives the process exit code; ignoring it defeats the validator"]
pub fn run_check_json(opts: &CheckerOptions, writer: &mut dyn Write) -> anyhow::Result<CheckerReport>
```

The return type is `Result`, which is **already** `#[must_use]`, so the
attribute changes only the diagnostic text for the one case that was already
caught (`run_check_json(&o, &mut w);` as a statement). The case the message
actually describes — using the call for its side effects and discarding the
report — is not caught at all:

```rust
run_check_json(&opts, &mut writer)?;   // compiles clean, no lint
let _ = run_check_json(&opts, &mut writer)?;   // likewise
```

Once `?` unwraps the `Result`, the `CheckerReport` inside is an ordinary
value: `must_use` on the *function* does not propagate through `?`, and
`CheckerReport` itself (`report.rs:33`) derives only `Debug, Default` with no
`#[must_use]`.

`CheckerReport::failed` (`report.rs:60`) is `#[must_use]`, but that only fires
if someone already decided to call it.

**Why it matters**: the attribute reads as a guarantee and is documented as
one, so the next caller added to `crates/cli/src/subcommands.rs` can drop the
report — and with it the non-zero exit — with no compiler complaint. That is
precisely the fail-open the message warns about: the checker runs, prints its
failure lines, and the CLI exits 0. TASK-1593 added these attributes for this
reason; the mechanism chosen does not reach the case.

The lint that does reach it is `#[must_use]` on the `CheckerReport` type,
which survives `?` because it attaches to the value rather than the call.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CheckerReport is annotated #[must_use] with a message naming the exit-code consequence, so a discarded report is a warning even after ?
- [x] #2 The now-redundant function-level #[must_use] attributes on run_check_json / run_check_yaml are removed, or kept only with a message that describes what they actually catch
- [x] #3 A compile-fail or trybuild-style check (or, at minimum, a documented manual verification) confirms that `run_check_json(&opts, &mut w)?;` produces an unused_must_use warning

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC3 verified manually: a scratch test discarding run_check_json(..)? produced unused_must_use with the type-level message under cargo clippy; scratch removed after verification.
<!-- SECTION:NOTES:END -->
