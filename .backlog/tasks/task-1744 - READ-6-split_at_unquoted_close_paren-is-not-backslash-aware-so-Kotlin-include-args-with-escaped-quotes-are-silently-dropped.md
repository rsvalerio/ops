---
id: TASK-1744
title: >-
  READ-6: split_at_unquoted_close_paren is not backslash-aware, so Kotlin
  include() args with escaped quotes are silently dropped
status: Done
assignee:
  - TASK-1990
created_date: '2026-08-27 11:13'
updated_date: '2026-08-28 15:48'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions-java/about/src/gradle/lexer.rs
  - extensions-java/about/src/gradle/mod.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle/lexer.rs:104` (`split_at_unquoted_close_paren`)

**What**: The lexer holds two quote scanners with different rules. `find_unescaped` (line 32) was deliberately made backslash-aware (PATTERN-1 / TASK-1047) so Groovy/Kotlin string literals containing escaped quotes round-trip. `split_at_unquoted_close_paren`, which runs *first* on the Kotlin `include(...)` path, tracks quotes with a plain byte scan and no escape handling:

```rust
Some(q) => { if b == q { quote = None; } }
```

So an escaped quote closes the string early, the following `)` is treated as the structural close paren, and the truncated fragment then fails `find_unescaped` and is discarded.

Reproduced against a verbatim copy of `lexer.rs` plus `parse_include_line`:

```
input : include("legacy\")module")
Kotlin include()      -> []                       # module silently dropped
Groovy bare include   -> ["legacy\\')module"]     # same value, handled correctly
extract_quoted alone  -> Some("legacy\\\")module") # escape-aware path is fine
```

The Groovy and Kotlin spellings of the same `include` therefore disagree, and the Kotlin one loses the entry — `module_count` in the About card is silently short by one, with only a `tracing::debug!` line to show for it.

A second, smaller instance of the same inconsistency sits alongside it: `parse_include_line` (`gradle/mod.rs:153`) pre-strips `// …` via `strip_trailing_comment` before any quote parsing, and `extract_quoted_list` (`lexer.rs:54`) strips it *again*. `extract_bare_method` (`gradle/mod.rs:196`) carries an explicit doc comment (READ-2 / TASK-0647) explaining why pre-stripping `//` is wrong — it chops quoted values containing `//`. The include path does the thing that comment forbids, and the double strip means an include argument containing `//` yields `[]`.

**Why it matters**: READ-6 — three quote/comment scanners in one small module with three different sets of rules. The escape-awareness fix from TASK-1047 was applied to one of them, so the module now behaves differently depending on which spelling of `include` the user wrote. Low impact in absolute terms (escaped quotes in Gradle module paths are rare), but the divergence is the kind that regrows every time one scanner is touched.

**Fix**: make `split_at_unquoted_close_paren` skip backslash-escaped characters the same way `find_unescaped` does — ideally by sharing one escape-aware scan primitive — and drop the redundant `strip_trailing_comment` call in `parse_include_line` so comment stripping happens in exactly one place.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 split_at_unquoted_close_paren skips backslash-escaped characters inside quoted spans, sharing the escape logic with find_unescaped rather than duplicating it
- [x] #2 include("legacy\")module") yields the same single entry as the equivalent Groovy bare-include form
- [x] #3 strip_trailing_comment is applied at exactly one point on the include path (not once in parse_include_line and again in extract_quoted_list)
- [x] #4 Existing gradle lexer and include tests still pass, and new tests pin the Kotlin/Groovy parity for an escaped-quote argument
<!-- AC:END -->
