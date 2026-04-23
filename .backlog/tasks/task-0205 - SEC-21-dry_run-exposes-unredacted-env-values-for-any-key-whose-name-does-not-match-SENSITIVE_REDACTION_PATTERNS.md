---
id: TASK-0205
title: >-
  SEC-21: dry_run exposes unredacted env values for any key whose name does not
  match SENSITIVE_REDACTION_PATTERNS
status: To Do
assignee: []
created_date: '2026-04-22 21:27'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - SEC
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/dry_run.rs:61-69` (print_exec_spec env loop) and `crates/runner/src/command/exec.rs:133-144` (SENSITIVE_REDACTION_PATTERNS).

**What**: `dry_run` prints `{k}={value}` for each env entry, redacting only if `is_sensitive_env_key(k)` returns true. The allowlist is narrow: password/secret/token/api_key/apikey/private/credential/auth. An env var called `DATABASE_URL=postgres://user:pw@host/db`, `GITHUB_PAT=ghp_xxx`, `SLACK_WEBHOOK=https://hooks.slack...`, or any custom sensitive name that does not contain one of those substrings prints fully in cleartext — including to stdout that the user may copy into a bug report.

**Why it matters**: SEC-21 (information disclosure) combined with SEC-5/SEC-6 (secret handling). The warning path `warn_if_sensitive_env` already uses the broader `SENSITIVE_KEY_PATTERNS` *and* `looks_like_secret_value(v)` (JWT/UUID/entropy heuristics). Apply the same value-side heuristic in dry_run: if the key is in the allowlist *or* the value itself looks like a secret, redact. Also consider reusing SENSITIVE_KEY_PATTERNS (the superset) for redaction, since the comment in exec.rs notes the allowlist is intentionally narrower but gives a weak rationale ("commonly appear in non-secret contexts").
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Dry-run redacts env values whose key or value looks like a secret (apply looks_like_secret_value to the value)
- [ ] #2 Consider unifying SENSITIVE_REDACTION_PATTERNS with SENSITIVE_KEY_PATTERNS or justify the split in code with examples
<!-- AC:END -->
