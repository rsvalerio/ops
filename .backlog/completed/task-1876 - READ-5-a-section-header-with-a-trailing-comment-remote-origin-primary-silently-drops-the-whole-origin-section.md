---
id: TASK-1876
title: >-
  READ-5: a section header with a trailing comment ([remote "origin"] # primary)
  silently drops the whole origin section
status: Done
assignee:
  - TASK-2007
created_date: '2026-08-27 15:31'
updated_date: '2026-08-28 23:27'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/git/src/config.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:355-378` (`strip_url_key`), `config.rs:380-393` (`is_origin_header`), `config.rs:83-110` (`parse_section_header`)

**What**: `parse_section_header` requires the trimmed line to be *exactly* `[…]` (`strip_prefix('[')` + `strip_suffix(']')`, then the decoded body must consume every remaining byte). git-config(1) is looser in two ways this scanner does not model:

1. **Trailing comment after the header.** git's parser treats `#` / `;` as starting a comment anywhere outside a quoted value, so `[remote "origin"] # primary` and `[remote "origin"] ; upstream mirror` are valid and git resolves `remote.origin.url` from them. Here `strip_suffix(']')` fails → `SectionHeaderError::NotASectionHeader` → `is_origin_header` returns false → `in_origin` is set false → **every** `url = …` line in that section is skipped and `git_info` reports `remote_url: None` for a perfectly ordinary repository. The comment-stripping added by READ-2 / TASK-0726 covers value lines only; header lines never see it.
2. **Key on the header line.** git accepts `[remote "origin"] url = https://…`. Same outcome: the header is unparseable and the value is lost.

Both cases also miss the debug breadcrumb's intent — `is_origin_header` does log at debug for headers starting with `r`/`R`, so the failure is at least discoverable, but it is reported as "rejected section header that looks like remote.*" rather than "we do not implement trailing comments".

The parser's documented limitation list (`config.rs:222-241`) names `insteadOf`, continuation lines, escaped quotes, and `include.path` as unsupported. Header comments are not on that list, so this is an undocumented divergence rather than an accepted one.

**Why it matters**: silent, total loss of repository identity for a config shape git itself accepts — the same class as TASK-0403 (case-sensitive section match) and TASK-0726 (inline value comments), both of which were fixed. `git_info.remote_url` is what `extensions/about/src/identity.rs:149` and `extensions-rust/about/src/identity/resolver.rs:40` fall back to, so the whole identity chain degrades with no user-visible reason.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 a trailing '#' or ';' comment after a section header is stripped before parse_section_header runs, so [remote "origin"] # primary is recognised as the origin section
- [x] #2 the stripping does not break a subsection name that legitimately contains '#' or ';' inside quotes
- [x] #3 either the header-line key form ([remote "origin"] url = …) is supported, or it is added to the documented limitation list in the read_origin_url_from doc comment
- [x] #4 unit tests cover header + '#' comment, header + ';' comment, and a quoted subsection containing a ';'
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#3 satisfied by the documented-limitation route: the header-line key form ([remote "origin"] url = ...) is listed in read_origin_url_from's limitation list and pinned by header_line_key_form_remains_unsupported.
<!-- SECTION:NOTES:END -->
