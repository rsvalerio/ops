---
id: TASK-028
title: "Context struct exposes all fields as public"
status: To Do
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-code-quality, CQ, API-9, medium, effort-M, crate-extension]
dependencies: []
---

## Description

**Location**: `crates/extension/src/lib.rs:257-264`
**Anchor**: `struct Context`
**Impact**: Context exposes all fields as public (`config`, `data_cache`, `working_directory`, `refresh`, and optional `db`), allowing direct mutation and preventing future internal representation changes without breaking the API. This violates API-9 (private fields for controlled construction).

**Notes**:
Context already has `new()`, `with_refresh()`, and `get_or_provide()` methods. Making fields private and adding getter methods (`config()`, `working_directory()`, `is_refresh()`) would preserve the existing API while enabling future flexibility. The `data_cache` field is mutable internal state and should especially not be publicly accessible. A `_private: ()` field or simply making fields private with accessors would suffice.
