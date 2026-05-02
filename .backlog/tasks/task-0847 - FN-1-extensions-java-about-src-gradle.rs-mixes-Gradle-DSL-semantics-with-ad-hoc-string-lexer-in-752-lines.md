---
id: TASK-0847
title: >-
  FN-1: extensions-java/about/src/gradle.rs mixes Gradle DSL semantics with
  ad-hoc string lexer in 752 lines
status: Done
assignee: []
created_date: '2026-05-02 09:16'
updated_date: '2026-05-02 14:14'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle.rs`

**What**: Production code is ~305 lines (rest is tests). Contains: provider impl, three top-level parsers (parse_gradle_settings/_properties/_build), six lexer helpers (extract_assignment, extract_bare_method, extract_quoted, extract_quoted_list, split_at_unquoted_close_paren, strip_*_comment), and the include-line state machine. Each function is itself OK in size, but the file mixes "Gradle DSL semantics" with "ad-hoc string lexer".

**Why it matters**: ARCH-1 module red flags trigger here (>300 production lines, 3 unrelated concerns colocated). Splitting gradle/lexer.rs from gradle/parse.rs makes both halves testable in isolation. The pom parser is already split this way.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 extract_quoted, extract_quoted_list, split_at_unquoted_close_paren, strip_trailing_comment, strip_properties_comment move to a sibling module (gradle/lexer.rs)
- [x] #2 parse_gradle_* functions stay in the high-level module; provider impl is a thin wrapper
- [x] #3 All existing tests still pass without modification (only use paths change)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Split extensions-java/about/src/gradle.rs (752 lines) into a directory module: gradle/lexer.rs (extract_quoted, extract_quoted_list, split_at_unquoted_close_paren, strip_trailing_comment, strip_properties_comment, all pub(super)), gradle/mod.rs (provider impl + parse_gradle_settings/_properties/_build + extract_assignment + extract_bare_method + parse_include_line), gradle/tests.rs (unchanged test bodies). All 41 gradle::* tests pass; ops verify clean. Mirrors the pom.rs split shape that the task pointed to as the precedent.
<!-- SECTION:NOTES:END -->
