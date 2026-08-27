---
id: TASK-1880
title: >-
  READ-6: parse_remote_url normalises the scheme to lowercase but leaves the
  host in its original case, so git_info.host is not canonical
status: Triage
assignee: []
created_date: '2026-08-27 15:32'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/git/src/remote.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:42-65` (`parse_remote_url`), `remote.rs:96-103` (`split_scheme_host_and_path`)

**What**: `split_scheme_host_and_path` deliberately canonicalises the scheme — it matches `ALLOWED_SCHEMES` with `eq_ignore_ascii_case` and returns the lowercase `&'static str`, and `scheme_normalises_to_lowercase` pins that behaviour with the rationale "so audit code downstream sees a canonical value". The host, whose case is equally insignificant (DNS names are case-insensitive; `is_valid_host` accepts both cases), is copied through verbatim:

```rust
parse_remote_url("HTTPS://GitHub.COM/o/r")
// → host: "GitHub.COM", url: "https://GitHub.COM/o/r"
```

Half the value is canonical and half is not, with no comment explaining the asymmetry, and no test covering host case at all.

**Why it matters**: `git_info.host` is the field consumers use to branch on forge identity (`host == "github.com"`, "is this our internal GitLab?"). Any such comparison, and any deduplication or grouping keyed on `host` or `remote_url`, silently fails for a remote whose config happens to carry mixed-case host — a shape git accepts and tools do write. The same argument the crate already accepted for the scheme applies verbatim to the host. Low severity because the input shape is uncommon, but the fix is a one-liner and it removes an inconsistency inside a single `format!`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 the host is lowercased (ASCII) before it is stored in RemoteInfo.host and interpolated into RemoteInfo.url, matching the scheme's canonicalisation
- [ ] #2 a unit test pins that 'HTTPS://GitHub.COM/o/r' yields host 'github.com' and url 'https://github.com/o/r'
- [ ] #3 the owner/repo case is left untouched (forge path segments are case-sensitive) and a comment records why host and scheme are treated differently from the path
<!-- AC:END -->
