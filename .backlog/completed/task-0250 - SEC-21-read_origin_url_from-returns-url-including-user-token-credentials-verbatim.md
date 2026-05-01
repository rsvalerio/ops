---
id: TASK-0250
title: >-
  SEC-21: read_origin_url_from returns url including user:token@ credentials
  verbatim
status: Done
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 07:45'
labels:
  - rust-code-review
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:38`

**What**: The returned string is the raw git config value; callers that log it expose stored HTTP credentials.

**Why it matters**: Credential leakage via logs/errors/data output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Redact userinfo segment inside this function or document requirement
- [x] #2 Test input with embedded credentials
<!-- AC:END -->
