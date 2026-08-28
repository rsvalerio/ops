---
id: TASK-1924
title: >-
  ERR-2: collect_rust_loc documents two error conditions that its own body makes
  unreachable — it is infallible in every configuration
status: Done
assignee:
  - TASK-1998
created_date: '2026-08-27 15:45'
updated_date: '2026-08-28 15:36'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions-rust/loc/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/lib.rs:108` (doc), `:112` (`collect_rust_loc`)

**What**: The public `# Errors` section reads:

    /// # Errors
    ///
    /// If a discovered source file cannot be read, or the collected records
    /// fail to serialize.

Neither condition can produce an `Err`:

1. "a discovered source file cannot be read" - the read at lib.rs:135 matches on the error, logs `tracing::warn!` and `continue`s (lines 137-143). The documented policy immediately above (lines 99-107) says so explicitly: "Anything that cannot be read ... is logged and skipped rather than aborting the scan". The same is true for the walker error at lines 120-126.
2. "the collected records fail to serialize" - the only exit is `Ok(serde_json::Value::Array(records))` at line 151. That constructs a `Value` from a `Vec<Value>`; it performs no serialization and has no failure mode. `push_records` builds each row with the infallible `serde_json::json!` macro.

`collect_rust_loc` therefore always returns `Ok`. Every caller carries dead error handling: `RustLocIngestor::collect` wraps it in `.map_err(external_err)` (ingestor.rs:20) for a branch that never fires, and the closure at lib.rs:70 does the same.

**Why it matters**: ERR-2 requires the documented error conditions of a public function to match what it actually returns. Here the doc says the opposite of the deliberate warn-and-skip policy documented eight lines above it, which is exactly the kind of contradiction that gets 'fixed' in the wrong direction: a future maintainer reading only the `# Errors` block has every reason to convert the `continue` at line 143 into a `?`, silently turning a partial count into a hard failure of the whole About page. This mirrors TASK-1805, where a function contradicted its own documented Errors contract.

The `Result` return type itself should stay - `ops_duckdb::try_provide_from_db` and `DataIngestor::collect` both require a fallible closure, and a future size cap or cancellation check could legitimately use it. The defect is the doc, not the signature.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The # Errors section on collect_rust_loc describes only conditions the body can actually produce; if none remain, it states that the function currently cannot fail and that the Result is kept for the try_provide_from_db / DataIngestor::collect contract
- [x] #2 The doc cross-references the warn-and-skip degradation policy above it so the two paragraphs no longer contradict each other
- [x] #3 Either the unreadable-file and unwalkable-path branches genuinely propagate an error and the doc stays as written, or they keep skipping and the doc is corrected - the change picks one and makes code and doc agree
<!-- AC:END -->
