---
id: TASK-027
title: "Stack::detect reaches 5-level nesting in iterator chain"
status: To Do
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-code-quality, CQ, FN-2, low, effort-S, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/stack.rs:57-82`
**Anchor**: `fn detect`
**Impact**: Stack::detect uses a loop with `DETECT_ORDER.iter().find(|s| s.manifest_files().iter().any(|f| current.join(f).exists()))` reaching 5 levels of nesting. While the search algorithm justifies some depth, the dense iterator chain reduces readability.

**Notes**:
Extract a helper predicate: `fn has_manifest_file(stack: Stack, dir: &Path) -> bool` that encapsulates `stack.manifest_files().iter().any(|f| dir.join(f).exists())`. This reduces the nesting in detect() to 3 levels (loop → if-let → early return).
