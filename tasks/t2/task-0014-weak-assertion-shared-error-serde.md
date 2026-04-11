---
id: TASK-0014
title: "Weak: shared_error_from_serde_json only asserts non-empty"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-11, low, crate-extension]
dependencies: []
---

## Description

**Location**: `crates/extension/src/lib.rs`
**Anchor**: `fn shared_error_from_serde_json`
**Impact**: The test wraps a `serde_json::Error` into `SharedError` but only asserts `!shared.to_string().is_empty()`. It does not verify the error message content or that the source chain is preserved.

**Notes**:
Compare with `shared_error_from_anyhow` which checks `contains("anyhow message")`. This test should similarly assert that the serde error message content is preserved in the `SharedError` display output. Low severity because the `From` impl is trivial delegation.
