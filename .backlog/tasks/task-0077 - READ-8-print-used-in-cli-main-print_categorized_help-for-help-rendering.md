---
id: TASK-0077
title: 'READ-8: print! used in cli/main print_categorized_help for help rendering'
status: Done
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 16:06'
labels:
  - rust-codereview
  - read
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:342`

**What**: print_categorized_help calls print!() three times to emit help output; the rest of the CLI uses writeln!(stdout) or anyhow-threaded write!.

**Why it matters**: Inconsistent stdout API; mixed print!/writeln! complicates test capture and output routing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace print!() calls with writeln!(std::io::stdout(), ...)
- [x] #2 Consider taking a &mut dyn Write so tests can assert on help rendering
<!-- AC:END -->
