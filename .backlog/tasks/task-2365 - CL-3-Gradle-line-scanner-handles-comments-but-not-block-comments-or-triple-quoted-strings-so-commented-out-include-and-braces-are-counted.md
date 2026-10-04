---
id: TASK-2365
title: 'CL-3: Gradle line scanner handles // comments but not /* */ block comments or triple-quoted strings, so commented-out include and braces are counted'
status: Done
assignee: []
created_date: '2026-10-04 14:10'
updated_date: '2026-10-04 16:14'
labels:
  - code-review-rust
  - CL
dependencies: []
parent_task_id: 'TASK-2424'
modified_files:
  - extensions-java/about/src/gradle/lexer.rs
  - extensions-java/about/src/gradle/mod.rs
priority: low
ordinal: 1000
dedup_key: 'CL-3:extensions-java/about/src/gradle/lexer.rs:scan_unquoted'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle/lexer.rs:60-110` (`scan_unquoted`, `brace_delta`, `strip_trailing_comment`), `extensions-java/about/src/gradle/mod.rs:96-122` (`parse_gradle_settings`), `mod.rs:200-210` (`parse_gradle_build`)

**What**: The scanner is line-oriented and only recognises `//` comments and single-line quoted strings. It assumes no construct spans lines. A `/* ... */` block (multi-line, with `include 'old'` or `rootProject.name = 'x'` or `description = ...` on a line inside it, or `{`/`}` inside it) is processed as live code, and a `"""` Kotlin/Groovy triple-quoted string spanning lines desynchronises quote state per line. The Maven sibling already strips multi-line `<!-- -->` (`strip_xml_comments`) so this is an unstated asymmetry.

**Why it matters**: A commented-out `include 'legacy'` inflates `module_count`/units, a commented `rootProject.name` can win first-writer-wins, and braces inside a block comment corrupt `depth`, so later top-level assignments are ignored or nested ones accepted. Block-commented old includes are common in settings.gradle files.

<!-- scan confidence: candidates to inspect --> no test in `gradle/tests.rs` exercises `/*`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Lines inside a multi-line /* ... */ block never contribute include, rootProject.name, description or brace depth
- [x] #2 Multi-line triple-quoted strings do not desynchronise quote or brace state, or the limitation is documented in the module docs
- [x] #3 Regression tests cover block-commented include and braces

<!-- AC:END -->
