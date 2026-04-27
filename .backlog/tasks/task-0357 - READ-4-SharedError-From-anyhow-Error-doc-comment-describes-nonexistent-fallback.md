---
id: TASK-0357
title: >-
  READ-4: SharedError::From<anyhow::Error> doc-comment describes nonexistent
  fallback
status: To Do
assignee:
  - TASK-0421
created_date: '2026-04-26 09:35'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/error.rs:24`

**What**: The inline comment says "anyhow::Error does not implement std::error::Error, so convert via into_inner() to extract the boxed source, falling back to an io::Error wrapper." The actual code performs a single infallible let boxed: Box<dyn Error + Send + Sync> = err.into(); — there is no into_inner, no fallback, and no io::Error wrapper.

**Why it matters**: Doc claims behaviour the code does not implement. Future readers debugging error chains will look for the missing fallback.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the comment with an accurate one-line note about anyhow Into<Box<dyn Error + Send + Sync>> impl preserving the source chain
- [ ] #2 No code change required; verify cargo doc rendering matches
<!-- AC:END -->
