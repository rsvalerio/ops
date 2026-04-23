---
id: TASK-0179
title: >-
  READ-2: ConfigurableTheme::render_error_detail boxed-layout branch mixes
  indent math with string surgery
status: To Do
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/theme/src/configurable.rs:104-142

**What**: The boxed branch of render_error_detail computes target_gutter, extra_indent, prefix_with_rail, right_target, and then in a single map closure does: a "reindent by splitting at prefix_with_rail" operation, a strip_ansi width measurement, and a right-pad-plus-border concat. The closure spans 11 lines with three distinct responsibilities and no named intermediates for the result of the split-and-inject step. Reviewers have to mentally simulate split_at on every iteration to understand that the goal is "insert extra_indent spaces after the left rail".

**Why it matters**: READ-2 / FN-2. Extracting two helpers (fn inject_gutter_indent(line: &str, rail_prefix: &str, indent: &str) -> String and fn right_pad_with_border(line: String, right_target: usize) -> String) makes the data flow explicit and each step independently testable. Right now render_error_block has test coverage but the boxed-layout transformation does not (see existing TASK-0129 about error-block color).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract the gutter-indent injection and right-pad-border steps into named helpers
- [ ] #2 Add at least one unit test that exercises the boxed-layout render_error_detail path
<!-- AC:END -->
