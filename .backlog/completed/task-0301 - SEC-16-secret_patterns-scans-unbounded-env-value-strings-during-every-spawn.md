---
id: TASK-0301
title: 'SEC-16: secret_patterns scans unbounded env-value strings during every spawn'
status: Done
assignee:
  - TASK-0323
created_date: '2026-04-24 08:52'
updated_date: '2026-04-25 12:22'
labels:
  - rust-code-review
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/secret_patterns.rs:89-132`

**What**: `looks_like_secret_value`/`has_high_entropy` iterate the full env value with no size cap. A large env value (MBs) triggers O(n) scanning per command spawn.

**Why it matters**: DoS / latency pitfall when a user sets a large env var; the secret detector becomes a hot-path bottleneck rather than a quick safety net.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Scan bounded to a prefix (e.g. 4 KiB or configurable limit) with early return
- [x] #2 Test added covering a >1 MiB value — detector must return within expected time
<!-- AC:END -->
