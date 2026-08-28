---
id: TASK-1913
title: >-
  TEST-15: has_staged_files_handles_large_output_without_deadlock races its own
  1500 ms probe timeout against 40 000 lines of fake-git output
status: To Do
assignee:
  - TASK-2009
created_date: '2026-08-27 15:40'
updated_date: '2026-08-28 14:17'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:353-385`

**What**: The deadlock regression test makes the value it is asserting depend on wall-clock speed:

```rust
let fake_git = write_fake_git(dir.path(), "git-loud",
    "#!/bin/sh\n yes path/to/some/file.txt | head -n 20000\n yes path/to/some/file.txt | head -n 20000 >&2\n exit 1\n");
let result = retry_while_text_file_busy(|| {
    has_staged_files_with_timeout(fake_git.to_str().unwrap(), dir.path(), Duration::from_millis(1500))
});
assert!(matches!(result, Ok(true)), "expected Ok(true), got {result:?}");
assert!(elapsed < Duration::from_secs(2), "should not deadlock on full pipe buffers, elapsed = {elapsed:?}");
```

The timeout handed to the function under test is 1500 ms, and the child forks four processes and writes 40 000 lines across two pipes. On a loaded CI box, a cold page cache, an emulated/QEMU runner, or a `cargo nextest` run saturating every core, that shell pipeline can exceed 1500 ms — at which point `has_staged_files_with_timeout` correctly returns `Err(Timeout)` and the test fails reporting a deadlock that did not happen. The `elapsed < 2s` assertion has the same 500 ms of headroom.

Nothing about the property under test (a full pipe buffer must not deadlock the wait) requires a short timeout: a deadlock hangs forever, so a 30 s bound distinguishes it from slowness just as well while removing the race entirely. The 1500 ms figure only shortens the failure report when the bug *is* present.

Two related timing couplings in the same module, worth fixing in the same pass:

- `has_staged_files_times_out_on_hanging_git` (lines 289-314) asserts `elapsed < Duration::from_secs(5)` against a 200 ms configured timeout — 25x headroom, so much safer, but still a wall-clock assertion in a parallel suite.
- `retry_while_text_file_busy` (lines 211-226) is a `thread::sleep(20ms)` x 50 spin used by four tests, i.e. up to 1 s of sleeping per test and up to 4 s across the module. The ETXTBSY race it works around is real, but the retry loop is itself a sleep-based wait rather than a deterministic sync point — writing the fake git into a directory created before the harness forks, or opening with `O_CLOEXEC` and closing the fd explicitly before exec, removes the race at the source.

**Why it matters**: TEST-15 — prefer deterministic sync points over sleep-based and wall-clock-bounded waits. A flaky failure here reads as "the pipe-buffer deadlock is back" (a CONC-3 regression closed by TASK-0650), which is the most expensive possible false alarm: it points an investigator at concurrency code that is fine. Timing-coupled tests in a hook crate also fail most often on exactly the loaded machines where the suite runs least often.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 has_staged_files_handles_large_output_without_deadlock passes a timeout generous enough (e.g. 30s) that a slow-but-correct run cannot produce Err(Timeout), so the assertion tests deadlock and not machine speed
- [ ] #2 The elapsed < 2s assertion is removed or widened to a bound that only a genuine hang can exceed
- [ ] #3 has_staged_files_times_out_on_hanging_git keeps its Timeout-variant assertion; any remaining elapsed bound is documented as a hang detector, not a performance budget
- [ ] #4 retry_while_text_file_busy is either replaced by a deterministic fix for the ETXTBSY race or carries a comment stating its worst-case added runtime per test
<!-- AC:END -->
