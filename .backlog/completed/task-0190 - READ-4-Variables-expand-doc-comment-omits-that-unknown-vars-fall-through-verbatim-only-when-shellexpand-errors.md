---
id: TASK-0190
title: >-
  READ-4: Variables::expand doc comment omits that unknown vars fall through
  verbatim only when shellexpand errors
status: Done
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/core/src/expand.rs:28-48

**What**: The doc comment on expand() says "Unknown variables are left as-is" (lib docs, line 13) but the implementation falls back to Cow::Borrowed(input) only when shellexpand::full_with_context returns Err — which happens for e.g., an env var whose value contains invalid UTF-8 (VarError::NotUnicode propagated from lookup). For the normal "unknown variable" case, shellexpand::full_with_context succeeds and substitutes the empty string (when it sees Ok(None)) — not "leaves as-is". The tests on lines 131-138 disprove the doc by chance: $__OPS_NONEXISTENT_TEST_VAR_12345__ passes through because shellexpand treats a missing var without a default as an error in the lookup closure path, not because None was returned.

The unknown_var_passes_through test uses a $ prefix on an unknown var and asserts the raw text is preserved, which suggests shellexpand surfaces an error and we hit the unwrap_or branch. This is fragile: if shellexpand 4.x changes error semantics, the behavior silently flips to substituting empty strings.

**Why it matters**: READ-4 / CL-3. The documented invariant is not actually enforced by the code; it is incidental to shellexpand error handling. Either (a) make the invariant explicit by handling VarError::NotPresent in the lookup closure and returning Ok(Some(Cow::Owned(format!("${}", var)))) for pass-through, or (b) update the doc to say "If shellexpand cannot resolve the expression, the original string is returned unchanged".

**Related**: TASK-0142 already tracks the OWN-8 angle on the same function.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Make the unknown-var pass-through explicit in the lookup closure OR update doc comment to describe actual behavior
- [ ] #2 Add a test that asserts the behavior for a known-unset env var (not one that errors)
<!-- AC:END -->
