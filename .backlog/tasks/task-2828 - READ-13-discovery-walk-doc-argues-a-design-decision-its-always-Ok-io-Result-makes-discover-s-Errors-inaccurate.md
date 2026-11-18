---
id: TASK-2828
title: 'READ-13: discovery::walk doc argues a design decision; its always-Ok io::Result makes discover''s # Errors inaccurate'
status: To Do
assignee: []
created_date: '2026-11-18 18:49'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2859'
modified_files:
  - extensions/text-fixers/src/discovery.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/text-fixers/src/discovery.rs:walk'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/discovery.rs:218-241` (`walk`), `extensions/text-fixers/src/discovery.rs:157-160` (`discover` `# Errors`)

**What**:
1. `walk`'s `///` block carries a "# No dispatch deadline here" section (lines 225-237) that compares the function with `tokei` / `rust-loc` providers, explains why a timeout was not added, and ends "Revisit only if a fixer ever becomes reachable through provider dispatch". That is a design-decision essay about what the code does *not* do — a process artifact READ-13 says belongs in an ADR / PR, not item docs; it will go stale as soon as either of the referenced crates changes.
2. `walk` returns `io::Result<..>` but can never fail (`#[allow(clippy::unnecessary_wraps)]`, line 240: every per-entry error is collected into the `Vec<String>`). `walked` inherits the dead `Result`, and `discover`'s `# Errors` section states "If the filesystem walk fails" — a failure mode that does not exist. A reader (or the config-checkers caller) handles an error path that is unreachable.

**Why it matters**: Item docs should describe the contract. The essay obscures it, and the inaccurate `# Errors` misstates which failures `discover` can return (only `git ls-files` failures).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 walk's doc describes what it returns and its error-collection behaviour; the no-deadline rationale moves to an ADR or is reduced to a one-line note
- [ ] #2 walk/walked either return plain values (dropping the allow(clippy::unnecessary_wraps)) or the dead Result is otherwise justified, and discover's # Errors lists only the git ls-files failure cases
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.31 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
