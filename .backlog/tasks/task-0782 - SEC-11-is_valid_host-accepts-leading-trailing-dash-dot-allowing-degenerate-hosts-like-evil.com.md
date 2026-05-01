---
id: TASK-0782
title: >-
  SEC-11: is_valid_host accepts leading/trailing dash/dot, allowing degenerate
  hosts like -evil.com
status: To Do
assignee:
  - TASK-0827
created_date: '2026-05-01 05:57'
updated_date: '2026-05-01 06:18'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:106`

**What**: is_valid_host checks only is_ascii_alphanumeric() || b == b'.' || b == b'-'. Inputs like -evil.com, .., .com, host- pass and become https://-evil.com/... in the synthesized url field that flows into JSON output / about cards.

**Why it matters**: A `-` leading host can be interpreted as an option flag by some shell consumers (e.g. curl legacy mode) and `..` is meaningless DNS. The reconstructed URL is presented to the user as a clickable normalised URL; SEC-11 layered validation says reject malformed before propagation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reject hosts with leading or trailing - or ., and reject empty labels (..)
- [ ] #2 Add tests covering each shape
<!-- AC:END -->
