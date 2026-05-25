---
id: TASK-1047
title: >-
  PATTERN-1: gradle lexer extract_quoted does not honor backslash escapes;
  values with embedded quotes truncate silently
status: Done
assignee: []
created_date: '2026-05-07 20:54'
updated_date: '2026-05-07 23:11'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `/Users/rsvaleri/projects/ops/extensions-java/about/src/gradle/lexer.rs:10-21`

**What**:
`extract_quoted` finds the first occurrence of the opening quote character via `rest.find(open)?` and returns the slice up to it. The loop is escape-blind: a Groovy / Kotlin string literal that contains a backslash-escaped quote (`"see \"v2\" docs"`, `'O\'Brien'`) terminates extraction at the *first inner* quote and silently drops the rest of the value.

In practice this falls out of:

- `description "see \"v2\" docs"` in `build.gradle` → the about card's description is rendered as `see \` with the rest of the string discarded.
- `rootProject.name = 'O\'Brien'` in `settings.gradle` → name becomes `O\` instead of `O'Brien`.

`extract_quoted_list` inherits the same behavior because its inner scan is essentially the same `find(quote)` shape, so an `include 'a\'b', 'c'` line would push only `a\` and then the lexer-bail diagnostic — the second token never lands.

**Why it matters**:
The about card is the user-visible artefact for a Gradle project. Silently truncating description / rootProject.name when the manifest contains a perfectly legal escaped quote produces a wrong identity rather than a missing one — the operator has no signal that something went wrong, so they cannot file a parse-error bug. The fix is contained: walk byte-by-byte, treat `\\` as a literal, and bail on the first *unescaped* matching quote. Real-world Gradle uses double-quoted strings for description in particular and embedded single quotes in apostrophed names are not rare.

This complements TASK-0630 (unbalanced-quote warning) and TASK-0619 (paren handling) which both made the lexer more honest about what it could and couldn't parse, but neither fixed the escape-blindness inside legitimately-quoted runs.

Suggested fix:
- Add a 5-line backslash-aware scanner: iterate `rest.char_indices()` keeping a `prev_was_backslash` flag; the closing quote is the first match where the flag is false. Apply the same shape to `extract_quoted_list`.
- Alternatively: if a backslash run is encountered, fall back to an explicit "give up and bail at debug" path so the operator at least sees the diagnostic rather than a truncated value.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extract_quoted returns the full content of "see \"v2\" docs" rather than "see \\" — escapes are honored or the lexer bails with a diagnostic
- [ ] #2 extract_quoted_list with input `'a\'b', 'c'` either pushes both tokens or emits a debug-level bail breadcrumb identifying the offending line
<!-- AC:END -->
