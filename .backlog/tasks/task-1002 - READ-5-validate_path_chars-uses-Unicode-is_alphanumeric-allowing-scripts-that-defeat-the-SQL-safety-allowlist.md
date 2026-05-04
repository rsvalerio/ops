---
id: TASK-1002
title: >-
  READ-5: validate_path_chars uses Unicode is_alphanumeric, allowing scripts
  that defeat the SQL-safety allowlist
status: Triage
assignee: []
created_date: '2026-05-04 22:03'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/validation.rs:194-224`

**What**: `validate_path_chars` accepts every char satisfying `ch.is_alphanumeric()` — the Unicode-aware predicate, not `is_ascii_alphanumeric()`. That admits the entire Unicode general categories L* (Letter, ~140k codepoints) and Nd (decimal digit, ~600 codepoints) plus several other Number subcategories. The accompanying tests pin rejection of zero-width space (U+200B is category Cf, not L*), but a bidirectional override (U+202E, RIGHT-TO-LEFT OVERRIDE) is also Cf and is correctly rejected — *however*, characters in the Latin Extended / Cyrillic / Greek scripts that visually mimic ASCII are accepted: e.g., Cyrillic small `а` (U+0430) flows through validation but is a different codepoint from ASCII `a`.

**Why it matters**:
- The validator is used for paths that get bound as DuckDB parameters (`query_crate_coverage::workspace_root`) and for paths that get interpolated into `read_json_auto('…')` SQL via `prepare_path_for_sql`. The latter then runs through `escape_sql_string`, which only neutralises `'`, `\\0`, and `\\` — so a homoglyph attack can produce a path string that matches none of the validator's blocklist *and* none of the escaper's, yet refers (post-tool-rendering) to an entirely different filesystem location than what the user typed in their `.ops.toml`.
- More directly: combining marks (e.g., U+0301 COMBINING ACUTE ACCENT, category Mn) are rejected by the current code, but ligatures (U+FB00 `ﬀ`, category Ll) pass — a hostile workspace name `ﬀ.json` survives validation and lands in DuckDB SQL where the rendering depends on the destination terminal / log surface.
- Path components on real filesystems are byte sequences; the SQL-safety contract should be over the byte representation, not over Unicode general categories. POSIX paths only need ASCII alnum + `.`/`-`/`_`/`/`/space; Windows adds `:` and `\\` (already gated). The current `is_alphanumeric()` widens the allowlist to ~140k codepoints with no security gain.

**Recommended fix**: switch `is_alphanumeric()` → `is_ascii_alphanumeric()`. The codebase's typical project-name / language-name / file-name set is already ASCII; widening was probably copy-paste convenience, not policy. If non-ASCII path support is a real requirement, document the allowed scripts explicitly and reject mixed-script identifiers.

**Cross-check**: SEC-12 / TASK-0729 already established defense-in-depth around interpolated paths; this is the missing rung. The existing tests at lines 484-500 test only Latin / control / zero-width inputs — none cover homoglyphs or ligatures.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 validate_path_chars rejects non-ASCII alphabetic codepoints (e.g., Cyrillic 'а' U+0430, Latin ligature 'ﬀ' U+FB00).
- [ ] #2 Existing accepted-path tests (slash, dot, dash, underscore, space) still pass.
- [ ] #3 Decision documented in the rustdoc whether non-ASCII identifiers are admissible at all (with rationale linked to SEC-12).
<!-- AC:END -->
