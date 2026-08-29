---
id: TASK-1854
title: >-
  SEC-31: try_expand never errors on an undefined variable, so the
  Variables::empty fail-loud fallback silently emits a literal $OPS_ROOT into
  argv
status: Done
assignee:
  - TASK-1984
created_date: '2026-08-27 15:27'
updated_date: '2026-08-29 00:37'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/core/src/expand.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:454-462` (`try_expand`'s `# Errors`), `crates/core/src/expand.rs:449-457` (`Variables::empty` doc), consumed at `crates/runner/src/command/mod.rs:231-233`

**What**: The strict entry point documents an error it cannot produce.

```rust
    /// # Errors
    ///
    /// [`ExpandError`] if `input` references an undefined variable, or if a
    /// referenced value is not valid Unicode.
    pub fn try_expand<'a>(&'a self, input: &'a str) -> Result<Cow<'a, str>, ExpandError> {
```

The lookup maps a missing variable to `Ok(None)`, and `shellexpand` renders `Ok(None)` as the **literal reference text**:

```rust
            match std::env::var(var) {
                Ok(val) => Ok(Some(Cow::Owned(val))),
                Err(std::env::VarError::NotPresent) => Ok(None),
                Err(e) => Err(e),
            }
```

The struct-level doc at expand.rs:49-55 describes this correctly — *"shellexpand handles `Ok(None)` itself by leaving the reference (e.g. `$UNDEFINED`) literal in the output"* — so the two doc blocks in this one file directly contradict each other. Measured against the built crate:

```
empty + $OPS_ROOT/src         -> Ok("$OPS_ROOT/src")
empty + ${OPS_ROOT}/src       -> Ok("${OPS_ROOT}/src")
empty + $DEFINITELY_UNSET_XYZ -> Ok("$DEFINITELY_UNSET_XYZ")
```

**Why it matters — this is a fail-open, not just a doc bug.** `Variables::empty()` exists specifically as the *safe* fallback when `from_env` rejects a workspace root, and its own doc says so:

```rust
    /// `from_env` surfaces a non-UTF-8 workspace root via
    /// `ExpandError::NotUnicode`) so downstream `try_expand` calls fail
    /// loud on the missing variable rather than panicking at runner
    /// construction time. ERR-1 / TASK-1462.
```

Its only production caller repeats the belief (`crates/runner/src/command/mod.rs:231-233`: *"A subsequent `try_expand("$OPS_ROOT/...")` will fail explicitly when callers actually touch the variable, preserving the 'fail-loud' intent"*). Neither is true. When `from_env` refuses a corrupt root, the runner falls back to `empty()`, `try_expand("$OPS_ROOT/target")` returns `Ok("$OPS_ROOT/target")`, and that literal string is materialised as an argv element or a cwd — the exact "literal `${VAR}` on disk" outcome `try_expand` was added (ERR-1 / TASK-0450) to make impossible. Worse: because the fallback still consults `std::env::var`, an ambient `OPS_ROOT` in the environment silently resolves to an **unrelated directory** instead of failing, so a rejected root becomes a *wrong* root rather than an error.

There is no test asserting the documented error, and there cannot be one while the code behaves this way.

Relationship to TASK-1805: that finding covers `from_env`'s own non-UTF-8 handling and the phantom `ExpandError::NotUnicode` doc links. This one covers what happens *after* `from_env` correctly refuses — the promised downstream failure never arrives. They are complementary; fixing 1805 alone leaves this open.

Which side to fix is a design call — either `try_expand` gains a real undefined-variable error (a `HashSet` of referenced names, or a lookup that returns `Err` instead of `Ok(None)` in strict mode), or the two doc blocks and the runner's fallback rationale are corrected and the fallback is replaced with something that actually fails. What is not acceptable is the current state, where three comments assert a guarantee the code does not provide.

<!-- scan confidence: verified by reading expand.rs:45-60, 449-475, and by running a probe binary linked against ops-core that printed the three Ok(...) results above -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 try_expand's documented behaviour and its actual behaviour agree: either an undefined variable produces an Err, or the # Errors section and Variables::empty's doc stop claiming it does
- [x] #2 If the docs are corrected rather than the code, the Variables::empty fallback is replaced by something that actually fails closed, and the runner's from_arc_config rationale is updated to match
- [x] #3 A test pins the chosen behaviour for a bare $UNDEFINED, a ${UNDEFINED} form, and ${UNDEFINED:-default}
- [x] #4 A test covers the ambient-environment case: with OPS_ROOT set in the process environment, a Variables::empty expansion of $OPS_ROOT does not silently resolve to that unrelated directory
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in wave TASK-1984. Chose the docs-plus-fail-closed-fallback branch of AC#1/AC#2 rather than making an undefined variable an error: shellexpand calls the same lookup for `${VAR:-default}` as for a bare `$VAR`, so returning Err on a miss cannot be done without also breaking the documented default form. Instead: (a) try_expand `# Errors` now states what it really produces and explains why an undefined variable is deliberately not an error; (b) `Variables::empty()` is replaced by `Variables::poisoned(err)`, which stores the ExpandError and returns it from EVERY try_expand call with no builtin lookup and no std::env fallback; (c) `CommandRunner::from_arc_config` now falls back to `Variables::poisoned(e)` and its rationale comment is rewritten to match. AC#4 substitution recorded: `Variables::empty` no longer exists (it had exactly one production caller, the runner fallback), so the ambient-environment test pins the same property on the fallback that actually ships — poisoned_variables_fail_closed_even_with_ambient_ops_root sets OPS_ROOT=/unrelated/ambient/root via EnvGuard under #[serial] and asserts try_expand errors and the lossy expand never resolves to that directory. AC#3 pinned by undefined_variable_forms_are_pinned ($UNDEFINED and ${UNDEFINED} stay literal, ${UNDEFINED:-default} resolves).
<!-- SECTION:NOTES:END -->
