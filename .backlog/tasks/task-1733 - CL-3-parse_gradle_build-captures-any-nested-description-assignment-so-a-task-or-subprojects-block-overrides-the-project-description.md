---
id: TASK-1733
title: >-
  CL-3: parse_gradle_build captures any nested description assignment, so a task
  or subprojects block overrides the project description
status: Triage
assignee: []
created_date: '2026-08-27 11:12'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-java/about/src/gradle/mod.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle/mod.rs:120` (`parse_gradle_build`)

**What**: `parse_gradle_build` scans `build.gradle` / `build.gradle.kts` line by line with no brace-depth tracking, and assigns `description = Some(val)` unconditionally on every match — so the **last** `description` assignment anywhere in the file wins, at any nesting level.

Real-world `build.gradle` shapes that hijack the project description:

```groovy
description = 'The real project description'

tasks.register('generateDocs') {
    description = 'Generates the API docs'   // <- wins; becomes the project description
}
```

```kotlin
description = "The real project description"

subprojects {
    description = "A subproject"             // <- wins
}
```

Task blocks that set `description` are idiomatic Gradle (it is a standard `Task` property, and the Gradle docs recommend setting it on every custom task), so this is the common case rather than an exotic one. `extract_bare_method` widens the surface further: a Groovy `description 'text'` line inside any block matches too.

Two contributing design issues in the same function:

- The precondition "a `description` assignment at column 0 of the file belongs to the root project" is implicit — nothing in the code or the doc comment states it, and nothing enforces it (CL-3).
- The last-writer-wins policy here is the opposite of the sibling Maven parser, which routes every field through `try_set_once` (first-writer-wins, `maven/pom.rs:255`). Two parsers in the same crate resolving duplicates in opposite directions is a READ-6 inconsistency; whichever policy is chosen, both should state it.

`parse_gradle_settings` has a milder version of the same shape for `rootProject.name` (line 84, also last-wins), though a nested `rootProject.name` is far less likely.

**Why it matters**: `ops about` renders a task's description as the project's, with no error and no log line — a silently wrong field the user has no way to trace back to the parser.

**Suggested fix**: track brace depth across lines (the lexer module already owns the quote-aware primitives needed to avoid counting braces inside strings) and only accept `description` at depth 0; or, at minimum, take the *first* depth-0 assignment via a `try_set_once`-style helper shared with the Maven parser and document the policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_gradle_build ignores description assignments that occur inside a nested block (task, tasks.register, subprojects, allprojects) and only accepts a top-level project description
- [ ] #2 Duplicate-resolution policy (first-wins vs last-wins) is stated in the doc comment and matches the Maven parser's try_set_once policy, or the divergence is documented with a reason
- [ ] #3 Tests cover: root description followed by a task block description (root wins), a task block description with no root description (yields None), and the Groovy bare-method form inside a block
- [ ] #4 Existing gradle tests in extensions-java/about/src/gradle/tests.rs still pass
<!-- AC:END -->
