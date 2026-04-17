---
id: TASK-0075
title: 'API: ProgressDisplay constructor has 4 positional params of the same shape'
status: To Do
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:148`

**What**: ProgressDisplay::new takes 4 parameters (OutputConfig, HashMap<String,String>, &IndexMap<String,ThemeConfig>, Option<PathBuf>); new_with_tty_check adds a 5th.

**Why it matters**: Call-sites repeat the same argument order; easy to swap display_map and themes by mistake since both are map-like.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce a DisplayOptions struct or builder so call-sites are self-documenting
- [ ] #2 Call-sites in run_cmd.rs updated and readable
<!-- AC:END -->
