---
id: TASK-1778
title: >-
  PATTERN-1: cargo's `Downgrading` lockfile verb is silently dropped — no entry,
  no count, no drift warn
status: Triage
assignee: []
created_date: '2026-08-27 11:22'
labels:
  - code-review-rust
  - correctness
dependencies: []
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
  - extensions-rust/cargo-update/src/tests.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:250-254` (`ACTION_PREFIXES`), `:301-319` (`starts_with_known_verb`), `:332-398` (`parse_action_line`)

**What**: `ACTION_PREFIXES` recognises exactly three cargo verbs — `Updating`,
`Adding`, `Removing`. Cargo's lockfile-change printer emits **five**. Verified
against the installed toolchain (cargo 1.98.0):

```
$ grep -a -o -b "Downgrading" ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo
2366564:Downgrading
# surrounding literal blob: ...AddingDowngradingUnchangedpublic_dependency...
```

`Adding`, `Downgrading` and `Unchanged` sit adjacent in the same string blob —
they are the arms of cargo's `print_lockfile_updates` change printer.

A line such as

```
    Downgrading serde v1.0.220 -> v1.0.219
```

flows through `parse_update_output` like this:

1. it is not empty / `Locking` / `warning:` / `note:`, so it survives the noise filter;
2. it does not start with `Updating`, so `is_index_progress_line` is never consulted;
3. `parse_action_line` iterates `ACTION_PREFIXES`, none of which `strip_prefix`es, and returns `None`;
4. `starts_with_known_verb` iterates the same three prefixes, matches none, and returns `false` — so the `tracing::warn!` drift breadcrumb at `:155` **does not fire**.

The entry is dropped with no `UpdateEntry`, no count increment, and no log
record at any level. The provider reports "no updates" for a lockfile that
cargo just told it is changing.

This is not a hypothetical drift shape: plain `cargo update --dry-run`
downgrades whenever `Cargo.lock` holds a version above what `Cargo.toml` now
requires (a tightened or lowered version requirement, a `[patch]` removal, a
yanked release). The workspace's own `[patch.crates-io]` pin of `quinn-proto`
(root `Cargo.toml`) is exactly the shape that produces `Downgrading` lines when
the pin is lifted.

**Why it matters**: silent data loss on the crate's single documented purpose.
Every other drop path in this file was hardened precisely so it could not
happen silently — TASK-0472 promoted unparsed verb lines to `warn`, TASK-0613
and TASK-0949 warn on trailing tokens, TASK-1054 stopped the `contains("index")`
predicate from eating `indexer`. `Downgrading` bypasses all of them because it
never reaches the verb table at all. `UpdateAction` is `#[non_exhaustive]`
(`:37`), so adding a `Downgrade` variant is a non-breaking change; the
`Unchanged` verb (verbose-only) should be added to the noise filter for the
same reason — today it is silently ignored rather than deliberately ignored.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ACTION_PREFIXES recognises cargo's Downgrading verb and parse_action_line maps it to a distinct UpdateAction variant (or, if downgrades are deliberately out of scope, the line is filtered explicitly and the decision is documented at the table)
- [ ] #2 A Downgrading line either produces an entry with a dedicated count field or is explicitly filtered — it must not be dropped with no entry AND no log record
- [ ] #3 starts_with_known_verb covers every verb cargo's lockfile-change printer emits, so an unhandled shape of a known verb still reaches the format-drift warn
- [ ] #4 The verbose-only Unchanged verb is added to the explicit noise-skip list alongside Locking/warning:/note: rather than falling through unrecognised
- [ ] #5 A test feeds a Downgrading line (and an Unchanged line) through parse_update_output and pins the chosen behaviour, including the log record
<!-- AC:END -->
