---
id: TASK-1491
title: >-
  PATTERN-1: classify_member uses redundant star_idx.expect after explicit
  is_none check
status: Done
assignee:
  - TASK-1578
created_date: '2026-05-18 17:00'
updated_date: '2026-05-19 18:48'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:474-492`

**What**: `classify_member` calls `member.find('*')` (returns `Option<usize>`), branches on `star_idx.is_none()` to return early, then immediately re-unwraps with `let idx = star_idx.expect("star_idx checked above");`.

```rust
let star_idx = member.find('*');
if star_idx.is_none() {
    if contains_unsupported_glob_meta(member) { return MemberShape::Unsupported; }
    return MemberShape::Literal;
}
let idx = star_idx.expect("star_idx checked above");
```

A `match`, `if let Some(idx) = star_idx`, or `let Some(idx) = star_idx else { ... }` would express the same control flow without the unwrap and self-explanatory `expect` string.

**Why it matters**: ERR-5 style: a provably-infallible `.expect()` is a readability and lint smell — readers must verify the "checked above" claim mentally, future edits to the early-return arm risk invalidating the invariant silently, and clippy lints (`unnecessary_unwrap`) flag the shape. Cost is one small refactor; benefit is the function reads as a flat dispatch with the compiler enforcing exhaustiveness.

Suggested shape:
```rust
let Some(idx) = member.find('*') else {
    return if contains_unsupported_glob_meta(member) {
        MemberShape::Unsupported
    } else {
        MemberShape::Literal
    };
};
if is_unsupported_glob(member, idx) { return MemberShape::Unsupported; }
MemberShape::Glob { prefix: &member[..idx] }
```
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 classify_member uses pattern matching (let-else, if let, or match) to bind `idx` instead of `expect` after an `is_none` check
- [ ] #2 No `star_idx.expect(...)` call remains in classify_member
- [ ] #3 Existing classify_member behaviour preserved (all current tests pass, including any literal/glob/unsupported edge cases)
<!-- AC:END -->
