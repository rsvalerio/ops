---
id: TASK-0863
title: 'PATTERN-1: parse_remote_url silently fails on IPv6 hosts with no operator log'
status: Done
assignee: []
created_date: '2026-05-02 09:20'
updated_date: '2026-05-02 10:38'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:112-128, 277-280`

**What**: is_valid_host rejects any non-alphanumeric/./-byte, which excludes [::1] and [2001:db8::1]. For developers self-hosting on an IPv6-only forge or running over Tailscale by [fd00:...], every about invocation silently drops remote_url, falling back to branch-only output. The ipv6_host_form_is_rejected test pins this behaviour intentionally.

**Why it matters**: "Reject rather than admit a partially-parsed weird shape" is sound, but the user-facing degradation is silent: no tracing::warn! fires when the unparseable URL is observed. The about card displays a partial card with no operator clue why.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When parse_remote_url returns None for a non-empty input, emit a single tracing::debug! from GitInfo::collect describing why
- [ ] #2 Optionally extend is_valid_host to accept a bracketed IPv6 literal and strip the brackets when reconstructing the URL
<!-- AC:END -->
