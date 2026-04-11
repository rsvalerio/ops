---
id: TASK-0010
title: "Test gap: warn_if_sensitive_env() has no direct test"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-5, medium, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/command/exec.rs`
**Anchor**: `fn warn_if_sensitive_env`
**Impact**: The public `warn_if_sensitive_env()` function, which emits tracing warnings for sensitive environment variables, has no direct test. Only the underlying predicate functions (`is_sensitive_env_key`, `looks_like_secret_value`) are tested. The warning emission path — including the tracing integration and log formatting — is untested.

**Notes**:
The predicate functions have thorough coverage (11+ tests in `exec.rs`, 12+ in the `sensitive_env_tests` module). The gap is specifically that `warn_if_sensitive_env` as a unit is never called in a test with `tracing_test` or similar to verify it emits the expected warnings. The dry-run redaction tests in `run_cmd.rs` cover the redaction path but not the warning path. Consider adding a test with `tracing_test::traced_test` that calls `warn_if_sensitive_env` with known sensitive vars and asserts the expected warning is emitted.
