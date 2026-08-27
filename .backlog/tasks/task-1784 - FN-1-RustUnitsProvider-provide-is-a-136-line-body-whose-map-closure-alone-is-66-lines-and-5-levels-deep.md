---
id: TASK-1784
title: >-
  FN-1: RustUnitsProvider::provide is a 136-line body whose map closure alone is
  66 lines and 5 levels deep
status: Triage
assignee: []
created_date: '2026-08-27 11:23'
labels:
  - code-review-rust
  - structure
dependencies: []
modified_files:
  - extensions-rust/about/src/units.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:36-171` (`RustUnitsProvider::provide`), with the offending closure at `:102-167`

**What**: `provide` runs 136 lines, ~65 of them executable, and mixes four concerns: manifest loading with error logging (`:39-51`), a `DuckDb` dep-count query (`:59-67`), canonical-path cache priming (`:73`), and a single iterator chain (`:80-168`) that does the rest.

The chain is where the cost concentrates. Its `.map()` closure spans `:102-167` — 66 lines, longer than FN-1's whole-function budget — and reaches five levels of nesting: `fn` → `.map(closure)` → `if pkg_name.is_some()` → `if let Some(key)` → `if lookup.is_none()`, exceeding FN-2's limit of four. Inside it: a path join, a struct destructure, a canonical-path lookup with an fs-canonicalize fallback, a UTF-8 validity branch with two distinct `tracing::debug!` breadcrumbs, a third breadcrumb for the missing-package-name case, a display-name format, and a five-field `ProjectUnit` assembly.

The depth is already being paid for explicitly. `:133` carries an `#[allow(clippy::option_if_let_else)]` whose stated justification is the nesting itself — *"`map_or_else` would nest two multi-line closures inside an already deeply indented `map` body."* That is the lint reporting the FN-1/FN-2 problem and the allow suppressing the report rather than the cause. It is also a bare `#[allow]` with no `reason = ` field, where READ-10 asks for `#[expect(..., reason = "…")]` so the suppression removes itself once the body is extracted.

The `.filter()` at `:83-101` has the same shape on a smaller scale: 19 lines of which 13 are a comment block, wrapping a single predicate call plus a warn.

**Why it matters**: the dep-count resolution logic (canonical path → `&str` key → `HashMap` lookup, with three separate diagnostic branches) is the part of this crate most likely to need changing — it has already been reworked by TASK-1253, TASK-1569 and TASK-1570 — and it is currently unreachable from a test except by driving the whole provider with a live `DuckDb`. Extracted as `fn resolve_dep_count(member: &str, canonical: &Path, dep_counts: &HashMap<String, i64>) -> Option<i64>`, it becomes directly unit-testable, the `option_if_let_else` allow disappears, and the map closure drops to roughly a dozen lines.

**Fix direction**: extract the dep-count resolution and the `ProjectUnit` assembly into named functions; move the filter predicate's warn into a small named helper shared with `resolve_crate_display_name` (`:243-250`), which open-codes the same reject-and-warn pair.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 RustUnitsProvider::provide is under 50 lines and its map closure is under 20
- [ ] #2 The canonical-path to dep_count resolution (including the non-UTF-8 and missing-package-name breadcrumbs) is a named function with its own unit tests, callable without a live DuckDb
- [ ] #3 The bare #[allow(clippy::option_if_let_else)] at units.rs:133 is removed rather than relocated; if any suppression remains it uses #[expect(..., reason = "…")] per READ-10
- [ ] #4 Nesting inside provide is at most 4 levels
- [ ] #5 Existing units.rs provider tests pass unmodified
<!-- AC:END -->
