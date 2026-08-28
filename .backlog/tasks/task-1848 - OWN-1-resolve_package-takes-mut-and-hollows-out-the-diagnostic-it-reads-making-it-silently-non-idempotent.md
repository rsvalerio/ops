---
id: TASK-1848
title: >-
  OWN-1: resolve_package takes &mut and hollows out the diagnostic it reads,
  making it silently non-idempotent
status: To Do
assignee:
  - TASK-1997
created_date: '2026-08-27 15:25'
updated_date: '2026-08-28 14:13'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-rust/deps/src/parse/deny.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/deny.rs:217-238` (`resolve_package`), `:265-296` (`push_diagnostic`)

**What**: `resolve_package` is a *read* — it answers "which package is this diagnostic about" — but its signature is `&mut DecodedDiagnostic` and its body destroys the data it reads:

```rust
fn resolve_package(diag: &mut DecodedDiagnostic) -> String {
    diag.advisory
        .as_mut()
        .and_then(|a| a.package.take())          // advisory.package -> None
        .or_else(|| {
            diag.graphs
                .as_mut()
                .and_then(|g| g.first_mut())
                .and_then(|g| g.krate.as_mut())
                .map(|k| std::mem::take(&mut k.name))   // krate.name -> ""
        })
        .unwrap_or_else(|| { /* debug log */ "<no package>".to_string() })
}
```

After it returns, the `DecodedDiagnostic` is in a state no reader can distinguish from genuine missing data: `advisory.package` is `None` and `graphs[0].krate.name` is `""`. A second call therefore returns `"<no package>"` for a diagnostic that has a perfectly good package name, and logs a `TASK-0597: … no package name in advisory or graphs[0].krate` warning that is false.

Nothing calls it twice *today* — `push_diagnostic` calls it once at `:266` and then reads `diag.advisory` again at `:269` for `adv.id` / `adv.title`, which happen to be the fields `resolve_package` does not steal. That is the whole safety argument, and it is invisible: it depends on `resolve_package` and `push_diagnostic` staying in exact agreement about which fields have been emptied, with nothing in the types or the signature recording it. Adding a package-bearing field to a later match arm, reordering the two calls, or reusing `diag` for a second classification (which a fix for the partial-decode-loss finding might plausibly want, to count and re-report dropped diagnostics) reintroduces it as a silent wrong-data bug rather than a compile error.

The mutation buys nothing measurable — it avoids one `String` clone per cargo-deny diagnostic, on a path that already allocates a `String` per field and runs a handful of times per `ops deps` invocation. Two clean alternatives: take `&DecodedDiagnostic` and clone the name (OWN-1: prefer `&T`), or take `DecodedDiagnostic` by value and destructure it into the package plus the remaining fields, which makes the "these fields are now consumed" fact a type-level one that `push_diagnostic` cannot get wrong.

**Why it matters**: this is an invariant held only by convention between two functions in the same file, and its failure mode is not a panic but a wrong package name plus a misleading log line — the least detectable class of bug. Encoding "the package has been taken" in the type (by consuming the value) costs one clone and removes the trap entirely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 resolve_package no longer leaves the DecodedDiagnostic partially emptied — it either borrows immutably and clones, or consumes the value and returns the remaining fields alongside the package
- [ ] #2 push_diagnostic no longer depends on an unwritten agreement about which fields resolve_package emptied
- [ ] #3 A test asserts that resolving the package for a diagnostic twice (or resolving then re-reading the advisory/graph) yields the same package name rather than the <no package> sentinel
- [ ] #4 Existing deny/tests.rs package-resolution cases (advisory package, graphs[0].krate fallback, <no package> sentinel) still pass unchanged
<!-- AC:END -->
