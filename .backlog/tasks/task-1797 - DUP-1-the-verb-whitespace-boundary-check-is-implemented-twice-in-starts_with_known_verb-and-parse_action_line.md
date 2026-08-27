---
id: TASK-1797
title: >-
  DUP-1: the verb + whitespace-boundary check is implemented twice, in
  starts_with_known_verb and parse_action_line
status: Triage
assignee: []
created_date: '2026-08-27 11:24'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:305-308` and `:334-342`

**What**: The same predicate — "does `line` start with one of `ACTION_PREFIXES`
followed by whitespace or end-of-string" — is written twice, in two functions,
each carrying its own copy of the `PATTERN-1 / TASK-1030` comment that
explains it.

`starts_with_known_verb`:

```rust
let matches_verb = ACTION_PREFIXES.iter().any(|(prefix, _, _)| {
    line.strip_prefix(prefix)
        .is_some_and(|rest| rest.chars().next().is_none_or(char::is_whitespace))
});
```

`parse_action_line`:

```rust
let Some(rest) = line.strip_prefix(prefix) else { continue };
if !rest.chars().next().is_none_or(char::is_whitespace) {
    continue;
}
```

**Why it matters**: these two must agree or the crate's drift-detection
contract breaks in one of two directions — a line the parser accepts but the
verb check rejects, or a line the parser rejects while the verb check stays
silent, which is precisely the "silently disappears from the count headline"
failure TASK-0472 installed the warn to prevent. They already diverged once:
TASK-1030 had to patch the boundary check into both sites separately. A single
`fn match_verb(line: &str) -> Option<(&'static str, UpdateAction, VersionRole, &str)>`
returning the matched entry plus the remainder would let
`starts_with_known_verb` be `match_verb(line).is_some() && has_version_token`
and give `parse_action_line` its `rest` from the same call — one definition,
one comment, no drift surface.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The verb + whitespace-boundary match exists in exactly one function, and both starts_with_known_verb and parse_action_line consume it
- [ ] #2 The PATTERN-1/TASK-1030 rationale comment appears once, next to the single implementation
- [ ] #3 Existing tests (verb_prefix_requires_whitespace_boundary and the drift-warn tests) still pass unchanged
<!-- AC:END -->
