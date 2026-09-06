---
id: TASK-1887
title: >-
  ERR-2: DataProviderError::computation_failed fabricates a std::io::Error to
  carry a message that has nothing to do with I/O
status: Done
assignee:
  - TASK-1985
created_date: '2026-08-27 15:34'
updated_date: '2026-08-28 19:26'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - crates/extension/src/error.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/error.rs:115-119`

**What**: the string-message constructor manufactures an `io::Error` purely as a container:

```rust
/// Create a computation failure from a string message.
pub fn computation_failed(msg: impl Into<String>) -> Self {
    let msg = msg.into();
    Self::ComputationFailed(SharedError(Arc::new(std::io::Error::other(msg))))
}
```

Nothing about the caller's condition is I/O. `io::Error::other` is being used as a generic "box a string as an `Error`", and the resulting value is indistinguishable — by type and by `ErrorKind` — from a genuine filesystem or process failure, because a real one surfaced through this crate also lands in `ErrorKind::Other`.

The type is public and lands in the error chain that `SharedError`'s alternate-Display walk and `std::error::Error::source()` both expose, so a caller doing `err.source().and_then(|s| s.downcast_ref::<std::io::Error>())` — the normal way to recover an I/O cause — gets a hit for an error that never touched a file descriptor, and inspecting `.kind()` tells it nothing. `DataProviderError` is `#[non_exhaustive]`, so adding a variant that carries the message directly is a non-breaking change.

**Why it matters**: ERR-2 asks for domain error types that say what actually happened. Borrowing a foreign concrete type to carry a message puts a false claim into the chain, and the chain is exactly what this crate goes out of its way to preserve — the whole `SharedError` design (EFF-002) plus the `{0:#}` rendering exists so root causes reach operator logs intact. A fabricated root cause undermines that. The cost is also non-zero at runtime: `io::Error::other` boxes the message on the heap and the `Arc` boxes it again.

**Suggested fix**: add a message-carrying variant — e.g. `#[error("data computation failed: {0}")] ComputationMessage(String)` — and have `computation_failed` build it, leaving `ComputationFailed(SharedError)` for cases that genuinely wrap a source error (which is what `computation_error` already does correctly). If keeping a single variant is preferred, introduce a small private `MessageError(String)` type in `error.rs` implementing `Display`/`Error` with no source, and wrap that instead of `io::Error` — same shape, no false type identity.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DataProviderError::computation_failed no longer constructs a std::io::Error for a non-I/O condition
- [ ] #2 The replacement carries the message with no fabricated source, and DataProviderError stays Clone with its existing Display output for this constructor unchanged
- [ ] #3 A test asserts that downcasting the source chain of a computation_failed error does not yield a std::io::Error
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1 + AC#2: added the message-carrying variant DataProviderError::ComputationMessage(String) (the enum is #[non_exhaustive], so this is not a breaking change) and computation_failed now builds it. No fabricated source at all — source() returns None — and no io::Error is constructed. Display is byte-identical ("data computation failed: {msg}"), and the enum is still Clone.
AC#3: computation_failed_has_no_fabricated_io_source walks the whole source chain and asserts no link downcasts to std::io::Error.
Two in-repo assertions that matched on ComputationFailed for a computation_failed()-produced error were updated to ComputationMessage: crates/extension/src/tests.rs::data_provider_error_computation_failed and extensions/about/src/identity.rs::build_identity_value_rejects_non_utf8_cwd (plus the two rustdoc references in identity.rs). ComputationFailed keeps its meaning: a real wrapped source error, which is what computation_error still produces.
<!-- SECTION:NOTES:END -->
