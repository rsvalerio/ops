---
id: TASK-1770
title: >-
  TEST-19: provide_drops_absolute_and_traversal_members writes to and rm -rf's a
  shared path outside its tempdir
status: Triage
assignee: []
created_date: '2026-08-27 11:21'
updated_date: '2026-08-27 11:21'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/about/src/units.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:335-374` (`provide_drops_absolute_and_traversal_members`), specifically `:344-359`

**What**: The test escapes its own sandbox on purpose and lands on a fixed, shared path:

```rust
let parent = root.parent().expect("tempdir has a parent");   // = /tmp (or $TMPDIR)
let hostile = parent.join("escape");                          // = /tmp/escape
std::fs::create_dir_all(&hostile).unwrap();
std::fs::write(hostile.join("Cargo.toml"), "...name = \"hostile\"...").unwrap();
...
impl Drop for Cleanup<'_> { fn drop(&mut self) { let _ = std::fs::remove_dir_all(self.0); } }
```

`root` is a `tempfile::tempdir()`, so `root.parent()` is the *system* temp directory. The path `/tmp/escape` is not derived from the tempdir's random component — it is the same absolute path on every run, for every user, in every test binary.

Three consequences:

1. **Not isolated per test (TEST-18/TEST-19).** `cargo test` runs test binaries concurrently and `cargo nextest` runs each test in its own process. Two concurrent runs of this test — or a rerun overlapping a previous one — race on creating, writing, and `remove_dir_all`-ing the same `/tmp/escape`. The loser sees the directory vanish mid-test and either fails or, worse, passes for the wrong reason: if the `Cargo.toml` was deleted by the other run before the provider is invoked, the assertion `arr.is_empty()` holds trivially and the traversal defence is never actually exercised. That is a silent false-green on a SEC-14 regression test.
2. **Destroys data outside the test sandbox.** `remove_dir_all("/tmp/escape")` runs unconditionally in `Drop`, including on the panic/abort path. Any pre-existing `/tmp/escape` belonging to the developer or to another process is deleted, and it is deleted recursively.
3. **Order-dependent with the shared `typed_manifest_cache`.** The test is `#[serial_test::serial(typed_manifest_cache)]`, which serialises it against siblings *within one binary* but gives no protection against a second test process.

The test's intent is sound — plant a hostile manifest where `../escape` would resolve to, so a regression surfaces as a non-empty unit list. The mistake is anchoring "one level above the workspace root" to the shared temp root instead of to a private tree.

**Why it matters**: A SEC-14 regression test that can be silently neutered by a concurrent run is worse than no test, because it reports green. The unconditional `remove_dir_all` of a fixed absolute path is a destructive side effect that no test should have.

**Fix direction**: create one tempdir and nest the workspace inside it (`tmp/ws/` as the root, `tmp/escape/` as the hostile sibling). Then `../escape` from `tmp/ws` still resolves to the planted manifest, everything stays inside the tempdir's random path, and cleanup is `TempDir`'s own `Drop` — no manual `remove_dir_all` and no `Cleanup` guard needed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The hostile 'escape' directory is created inside the test's own tempfile::tempdir() (e.g. tmpdir/ws as the workspace root and tmpdir/escape as its sibling), not at tempdir.parent()
- [ ] #2 The manual Cleanup Drop guard and its std::fs::remove_dir_all call are removed; cleanup is handled by TempDir's own Drop
- [ ] #3 The test passes when two instances of the test binary run concurrently
- [ ] #4 The test still fails if the SEC-14 absolute/parent-dir member filter is removed — verify by temporarily disabling member_path_is_workspace_safe and confirming a non-empty unit list is produced
<!-- AC:END -->
