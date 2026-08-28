---
id: TASK-1972
title: >-
  ERR-2: collect_tokei returns a Result that can never be Err, and its # Errors
  doc describes a failure that does not exist
status: To Do
assignee:
  - TASK-2012
created_date: '2026-08-27 15:54'
updated_date: '2026-08-28 14:18'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions/tokei/src/lib.rs
  - extensions/tokei/src/ingestor.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:120-129`

**What**: The signature and doc comment promise an error channel the function does not have, and the real failure mode has none.

    /// # Errors
    ///
    /// If the collected language statistics fail to serialize to JSON.
    pub fn collect_tokei(working_dir: &Path) -> Result<serde_json::Value, anyhow::Error> {
        let mut languages = Languages::new();
        let tokei_config = TokeiConfig::default();
        languages.get_statistics(&[working_dir], TOKEI_DEFAULT_EXCLUDED, &tokei_config);
        Ok(flatten_tokei_to_json(&languages, working_dir))
    }

- The body has no fallible step. `Languages::get_statistics` returns `()`; `flatten_tokei_to_json` is infallible and returns `serde_json::Value` directly (line 132, and it is even `#[must_use]` rather than `Result`). No serialisation happens here at all -- the JSON is built with `serde_json::json!`, which cannot fail -- so the documented error condition cannot occur.
- Every caller pays for the phantom `Err`. `TokeiIngestor::collect` maps it with `external_err` (`extensions/tokei/src/ingestor.rs:20`), and `TokeiProvider::provide` converts it into `DataProviderError` (lines 59-63), for a variant no code path can produce.
- Meanwhile the failures that **do** occur are invisible. A nonexistent, unreadable or permission-denied `working_dir` produces an empty walk: `get_statistics` swallows the walk error (tokei logs it via the `log` crate and continues), so `collect_tokei` returns `Ok([])` and the provider reports success with a JSON array claiming the project has zero lines of code. The same applies per file -- any file that fails to open is dropped from the statistics with no counter, no flag in the output, and no change to the return value. `LoadResult.record_count` downstream is therefore not distinguishable from a correct answer.

Note the same crate already made the opposite call deliberately in `views.rs:20-31`, where a `Result` whose `Err` could never occur was removed in favour of a compile-time-validated newtype (TASK-1003). This function is the remaining instance of the pattern it argued against.

**Why it matters**: ERR-2 -- a public function must document which errors it returns and under what conditions; here the documented condition is fictional, so a caller writing an error-handling arm is writing dead code, and a caller trusting `Ok` cannot tell a genuinely empty project from a directory that could not be read. Either the return type should collapse to the infallible `serde_json::Value`, or it should carry the failure that actually exists (the walk root being unreadable, and a count of files skipped).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 collect_tokei either returns an infallible value or returns an error that some code path can actually produce; the # Errors doc matches the implementation
- [ ] #2 A working_dir that does not exist or cannot be read is distinguishable by the caller from a directory that genuinely contains no source files
- [ ] #3 Files skipped because they could not be read are counted and surfaced rather than silently dropped from the statistics
- [ ] #4 A test passes a nonexistent path to collect_tokei and asserts the outcome is not an empty success
<!-- AC:END -->
