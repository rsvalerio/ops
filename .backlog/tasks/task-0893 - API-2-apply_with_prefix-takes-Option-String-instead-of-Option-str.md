---
id: TASK-0893
title: 'API-2: apply_with_prefix takes &Option<String> instead of Option<&str>'
status: Triage
assignee: []
created_date: '2026-05-02 09:47'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/theme/src/style.rs:184

**What**: pub fn apply_with_prefix(text, prefix: &Option<String>) -> Cow<str> uses the &Option<String> parameter style flagged by clippy::ref_option. Callers must pass &self.field and the API is locked to String storage on the caller side.

**Why it matters**: &Option<String> forces caller storage type and prevents passing Option<&str> (e.g. from a builder, a Cow, or a borrow of a different owner). Idiomatic Rust uses Option<&str>, which composes better and matches the rest of the theme crate. This is a public API.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change signature to take prefix: Option<&str> instead of &Option<String>
- [ ] #2 Update call sites in configurable.rs to pass self.header_prefix.as_deref() etc.
- [ ] #3 Update BorderArgs.title_prefix and other carriers to match
- [ ] #4 Verify clippy::ref_option no longer fires on this function
<!-- AC:END -->
