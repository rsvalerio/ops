---
id: TASK-1622
title: >-
  DUP-1: ops-git config quoted-string escape decoder duplicated between
  strip_url_key and parse_section_header
status: Done
assignee:
  - TASK-1639
created_date: '2026-05-22 07:07'
updated_date: '2026-05-22 13:28'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:349-371` (`strip_url_key` quoted branch) and `extensions/git/src/config.rs:444-458` (`parse_section_header` subsection body).

**What**: Both functions decode the same git-config quoted-string grammar (`\\` → `\`, `\"` → `"`, every other backslash escape is malformed). They keep separate ~15-line decode loops with subtly different surfaces:

- `strip_url_key` reads quoted body char-by-char, looking for an unescaped closing `"`, returning `Option<Cow>`; an unbalanced or unknown-escape value collapses to `None`.
- `parse_section_header` pre-strips a trailing `"` via `strip_suffix`, then decodes the inner body, returning typed `SectionHeaderError::UnknownEscape` / `::UnterminatedEscape` on failure.

The two decoders are intentionally aligned (the `strip_url_key` docstring even references `parse_section_header`'s escape rules at line 350-351), so future tweaks — adding a third escape, tightening on lone `\`, hardening against an attacker-shaped value — have to be remembered in two places. The git-config(1) escape grammar is one rule; encoding it twice is the duplication.

**Why it matters**: Maintenance / drift risk. This is a security-sensitive parser (every git-config value flows into `RedactedUrl` and then JSON / About cards / logs). If one side gets a hardening fix and the other does not, the inconsistency is silent until an attacker crafts a value that pivots on the difference. The existing safety hardening trail (TASK-1102, TASK-1213, TASK-1238) shows this code path attracts repeated tightening — keeping the rule single-source minimises the chance a future tightening skips one call site.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a single quoted-git-config-value decoder helper (e.g. `decode_quoted_body`) honouring `\\` and `\"` escapes.
- [ ] #2 Both `strip_url_key` (quoted branch) and `parse_section_header` call the shared helper; existing typed-error / Option semantics preserved at the call sites.
- [ ] #3 All existing tests in extensions/git/src/config.rs still pass (quoted URL, escaped subsection, unbalanced quotes, unknown escapes).
<!-- AC:END -->
