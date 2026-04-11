---
id: TASK-026
title: "AboutCard::from_identity exceeds 50 lines with CC≈8"
status: To Do
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-code-quality, CQ, FN-1, FN-6, low, effort-S, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/project_identity.rs:63-113`
**Anchor**: `fn from_identity`
**Impact**: AboutCard::from_identity is 51 lines with cyclomatic complexity ≈8 due to 6 sequential if-let blocks accumulating optional fields. While all conditions are sequential (non-nested, depth ≤2), the function operates at mixed abstraction levels — field extraction and formatting interleaved.

**Notes**:
Each if-let block adds an optional line to the card. The logic is sequential and simple per-branch, so cognitive load is moderate despite CC≈8. Refactoring: extract field formatting into small helpers (e.g., `format_version_line()`, `format_language_line()`) or use an iterator over optional fields with `filter_map`.
