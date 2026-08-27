---
id: TASK-1863
title: >-
  SEC-11: read_head_branch ships the raw .git/HEAD ref through git_info.branch —
  none of the SEC-2 control-byte / bidi hardening applied to remote_url covers
  it
status: Triage
assignee: []
created_date: '2026-08-27 15:29'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/git/src/config.rs
  - extensions/git/src/provider.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:120-142` (`read_head_branch`), surfaced at `extensions/git/src/provider.rs:45`

**What**: `read_origin_url` funnels every `url = …` value through `RedactedUrl::redact`, which rejects ASCII control bytes (SEC-2 / TASK-1102) and Unicode formatting / bidi / zero-width / line-separator codepoints (SEC-2 / TASK-1238) before the value can reach JSON, About cards, or logs. `read_head_branch` — same file, same threat model, same output struct — applies none of that. It reads `.git/HEAD`, strips `ref:` and `refs/heads/`, checks only `is_empty()`, and returns the remainder verbatim into `GitInfo.branch`, which `GitInfoProvider::provide` serialises straight into the `git_info` provider JSON.

A `.git/HEAD` of `ref: refs/heads/main\u{1b}[2J\u{1b}[31mFAKE` yields `branch = "main\u{1b}[2J\u{1b}[31mFAKE"`. Nothing between that read and the terminal filters it. The same path admits:
- bidi / RTL-override and zero-width codepoints (U+202E, U+200B…) — the exact homograph-spoofing surface TASK-1238 closed for the remote URL;
- `\r` (`str::lines`-style splitting is not involved here, and `trim` only removes leading/trailing whitespace, so an interior CR survives);
- traversal-shaped refs: `ref: refs/heads/../../../etc` returns `branch = "../../../etc"`, the shape `is_valid_path_segment` (`remote.rs:168-177`, SEC-13 / TASK-0929) exists to reject on the remote side;
- an unbounded-length branch string (see the companion SEC-33 finding on the missing HEAD byte cap).

`.git/HEAD` is a plain file: git itself will not write a control character into a refname, but an adversarial repo obtained as a tarball, mounted volume, submodule, or third-party checkout can — that is precisely the threat model the file's own TASK-0910 / TASK-1102 comments already adopt for `.git/config`.

**Why it matters**: `git_info.branch` is rendered on About cards and emitted in provider JSON consumed by other extensions. Escape sequences repaint or clear the operator's terminal and can forge surrounding output; bidi overrides let a hostile checkout display a branch name that reads as something else. The crate hardened one of its two `.git` readers and left the other fail-open, so the documented guarantee ("operator-facing git metadata is control-character free") is only half true.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_head_branch applies the same whole-codepoint policy as RedactedUrl::redact (is_ascii_control_byte + is_unicode_format_or_separator), returning None rather than a partially-sanitised branch when the ref contains a rejected codepoint
- [ ] #2 the rejection reuses the existing helpers in config.rs rather than a third copy of the predicate
- [ ] #3 a ref that resolves to a traversal shape (any path segment consisting solely of '.') is rejected
- [ ] #4 a rejected HEAD emits one tracing::warn! naming the reason, mirroring the read_origin_url rejected-line breadcrumb (TASK-1215)
- [ ] #5 unit tests cover an ANSI-escape branch, a U+202E branch, an interior CR, a '..' segment, and pin that a normal 'feature/foo' branch still round-trips
<!-- AC:END -->
