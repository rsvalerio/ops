---
id: TASK-1852
title: >-
  TEST-1: merge_conf_d's broken-symlink guard is unreachable since O_NOFOLLOW
  landed, and its regression test passes on the wrong error
status: Done
assignee:
  - TASK-1983
created_date: '2026-08-27 15:26'
updated_date: '2026-08-28 23:53'
labels:
  - code-review-rust
  - testing
dependencies: []
modified_files:
  - crates/core/src/config/loader/conf_d.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader/conf_d.rs:61-66` (the doc), `:77-82` (the guard), `:193-215` (`merge_conf_d_rejects_broken_symlink`)

**What**: The doc describes a specific mechanism:

```rust
/// ERR-4 / TASK-1448: a `.toml` entry that resolves to a broken symlink
/// (`DirEntry::path` exists in the listing but `File::open` reports
/// `NotFound`) is treated as a hard error here rather than being silently
/// mapped to `Ok(None)` by [`super::read_capped_toml_file`].
```

That path no longer exists. Since SEC-25 / TASK-1468 put `O_NOFOLLOW` on the config open, opening a dangling symlink returns **`ELOOP`, not `NotFound`** (confirmed: `open(dangling, O_RDONLY|O_NOFOLLOW)` → errno 40 `ELOOP`; `text.rs:184-186` maps `ELOOP` to `InvalidInput`). So a broken symlink now takes the `Err(e) => return Err(e)` arm, and the guard the doc is about:

```rust
            Ok(None) => {
                anyhow::bail!(
                    "config overlay listed in .ops.d disappeared or is a broken symlink: {:?}",
                    path.display()
                );
            }
```

is reachable only through a delete-between-`read_dir`-and-`open` race — which nothing covers.

The regression test that is supposed to guard it now passes for the wrong reason. Its only assertion is a substring of the path:

```rust
    fn merge_conf_d_rejects_broken_symlink() {
        ...
        std::os::unix::fs::symlink(dir.path().join("does-not-exist.toml"), ops_d.join("dangling.toml"))
        ...
        assert!(msg.contains("dangling.toml"), "error must name the broken overlay, got: {msg}");
```

and the symlink-refusal message satisfies it just as well as the `bail!` does. Confirmed against the binary:

```
$ ops --help   # .ops.d/dangling.toml -> nope.toml
... loading .ops.d overlay configs: failed to open config file: ".../dangling.toml":
    refusing to follow symlink at ".../dangling.toml"
```

**Why it matters**: deleting the `Ok(None)` arm entirely would keep the suite green while reintroducing the exact silent-drop that ERR-4 / TASK-1448 was filed to close — a `.ops.d` overlay that vanishes between listing and open would once again be treated as benign absence. The doc also actively misdirects the next reader about how broken overlays are classified today.

Secondary, and worth deciding in the same change: because `O_NOFOLLOW` now refuses **every** symlink in `.ops.d`, a repo that legitimately symlinks shared fragments into `.ops.d` hard-fails the whole config load — the same over-broad-policy shape as the global-config case.

<!-- scan confidence: verified by reading conf_d.rs:61-82,193-215, by an errno probe of open(2) with O_NOFOLLOW on a dangling symlink, and by running the built binary -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 merge_conf_d_rejects_broken_symlink asserts the specific error the code produces today, so it fails if the branch it names is deleted or reclassified
- [x] #2 The ERR-4 doc comment describes the condition that actually reaches the Ok(None) arm (a race between read_dir and open) rather than the broken-symlink case O_NOFOLLOW now intercepts
- [x] #3 The delete-between-listing-and-open race has a test, or the Ok(None) arm is documented as defence-in-depth with the reason it cannot currently be exercised
- [x] #4 A decision is recorded on whether a legitimate symlink inside .ops.d should hard-fail the load, consistent with whatever the global-config path settles on
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC #1: `merge_conf_d_rejects_broken_symlink` now also asserts the message
contains `refusing to follow symlink`, i.e. the SEC-25 `ELOOP` branch the code
actually takes today. The old path-substring-only assertion was satisfied by
either branch, so the `Ok(None)` bail the test is named for could have been
deleted with the suite still green.

AC #2 + #3: the ERR-4 doc on `merge_conf_d` was rewritten. It now says a
dangling symlink returns `ELOOP` under `O_NOFOLLOW` (mapped to `InvalidInput`)
and therefore takes the `Err` arm, and that the only remaining route into
`Ok(None)` is a delete/rename **between `read_dir` and `open`** — a genuine
TOCTOU race, documented as defence in depth with the reason it is not
unit-tested (reproducing it needs a scheduler-timed unlink inside the loop).

AC #4 (recorded decision): symlinks inside `.ops.d` **stay refused**. `.ops.d`
is workspace-relative and therefore repo-supplied — the same
attacker-controlled trust boundary as `.ops.toml`. The SEC-14 / TASK-1847
relaxation landed in the same wave applies only to the user's own
`~/.config/ops/config.toml`; the decision and its reasoning are recorded on
`merge_conf_d` and cross-referenced to `loader::SymlinkPolicy`. New test
`merge_conf_d_refuses_symlink_to_a_real_file` pins it, so the global-config
relaxation cannot leak into `.ops.d` later.
<!-- SECTION:NOTES:END -->
