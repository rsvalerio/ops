---
id: TASK-1632
title: >-
  API-5: pure SQL helpers escape_sql_string and sanitize_path_for_sql lack
  must_use
status: Done
assignee:
  - TASK-1640
created_date: '2026-05-22 07:17'
updated_date: '2026-05-22 13:43'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`extensions/duckdb/src/sql/validation.rs:209\` and \`extensions/duckdb/src/sql/validation.rs:221\`

**What**: \`pub fn escape_sql_string(s: &str) -> String\` and \`pub fn sanitize_path_for_sql(path: &str) -> String\` are pure transformations whose return value is the whole reason to call them. Today neither carries \`#[must_use]\`, so a caller writing \`escape_sql_string(name);\` (forgetting to bind the result and interpolating the raw \`name\` instead) compiles silently — the exact regression the SEC-12 defence-in-depth helpers are meant to prevent.

Sister helpers in the same file have analogous shape: \`TableName::as_str\`, \`TableName::quoted\` already carry \`#[must_use]\`, but the bare \`fn\`s do not.

**Why it matters**: \`#[must_use]\` is the cheapest defence against the "forgot to use the sanitized value" footgun. The SEC-12 model relies on the validation+escape pair being applied at every interpolation site; an unused return value is one drift away from a raw-string interpolation in a future hand-edit. \`Result<_, SqlError>\`-returning helpers (\`validate_identifier\`, \`quoted_ident\`, \`prepare_path_for_sql\`, \`validate_extra_opts\`, \`validate_path_chars\`, \`validate_no_traversal\`) are already covered by the must_use lint on \`Result\`; this finding only covers the bare-String returns.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 escape_sql_string carries #[must_use] with a 1-line message referencing the SEC-12 contract
- [ ] #2 sanitize_path_for_sql carries #[must_use] with a 1-line message referencing the SEC-12 contract
- [ ] #3 Build passes with -D unused_must_use
<!-- AC:END -->
