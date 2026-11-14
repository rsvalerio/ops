---
id: TASK-2254
title: 'PATTERN-1: go_mod::GoMod::local_replaces has no production reader after TASK-2178 dropped replaces from module_count'
status: Done
assignee: []
created_date: '2026-09-08 16:58'
updated_date: '2026-09-16 17:25'
labels:
  - code-review-rust
  - pattern
dependencies: []
parent_task_id: 'TASK-2268'
modified_files:
  - extensions-go/about/src/go_mod.rs
priority: low
ordinal: 160000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:32`, `extensions-go/about/src/go_mod.rs:110-156`

**What**: TASK-2178 (wave5) removed the only production consumer of `GoMod::local_replaces` — `compute_module_count` no longer counts local `replace` targets (a replace is a dependency substitution, not a workspace member). The parser still collects them: `parse_replace_directive` plus its version-shape classification (`looks_like_module_version`), Windows-absolute detection, SEC-14 traversal scrub, and quoting rules, with ~20 tests in `go_mod.rs` pinning that behaviour. Nothing outside `#[cfg(test)]` reads the field now.

**Why it matters**: A `pub(crate)` field written but never read in production is dead weight that the next reviewer must re-investigate: either some future consumer (a deps view? a replace diagnostics card?) wants it — in which case the capability should be documented as deliberately retained — or the parse arm, the helper predicates, and their tests should be deleted. Removal was not done in-wave because it is not mechanical (security-hardened parsing with task-anchored rationale, and waves 6/13/14 concurrently touch this crate).

**Origin**: discovered during TASK-2239 (wave5) while fixing TASK-2178.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A decision is recorded: retain local_replaces with a documented future consumer, or remove the field, its parse arm, and its tests

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision: removed. `local_replaces`, its parse arm (`parse_replace_directive`, `looks_like_module_version`, `is_windows_absolute`), and the replace-target tests are gone; the parser still recognizes and skips `replace` directives (block tracking, unterminated-block rollback and warn kept, entries consumed not parsed). Rationale: `about dependencies` is Rust-only per README and no planned Go consumer exists, so the future-consumer branch of the decision was speculative. Stale sharing-claim doc comments in go_syntax.rs and modules.rs updated.
<!-- SECTION:NOTES:END -->
