---
id: TASK-0336
title: >-
  SEC-11: looks_like_aws_key matches any 40-char base64-ish string including git
  SHAs
status: To Do
assignee:
  - TASK-0419
created_date: '2026-04-26 09:33'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/secret_patterns.rs:171-176`

**What**: The detector flags any 40-char string of `[A-Za-z0-9+/=]` as an AWS-style key. Git commit SHAs (40 hex chars) and many CI build tokens fit that mold.

**Why it matters**: `warn_if_sensitive_env` will log a SEC-002 warning recommending users move their git SHA to OS env, which is noise that trains operators to ignore the warning.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tighten looks_like_aws_key to require AWS access-key prefix patterns (AKIA, ASIA, AGPA, etc.) or stricter mixed-case + non-hex requirement
- [ ] #2 Test asserts a 40-char git SHA does not trigger looks_like_secret_value
<!-- AC:END -->
