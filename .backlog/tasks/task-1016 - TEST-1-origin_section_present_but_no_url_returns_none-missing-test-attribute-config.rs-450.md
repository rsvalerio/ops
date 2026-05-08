---
id: TASK-1016
title: >-
  TEST-1: origin_section_present_but_no_url_returns_none missing #[test]
  attribute (config.rs:450)
status: Done
assignee: []
created_date: '2026-05-07 20:21'
updated_date: '2026-05-08 06:23'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:450`

**What**: The function `origin_section_present_but_no_url_returns_none` (lines 450-453) is declared inside the `#[cfg(test)] mod tests` module but lacks a `#[test]` attribute. The preceding doc-comment at lines 412-416 (referencing TASK-0966) advertises it as a test, but the `#[test]` attribute that should sit just below the doc-comment is consumed instead by the *next* function (`parse_section_header_unknown_escape_returns_typed_error`, lines 423-431) — which now has TWO stacked `#[test]` attributes (lines 417 and 423). The result: the TASK-0966 contract test never runs; the `parse_section_header_unknown_escape_returns_typed_error` test runs once instead of twice (Rust accepts duplicate `#[test]` quietly).

Concretely, lines 416-424 read:

```
/// source — call-site presence is guarded by code review.
#[test]                                                         // line 417 — orphaned, attached to next fn
/// READ-5 / TASK-1006: a malformed escape in a `[remote "…"]` header
/// returns a typed `SectionHeaderError` rather than collapsing the
/// whole section silently. The behaviour-pinning assertion is that
/// `parse_section_header` reports a typed error so `is_origin_header`
/// can log a debug breadcrumb naming the failure category.
#[test]                                                         // line 423
fn parse_section_header_unknown_escape_returns_typed_error() {
```

And then at line 450 the helper that *should* be the first `#[test]` body has no attribute:

```
fn origin_section_present_but_no_url_returns_none() {
    let cfg = "[remote \"origin\"]\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n";
    assert!(read_origin_url_from(cfg).is_none());
}
```

`cargo test` therefore silently skips the TASK-0966 regression assertion. A follow-up commit that breaks `read_origin_url_from`'s "section-present-but-no-url returns None" contract will not fail CI.

**Why it matters**: This is a real loss of test coverage for a security-adjacent contract — `read_origin_url_from` returns the **last** url= entry per git-config last-wins semantics, and TASK-0966 specifically guards the "section present but every url= line was malformed / empty" breadcrumb. With the test silently inert, a regression to "first match wins" or "panic on empty" would not be caught until production. The fix is one line: add `#[test]` above `fn origin_section_present_but_no_url_returns_none()` at line 450 and remove the duplicate `#[test]` at line 417 (or 423).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add a single #[test] attribute directly above fn origin_section_present_but_no_url_returns_none() at extensions/git/src/config.rs:450
- [x] #2 Remove the duplicate #[test] attribute orphaned at extensions/git/src/config.rs:417 (the doc comment above it belongs to the test that should be at line 450)
- [x] #3 Verify with cargo test -p ops_git that the test count for the config.rs tests module increases by one
- [x] #4 Confirm origin_section_present_but_no_url_returns_none asserts read_origin_url_from(...) returns None when [remote "origin"] is present but contains no url= line
<!-- AC:END -->
