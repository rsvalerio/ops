---
id: TASK-0985
title: >-
  DUP-3: four near-identical *_path_debug_escapes_control_characters tests
  across about extensions
status: Triage
assignee: []
created_date: '2026-05-04 21:58'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions-node/about/src/package_json.rs:190-197` (`package_json_path_debug_escapes_control_characters`)
- `extensions-node/about/src/units.rs:256-263` (`workspace_member_globs_path_debug_escapes_control_characters`)
- `extensions-python/about/src/lib.rs:309-316` (`pyproject_path_debug_escapes_control_characters`)
- `extensions-go/about/src/modules.rs:222-238` (`directive_debug_escapes_control_characters`)

**What**: Four tests with the same body shape: build a `Path` (or `&str` for the go variant) containing `\n` and `\u{1b}[31m`, render it via `format!("{:?}", p.display())` (or `format!("{x:?}")`), then assert no raw `\n` / `\u{1b}` survives and `\\n` is present. Each test is bound to its module's specific tracing site (TASK-0818 / TASK-0930 / TASK-0809 sweep coverage), but they all assert the same property of the same formatter — `Debug` on `Path::display()` or `&str` escapes control characters.

**Why it matters**: The pin protects the ERR-7 sweep contract, but four copies that re-prove the same property of `std::fmt::Debug` make it tempting to delete one as redundant — the deletion would weaken sweep coverage without the reviewer noticing. Extract a single `assert_path_debug_escapes_control_characters(path)` helper in `ops_about::test_support` (or the existing `ops_about::manifest_io` test module) and have each provider's test call it with a path stitched onto its own filename, so all four sites still exist as guard tests but share the assertion logic.

**Candidate fix**: helper in `ops_about::test_support`:
```rust
pub fn assert_debug_escapes_control_chars<T: std::fmt::Debug>(value: T) {
    let r = format!("{value:?}");
    assert!(!r.contains('\n'));
    assert!(!r.contains('\u{1b}'));
    assert!(r.contains("\\n"));
}
```
Each provider test then becomes a 2-line call.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Helper assertion lives in a single shared location and is the only place that knows the Debug-escape guarantees
- [ ] #2 Each provider keeps its own #[test] entry point that calls the helper with its filename
- [ ] #3 Removing one provider's per-site test still leaves the property pinned somewhere
<!-- AC:END -->
