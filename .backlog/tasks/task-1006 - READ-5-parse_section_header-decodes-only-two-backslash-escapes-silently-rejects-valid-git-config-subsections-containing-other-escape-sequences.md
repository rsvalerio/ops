---
id: TASK-1006
title: >-
  READ-5: parse_section_header decodes only two backslash escapes, silently
  rejects valid git-config subsections containing other escape sequences
status: Done
assignee: []
created_date: '2026-05-04 22:04'
updated_date: '2026-05-05 01:02'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:226-247`

**What**: `parse_section_header` decodes only `\\\\` and `\\"` escapes inside subsection names; any other backslash sequence (e.g., `\\n`, `\\t`, `\\u{…}`, or just a literal `\\` followed by a non-special char) returns `None`, dropping the entire section header. git-config's own grammar treats unknown `\\X` sequences inside double-quoted strings as a syntax error too, so the rejection is technically conformant — *but* the silent `None` return means a malformed `[remote "…"]` header silently disqualifies the whole section. Combined with `read_origin_url_from`'s "if !in_origin, skip" loop, a single typo in a subsection escape wipes out remote detection with no diagnostic.

**Why it matters**:
- Symptom asymmetry: the rest of the parser surfaces structural errors via `tracing::warn!` at the read site (read_origin_url, line 83-87 in config.rs) — but a header-decode failure produces no log because `parse_section_header` doesn't have access to the path. The user observes "remote URL not detected", greps logs, finds nothing.
- Comparison to `is_valid_host` / `is_valid_path_segment` in remote.rs: those *also* silently reject malformed inputs but are called from `parse_remote_url`, which is wrapped at line 67-83 of provider.rs in a `tracing::debug!` breadcrumb explaining "git remote URL did not match parse_remote_url shape". The section-header parser has no equivalent breadcrumb.
- The fix is small: have `parse_section_header` return `Result<…, ParseError>` (or just `Option` plus a separate `enum SectionError { UnknownEscape, MalformedQuotes, BareWordSubsection }`) so `read_origin_url_from` can log the specific reason. This costs ~20 lines and matches the diagnostic posture of the other parsers in the same module.

**Note**: low severity because the affected git-config files are user-authored and the cost of "remote URL appears as None" is recoverable by re-running with `RUST_LOG=ops_git=debug`. But that recovery hint is itself missing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_section_header surfaces the specific failure mode (unknown escape vs bareword subsection vs unbalanced quotes) so read_origin_url_from can log it.
- [ ] #2 read_origin_url_from emits a tracing::debug! breadcrumb when a section header is rejected, naming the failure category.
- [ ] #3 Test pins that an unknown escape in a subsection name produces a debug log rather than silent None.
<!-- AC:END -->
