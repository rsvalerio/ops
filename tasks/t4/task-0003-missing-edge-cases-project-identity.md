---
id: TASK-003
title: "Missing edge-case tests for project_identity formatting"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-8, low, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/project_identity.rs:168-243`
**Anchor**: `mod tests`
**Impact**: No tests for boundary conditions in formatting logic: pluralization of "file"/"files" (count=0, count=1), multiple-author comma joining, and `format_number(0)`.

**Notes**:
- TEST-8: Test boundary conditions — zero values, singular/plural, empty collections
- The production code handles file_count pluralization and author joining, but tests only cover "full" and "minimal" identity scenarios
- Fix: Add tests for `file_count == Some(1)` (singular "file"), `file_count == Some(0)`, `authors` with 2+ entries, and `format_number(0)`
