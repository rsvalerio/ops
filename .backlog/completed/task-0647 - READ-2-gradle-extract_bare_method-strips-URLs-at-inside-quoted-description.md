---
id: TASK-0647
title: 'READ-2: gradle extract_bare_method strips URLs at // inside quoted description'
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 04:52'
updated_date: '2026-04-30 07:47'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle.rs:174` (and `strip_trailing_comment` at `:230`)

**What**: `extract_bare_method` calls `strip_trailing_comment(rest)` BEFORE `extract_quoted`. `strip_trailing_comment` is a textual `find("//")` with no awareness of quote state, so a URL inside the value (`description "Visit https://example.com"`) is chopped at the `//` of `https://`, leaving `"Visit https:` — `extract_quoted` then fails to find a closing quote and the description is silently dropped to `None`. The same shape parses correctly via the assignment form (`description = "..."`) because `extract_assignment` does NOT pre-strip comments — it relies on the closing quote of `extract_quoted` to terminate the value, which is the correct order.

**Why it matters**: Real `build.gradle` files use the bare-method form for `description` (the existing test `parse_gradle_build_bare_method` at `:531` pins it) and a description containing a URL is common. The provider silently produces an empty description for these projects rather than the real value. Order is wrong: extract the quoted span first, then strip a trailing comment from any post-quote remainder if needed (matching the assignment-form invariant pinned by `parse_gradle_settings_root_project_name_with_inline_comment`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 extract_bare_method preserves `//` characters that appear inside the quoted value
- [x] #2 Regression test added: `description "see https://example.com"` round-trips through parse_gradle_build with the URL intact
- [x] #3 Trailing `// comment` after the closing quote is still ignored (parity with the assignment form)
<!-- AC:END -->
