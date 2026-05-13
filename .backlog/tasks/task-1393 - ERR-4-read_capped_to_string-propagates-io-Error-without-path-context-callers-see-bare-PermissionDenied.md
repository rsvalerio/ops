---
id: TASK-1393
title: >-
  ERR-4: read_capped_to_string propagates io::Error without path context,
  callers see bare PermissionDenied
status: Done
assignee:
  - TASK-1450
created_date: '2026-05-13 18:03'
updated_date: '2026-05-13 19:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:117-121` (`read_capped_to_string_with` and the public `read_capped_to_string`)

**What**: The public read path `read_capped_to_string` / `read_capped_to_string_with` uses `(&mut file).take(limit).read_to_string(&mut buf)?` and `File::open(path)?` to propagate IO errors verbatim, with no `with_context` to attach the path. The explicit oversize branch above does include the path in its `InvalidData` message, but a `PermissionDenied`, `IsADirectory`, or `NotFound` from `File::open`/`read_to_string` surfaces to callers as a bare `io::Error`.

**Why it matters**: ERR-4 recommends attaching path context on file-IO error propagation where the path is known. Callers like `for_each_trimmed_line_with` happen to log `path = ?path.display()` separately, but other consumers of `read_capped_to_string` (config loader, project-identity readers) propagate the bare error up, producing "Permission denied (os error 13)" with no file name in the resulting user-facing diagnostic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either wrap the io::Error sites in read_capped_to_string_with with a typed error carrying PathBuf, or remap them so the resulting io::Error message includes path.display()
- [ ] #2 Make the failure mode consistent with the existing InvalidData (oversize) branch, which already names the path in its message
- [ ] #3 Add a test that asserts the error returned for a PermissionDenied file contains the path string, so future regressions are caught
<!-- AC:END -->
