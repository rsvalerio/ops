---
id: TASK-1669
title: >-
  VER: declared rust-version = 1.80 is wrong; the tree needs 1.82+ and nothing
  enforces it
status: Done
assignee: []
created_date: '2026-08-16 09:43'
updated_date: '2026-08-16 10:31'
labels:
  - rust-code-review
  - ci
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`[workspace.package] rust-version = "1.80"` in the root `Cargo.toml` does not match what the code actually compiles on. Cargo refuses to build for a consumer whose toolchain is older than `rust-version`, so the declaration is a contract — and it is currently a false one.

**Measured on main, 2026-08-16**, by temporarily adding `msrv = "1.80"` to `clippy.toml` and running `cargo clippy --workspace --all-features --all-targets -- -W clippy::incompatible_msrv`:

| Site | Item | Stable since |
|---|---|---|
| `crates/core/src/output.rs:37` | `std::iter::repeat_n` | 1.82 |
| `extensions-rust/cargo-toml/src/types.rs:406` | `Option::is_none_or` | 1.82 |
| `extensions-rust/loc/src/lib.rs:126` | `Option::is_none_or` | 1.82 |
| `extensions-rust/cargo-update/src/lib.rs:310` | `Option::is_none_or` | 1.82 |
| `extensions-rust/cargo-update/src/lib.rs:343` | `Option::is_none_or` | 1.82 |

That lint only sees **standard-library** APIs. It does not catch language-feature drift, and there is some: `unsafe extern "C"` blocks at `crates/core/src/test_utils.rs:541` and `crates/core/src/config/edit.rs:679` also require 1.82. So 1.82 is the floor these two sources agree on, but neither proves it is the true floor.

**Why nothing caught it**: CI pins no toolchain — every job uses `actions-rust-lang/setup-rust-toolchain` at default (stable), so CI has only ever built on a recent compiler. `clippy.toml` sets no `msrv`, so `clippy::incompatible_msrv` never fires. The declaration is checked by nobody.

**This is not cosmetic.** `clippy::duration_suboptimal_units` is allowed in `[workspace.lints.clippy]` with the stated justification that its fix (`Duration::from_mins`) would raise the MSRV past 1.80. If 1.80 was never real, that allow may rest on a false premise and should be re-evaluated — see `docs/clippy.md`.

**Decide which way to resolve it**, then make it enforceable:

- *Raise the declaration* to the real floor (at least 1.82) — cheapest, and honest about what the code already does.
- *Hold the floor at 1.80* — revert the five stdlib uses and the two `unsafe extern` blocks, which is real work for a version nothing appears to demand.

Either way, add `msrv` to `clippy.toml` so `clippy::incompatible_msrv` enforces the choice from then on. A CI job on the pinned MSRV toolchain would catch language-feature drift the lint cannot.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rust-version in the root Cargo.toml matches a floor the tree actually compiles on
- [x] #2 clippy.toml sets msrv so clippy::incompatible_msrv enforces the declared version
- [x] #3 The duration_suboptimal_units allow in [workspace.lints.clippy] is re-evaluated against the corrected MSRV, and kept or removed with a comment saying which
- [x] #4 docs/clippy.md's MSRV notes reflect the outcome
- [x] #5 ops verify and ops qa pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Done. `rust-version` is now `1.88`, enforced by `msrv = "1.88"` in `clippy.toml`.

**The decision made itself.** This task offered two options — raise the
declaration, or hold 1.80 by reverting our own 1.82 usage. The second was never
available: the dependency tree already requires **1.88**. `cargo metadata`
declared floors, highest first:

| Floor | Crate |
|---|---|
| 1.88 | `home 0.5.12`, `ignore 0.4.31` |
| 1.87 | `wasip2 1.0.4` |
| 1.86 | `clap-cargo 0.18.3` |
| 1.85.1 | `duckdb`, `libduckdb-sys` |

Both 1.88 crates are genuinely reachable — `cargo tree -i` puts them under
`tokei` → `ops-tokei` → `ops`, not behind an unused feature. So the workspace
could not have built on 1.80 no matter what our own code did, and reverting the
five stdlib calls would have achieved nothing. 1.88 = max(dependency floors,
our own 1.82 usage).

**The `duration_suboptimal_units` allow was deleted rather than kept** (AC #3).
Setting `msrv` subsumes it: the lint is MSRV-aware and silences itself when the
floor cannot accept `Duration::from_mins` (~1.92). Measured directly, holding
the code constant: **9 sites without `msrv`, 0 with `msrv = "1.88"`.** This is
strictly better than the hand-written allow — the constraint is stated once as a
fact, and the lint re-enables itself automatically once the floor passes 1.92.
Because that makes the `msrv` line load-bearing in a non-obvious way, both jobs
are documented in `clippy.toml` so it is not deleted as redundant.

**Setting `msrv` also surfaced a new warning**, which is the mechanism working:
`clippy::unnecessary_debug_formatting` fired at `crates/cli/src/run_cmd.rs:58`
on an `&OsString` — `OsStr::display()` is 1.87, so the suggestion only becomes
applicable once the floor is known to be ≥ 1.87. Given a site allow with the
same reasoning already used at `build.rs:435` and `identity.rs:128`: the value
is not valid UTF-8, which is exactly what the error reports, so `Debug` escaping
is the point.

**Limits of the verification.** No 1.88 toolchain is installed here, so the
floor was not proved by building on it. It rests on two sources: dependency
`rust-version` metadata, and `clippy::incompatible_msrv` over our own code
(silent at 1.88). Neither covers *language* features — `msrv` only checks
library APIs, and the `unsafe extern "C"` blocks in
`crates/core/src/{test_utils.rs,config/edit.rs}` need 1.82 with nothing to flag
them. A CI job pinned to the MSRV toolchain remains the only real proof; CI
currently pins no toolchain at all, which is why this drifted unnoticed.

Gates: `ops verify` 7/7, `ops qa` 3/3. One `ops-core` test failed on the first
`ops qa` run and passed in isolation (325/325) and on re-run — the known
load-sensitive tail tracked by TASK-1664 AC #6, not caused by this change.
<!-- SECTION:NOTES:END -->
