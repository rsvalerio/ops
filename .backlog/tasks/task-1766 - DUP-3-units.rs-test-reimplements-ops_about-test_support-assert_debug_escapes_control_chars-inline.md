---
id: TASK-1766
title: >-
  DUP-3: units.rs test reimplements
  ops_about::test_support::assert_debug_escapes_control_chars inline
status: Done
assignee:
  - TASK-1992
created_date: '2026-08-27 11:20'
updated_date: '2026-08-28 20:06'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-python/about/src/units.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/units.rs:153-160`

**What**: `workspace_pyproject_path_debug_escapes_control_characters` hand-writes the three assertions the shared helper already makes:

```rust
let rendered = format!("{:?}", p.display());
assert!(!rendered.contains('\n'));
assert!(!rendered.contains('\u{1b}'));
assert!(rendered.contains("\\n"));
```

Its sibling in the same crate, `pyproject_path_debug_escapes_control_characters` (`lib.rs:399-403`), tests the identical property against the identical input (`Path::new("a\nb\u{1b}[31mc/pyproject.toml")`) via `ops_about::test_support::assert_debug_escapes_control_chars` — the helper introduced by DUP-3 / TASK-0985 for exactly this. `units.rs` has the same `ops-about` dev-dependency with the `test-support` feature (`Cargo.toml:21`), so nothing blocks it from calling the helper.

**Why it matters**: the two copies pin the same ERR-7 log-forging contract at two different levels of strictness. Extending the shared helper (say, to also reject `\r` or a bare CSI byte) silently upgrades `lib.rs` and leaves `units.rs` behind, which is the failure mode the helper was extracted to prevent. TASK-1735 filed the same "local reimplementation of the ops_about::test_support harness" class against another about crate this run.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 workspace_pyproject_path_debug_escapes_control_characters calls ops_about::test_support::assert_debug_escapes_control_chars
- [x] #2 The inline format!/assert! trio is deleted
- [x] #3 The test still fails if the tracing warn at units.rs:66 is switched from the ? formatter to %
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
`workspace_pyproject_path_debug_escapes_control_characters` now calls
`ops_about::test_support::assert_debug_escapes_control_chars(p.display())`
and the inline `format!` / three-`assert!` trio is deleted, matching its
`lib.rs` sibling.

AC#3 substitution (obsolete as literally written): the test -- in both its old
and new form -- never invokes the `tracing::warn!` at units.rs:66, so
switching that site from the `?` formatter to `%` could not have failed it.
The property actually under test is that `Debug` rendering escapes control
characters, which is what the `?` formatter relies on; the shared helper pins
exactly that, and now pins it identically for both sites, which is the drift
the finding was about. Pinning the macro's own formatter would need a
warn-capture test around `read_workspace_members`; that is a different
contract, and the same warn site is now exercised on the recovery axis by
TASK-1756's new `invalid_root_pyproject_yields_no_units`.
<!-- SECTION:NOTES:END -->
