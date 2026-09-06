---
id: TASK-1521
title: >-
  FN-1: deps format_severity_section spans 61 lines and branches on Option<id>
  per-row, duplicating the row-emit path
status: Done
assignee:
  - TASK-1646
created_date: '2026-05-19 07:28'
updated_date: '2026-05-25 17:58'
labels:
  - code-review-rust
  - fn
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:220-280` (`format_severity_section`)

**What**: `format_severity_section` is 61 lines (220-280). After the early-return / width passes (229-249), the per-row loop runs an `if let Some(id) = row.id` branch that emits two near-identical `writeln!` shapes — one with the id column and one without (lines 250-274). Both branches share `colorize_severity(icon, row.severity)`, the `dim(row.message)` shaping, and the `pkg_w` padding logic — the only structural difference is the additional `{:<id_w$}` column.

This is the second source of "Some-vs-None" boilerplate paired with the `AdvisoryRow.id: Option<&str>` field. Either:

- separate the entries into two row-emit helpers (`emit_row_with_id`, `emit_row_without_id`), or
- swap `Option<&str>` for an enum that owns the formatting decision (`enum RowShape { Advisory { id }, Plain }`).

Either path drops the function below 50 lines and removes the asymmetric writeln pair.

**Why it matters**: FN-1 (function > 50 lines). Compound with PATTERN-1: the `Option<&str>` id field is a flag-shaped seam where a `Vec<RowShape>` enum or two specialised helpers would express the layout intent at compile time.

<!-- scan confidence: function length verified — 61 lines per `awk '/^fn format_severity_section/,/^}/' | wc -l` -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 format_severity_section reduced below 50 lines by extracting the with-id / without-id writeln pair into a named helper or a typed RowShape enum
- [ ] #2 AdvisoryRow no longer exposes an Option<&str> id field at the per-row branch level — either the helper or the enum encodes the layout decision
- [ ] #3 advisory_section_renders_id_column, deny_section_omits_id_column, and format_report_advisories_mixed_severities continue to pass with no expected-output diff
<!-- AC:END -->
