---
id: TASK-1504
title: >-
  TEST-11: cargo-toml provider_missing_cargo_toml / provider_invalid_toml assert
  via Display substring match
status: To Do
assignee:
  - TASK-1644
created_date: '2026-05-18 18:04'
updated_date: '2026-05-25 16:08'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/tests/provider.rs:24-46`

**What**: Both tests assert `result.unwrap_err().to_string().contains("reading"|"parsing")`. The error type is `DataProviderError`; `with_context(|| format!("reading {}", ...))` is an anyhow attachment. A future error-message tweak (e.g. capitalising "Reading" or rewording for SEC-21) breaks the test without changing behaviour.

**Why it matters**: TEST-11 prefers asserting structured values over stringly-typed contracts. The error chain (or a `kind()` accessor) is the durable surface; the human message is not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace substring matching with either a downcast against the typed underlying error (io::ErrorKind::NotFound, toml::de::Error) or a matches! against a DataProviderError variant
- [ ] #2 Tests still fail when the corresponding error path is removed
<!-- AC:END -->
