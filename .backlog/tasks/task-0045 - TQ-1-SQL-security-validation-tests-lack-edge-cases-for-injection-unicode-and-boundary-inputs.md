---
id: TASK-0045
title: >-
  TQ-1: SQL security validation tests lack edge cases for injection, unicode,
  and boundary inputs
status: Done
assignee: []
created_date: '2026-04-14 20:22'
updated_date: '2026-04-15 09:56'
labels:
  - rust-test-quality
  - TestGap
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions/duckdb/src/sql.rs — The SQL security validation functions (escape_sql_string, validate_path_chars, validate_no_traversal, prepare_path_for_sql) are tested with basic cases but missing critical edge cases: (1) SQL injection patterns beyond semicolons (UNION SELECT, comment injection --, quote variants), (2) Unicode/internationalization (CJK chars, emoji, combining diacritics, zero-width chars), (3) control characters beyond null (\x01, \x1F, \x7F), (4) empty string inputs, (5) combined attack vectors (null byte + semicolon), (6) boundary conditions (very long paths). Current coverage is ~65% of attack surface. Rules: TEST-6, TEST-8.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 SQL injection payloads (UNION, --, quote escapes) are tested
- [ ] #2 Unicode and control character handling is tested
- [ ] #3 Empty and boundary inputs are tested
<!-- AC:END -->
