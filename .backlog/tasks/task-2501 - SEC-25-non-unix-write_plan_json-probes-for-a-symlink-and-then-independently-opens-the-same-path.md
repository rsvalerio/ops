---
id: TASK-2501
title: 'SEC-25: non-unix write_plan_json probes for a symlink and then independently opens the same path'
status: To Do
assignee: []
created_date: '2026-10-10 15:31'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2622'
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-25:extensions-terraform/plan/src/lib.rs:write_plan_json'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:628`

**What**: In write_plan_json, the unix arm opens with custom_flags(libc::O_NOFOLLOW) so the open itself atomically rejects a symlink at the destination (mapping ELOOP to the named error). The #[cfg(not(unix))] fallback instead does symlink_metadata(path) first and only then open_opts.open(path) — two independent resolutions of the same path. Between the lstat and the open, the path can be replaced with a symlink/reparse point, and the write then goes through it: the exact check-then-act shape SEC-25 names, on the function whose documented guarantee is "creates the plan JSON at 0600 without ever following a pre-existing symlink at the destination". The code comment acknowledges this is "the rejection half only", which detection-after-the-fact is not: the check happens before the act, and the act re-resolves the path.

**Why it matters**: SEC-25 (TOCTOU; OWASP A01). The plan JSON carries the after values of generated passwords and keys plus full provider configuration, and the symlink swap is the canonical exfiltration shape — the attacker's target receives the secrets and the follow-up chmod. On unix the flag makes the open itself the check, so the race is closed; the non-unix build keeps a window std cannot close with OpenOptions alone. Marked low because it is confined to the non-primary platform, no local attacker is in the default threat model there, and std exposes no portable O_NOFOLLOW — but the residual gap should be a recorded decision rather than silent.

<!-- scan confidence: candidates to inspect — single candidate, non-unix cfg arm only; the unix arm is atomic via O_NOFOLLOW -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On the non-unix arm the symlink rejection and the open are combined into one syscall-level operation (e.g. FILE_FLAG_OPEN_REPARSE_POINT via the win32 API behind a small cfg gate), or the residual TOCTOU window is documented at the function as an accepted platform limitation with the reasoning
- [ ] #2 The unix arm keeps its O_NOFOLLOW + ELOOP behaviour unchanged
<!-- AC:END -->
