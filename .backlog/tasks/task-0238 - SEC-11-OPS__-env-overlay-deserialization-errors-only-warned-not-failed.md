---
id: TASK-0238
title: 'SEC-11: OPS__ env overlay deserialization errors only warned, not failed'
status: To Do
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:26`

**What**: merge_env_vars logs a warning on deserialization failure and silently continues.

**Why it matters**: Silent misconfiguration of security-relevant fields means operator intent is not enforced.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Return an error and propagate through load_config
- [ ] #2 Add test where malformed OPS__ env var causes load_config to fail fast
<!-- AC:END -->
