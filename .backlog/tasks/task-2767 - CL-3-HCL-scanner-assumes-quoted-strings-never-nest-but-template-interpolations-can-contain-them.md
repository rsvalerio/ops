---
id: TASK-2767
title: 'CL-3: HCL scanner assumes quoted strings never nest, but template interpolations can contain them'
status: Done
assignee: []
created_date: '2026-11-18 18:43'
updated_date: '2026-11-18 22:52'
labels:
  - code-review-rust
  - CL
dependencies: []
parent_task_id: 'TASK-2856'
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'CL-3:extensions-terraform/about/src/lib.rs:strip_code_chars'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:898` (`strip_code_chars`) and `extensions-terraform/about/src/lib.rs:543` (`scan_code_chars`, with `LineWalk::step_string` at line 524)

<!-- scan confidence: candidates to inspect -->

**What**: Both lexer stages model a quoted HCL string as "`\"` opens it, the next unescaped `\"` closes it". HCL quoted strings are templates, though, and a `${ ... }` or `%{ ... }` sequence inside one can hold nested quoted strings, for example `"${replace(var.x, "_", "-")}"`. In that case both stages flip in and out of string state at the inner quotes and read the inner text as code. Most real lines balance by accident. A nested string that contains an escaped quote or a brace breaks the scan, which then misreads the file. Traced by hand:

- `x = "${replace(var.x, "\"", "")}"`: after the toggles, the `}` that closes the interpolation is read as structural, and the last `"` opens a string that never closes. The file is refused with `UnterminatedString`, although it is valid HCL.
- `x = "${format("%s}", var.y)}"`: the `}` inside the nested `"%s}"` sits in a code region and pops the block stack. Depending on where the line sits, this desynchronises nesting or leads to an `UnbalancedClosingBrace` or `UnclosedBlock` refusal.

The module docs ("Quoted strings are line-local", the `strip_comments` and `scan_line` docs) never state this assumption, and no test covers an interpolation that contains nested quotes.

**Why it matters**: A valid `versions.tf` or `main.tf` holding such a line is refused with a misleading "unterminated string" or "unbalanced brace" warning, and its `required_version` drops out of the About card. CL-3 asks for assumptions like this to be made explicit: either the lexer tracks template-interpolation depth inside strings, or the limitation is documented and pinned by tests so that nobody later reads a refusal as a malformed manifest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either both stages track ${ / %{ interpolation depth inside quoted strings (nested quotes inside an interpolation do not end the outer string), or the limitation is stated in the strip_comments / scan_line docs
- [x] #2 Unit tests cover a terraform block next to lines like x = "${replace(var.x, "\\"", "")}" and x = "${format("%s}", var.y)}", asserting the chosen behaviour
- [x] #3 Both stages still agree on string boundaries (both_stages_* style test extended to interpolations)

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.20 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
