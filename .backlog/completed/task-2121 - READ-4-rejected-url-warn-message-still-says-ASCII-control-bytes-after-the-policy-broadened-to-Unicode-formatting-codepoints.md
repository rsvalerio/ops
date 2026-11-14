---
id: TASK-2121
title: 'READ-4: rejected-url warn message still says "ASCII control bytes" after the policy broadened to Unicode formatting codepoints'
status: Done
assignee: []
created_date: '2026-09-08 06:54'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2236'
modified_files:
  - extensions/git/src/config.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:311-325` (`parse_origin_url_inner`), and the inline comment at `:293-296`

**What**: `RedactedUrl::redact` rejects on two predicates — `is_ascii_control_byte` **and** `is_unicode_format_or_separator` (SEC-2 / TASK-1238). The caller's rejection breadcrumb still reports only the first:

```
"SEC-2 / TASK-1215: dropped origin url= line(s) containing ASCII control bytes"
```

and the comment above the counter likewise says "a `url = ...` line with embedded ASCII control bytes (raw newline, ANSI escape, NUL)". A remote dropped for a U+202E RIGHT-TO-LEFT OVERRIDE or a U+200B zero-width space logs as an ASCII control-byte problem.

**Why it matters**: This warn line is the operator's only signal for "branch shows but remote_url is stale/None" (its stated purpose in TASK-1215). Naming the wrong cause sends whoever is debugging to `hexdump -C .git/config | grep` for C0 bytes, finds none, and leaves the actual bidi/zero-width codepoint undiagnosed — the same misdirection READ-4 / TASK-1878 was filed for on this crate's doc strings.

<!-- Reviewer note: cheapest fix is a message that names the whole policy ("control or Unicode formatting/separator codepoint"), matching the wording read_head_branch already uses at config.rs:663. Better: have redact() return which predicate fired and log that. -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The rejected-url warn message and the adjacent comment describe the full rejection policy, not only ASCII control bytes
- [ ] #2 Wording is consistent with the read_head_branch rejection warn in the same file
<!-- AC:END -->
