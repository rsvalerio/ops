---
id: TASK-0007
title: "Useless: theme list tests assert only is_ok(), never inspect output"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-11, medium, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/theme_cmd.rs:213-end`
**Anchor**: `fn run_theme_list_outputs_themes`, `fn run_theme_list_includes_builtin_themes`, `fn run_theme_list_marks_custom_themes`
**Impact**: All three `run_theme_list_*` tests assert only `is_ok()`. None captures or inspects the output. `run_theme_list_marks_custom_themes` claims to verify the `(custom)` marker but never actually checks for it. These tests create false confidence — they would pass even if the list output was empty or incorrect.

**Notes**:
Each test should capture the writer output and assert that: (1) expected theme names appear, (2) `(custom)` marker appears for custom themes, (3) builtin themes are listed. The `run_theme_list` function takes a `&mut impl Write` — pass a `Vec<u8>` and inspect its contents.
