# Clippy and Lint Policy

Lint policy for this workspace is centralized. A crate does not choose its own
lint levels — it opts into the shared policy and, where it genuinely needs an
exception, records that exception next to the code with a reason.

`ops verify` runs the gate:

```bash
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

Warnings are errors. Either the code changes or the exception is written down.
The one bounded exception is the temporary-allow block described below, which
only ever shrinks.

## Where policy lives

| File | Owns |
|---|---|
| `Cargo.toml` → `[workspace.lints]` | Which lints are on, and which are allowed workspace-wide |
| `clippy.toml` | Lint thresholds, and the `msrv` that gates version-sensitive lints |
| Each member's `Cargo.toml` → `[lints] workspace = true` | Opt-in. Every one of the 28 members has it |
| Each crate root (`lib.rs` / `main.rs`) | The `#[cfg(test)]` relaxations |
| Individual call sites | `#[allow(...)]` with a comment saying why |

A member that omits `[lints] workspace = true` silently drops out of the
policy — its code compiles under default lint levels and the gate still passes.
Adding it is the first step when creating a crate; see the checklist at the end.

## The three layers

Exceptions are granted at the narrowest layer that works.

```text
┌─ [workspace.lints]  ── whole workspace, every file
│  └─ crate root #![cfg_attr(test, allow(..))]  ── test code in one crate
│     └─ #[allow(..)] at the item  ── one function, one statement
```

Reach for the innermost layer first. A workspace-level `allow` turns a lint off
everywhere, including in code written years from now by someone who never read
this page, so it needs to be justified as policy rather than as convenience.

### Layer 1 — workspace

`[workspace.lints.rust]`:

| Lint | Level |
|---|---|
| `elided_lifetimes_in_paths` | warn |
| `unsafe_op_in_unsafe_fn` | warn |
| `unused_lifetimes` | warn |

`rust_2018_idioms` is deliberately **not** enabled as a group. It implies
`unused_extern_crates`, which flags the `extern crate` lines in
`crates/cli/src/main.rs`. Those lines are load-bearing: they exist only to stop
the linker discarding extension crates that register through linkme distributed
slices and are otherwise unreferenced from the binary. Removing them drops those
extensions from the build with no compile error and no test failure — the
extension simply stops appearing in `ops extension list`.

`[workspace.lints.clippy]`:

| Lint | Level | Notes |
|---|---|---|
| `all` | deny | priority `-1` so specific entries can override |
| `pedantic` | deny | priority `-1`, same reason |
| `nursery` | deny | priority `-1`, same reason |
| `unwrap_used` | deny | Production code has no `unwrap`. See below |
| `arithmetic_side_effects` | deny | Integer `+ - * / %` carries a `checked_*`/`saturating_*` form or a proof (TASK-1671) |
| `as_conversions` | deny | `as` between integer widths is a `TryFrom`/`From` conversion or carries a proof (TASK-1674) |
| `unimplemented` | deny | |
| `unchecked_time_subtraction` | deny | |
| `todo` | deny | |
| `panic` | deny | |
| `exit` | deny | Only `main` decides the process exit code |
| `indexing_slicing` | deny | `v[i]` / `&v[a..b]` panic out of bounds. Use `get`, `first`, `last`, slice patterns (TASK-1672) |
| `string_slice` | deny | `&s[a..b]` panics off a UTF-8 char boundary. Use `get`, `split_at_checked`, `split_once`, `char_indices` (TASK-1673) |

The panic-adjacent lints are relaxed for test code through `clippy.toml` rather
than through a crate-root attribute, because the relaxation is policy for the
whole workspace:

```toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
allow-indexing-slicing-in-tests = true
```

`string_slice` has no `allow-*-in-tests` key of its own, so the handful of test
helpers that slice a `&str` by byte index go through `get(..idx).expect(..)`
instead — `expect` in a test is already the sanctioned failure mechanism
(layer 2 below).

#### The temporary-allow block (drained)

Turning `nursery` on, together with the rest of the panic and arithmetic lints,
surfaced 948 pre-existing sites. Rather than water the policy down, those lints
sat in a clearly fenced `# --- Temporary allows ---` block at the bottom of
`[workspace.lints.clippy]`, each line carrying its backlog task ID and its site
count:

```toml
arithmetic_side_effects = "allow" # TASK-1671 — 166 sites, 54 files
```

Two rules governed that block:

1. **It only shrinks.** Each task's final acceptance criterion is deleting its
   own line. A lint that leaves the block never comes back to it.
2. **Nothing new goes in.** A lint that fires on code written from now on is a
   code problem, not a policy problem — grant the exception at layer 2 or 3,
   next to the code that needs it, with the reason written down.

It was the one place in this policy where "fix it later" existed, and it was
bounded: TASK-1671 through TASK-1682 drained it, and **the block is now gone**.
Rule 2 outlives it — there is no longer anywhere in `Cargo.toml` for a new
workspace-wide exception to go.

Deleting a line was only ever enough for lints that a group already enables.
The `restriction`-group entries — `arithmetic_side_effects`, `as_conversions`,
`indexing_slicing`, `string_slice`, `expect_used`, `unreachable`,
`panic_in_result_fn` — belong to no group `[workspace.lints.clippy]` turns on,
so removing an `allow` line drops that lint back to clippy's default of `allow`
and the gate stays silently green. Those lints had to leave the block by moving
up to the explicit `deny` list above it, next to `unwrap_used` and `panic`. Any
restriction lint this workspace adopts from now on goes there directly.

### Layer 2 — test code

`unwrap_used` is the reason this layer exists. Banning `unwrap` in production is
worth it; banning it in tests is not, where `unwrap` on a fixture is the clearest
way to say "this cannot fail and I want a panic if it does".

Cargo's `[lints]` table cannot be `cfg`-gated, so the relaxation lives in each
crate root:

```rust
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]
```

The three cast lints are here for the same reason: `(i % 256) as u8` in a
fixture generator carries no risk and no information.

All 28 crate roots carry this block. **Integration test targets under `tests/`
are separate crates** and are not covered by the library's crate root — if one
ever needs `unwrap`, it needs its own inner attribute at the top of that file.
`crates/cli/tests/integration.rs` uses `expect` in its helper functions, which
sit outside `#[test]` bodies and so are not covered by `allow-expect-in-tests`
either; it carries its own `#![allow(clippy::expect_used)]` for that reason
(TASK-1675).

### Layer 3 — call site

For a specific place where the lint is wrong. Always with a comment giving the
reason, on the line above:

```rust
// serde's `skip_serializing_if` predicates are always called with `&T`.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_default_columns(v: &u16) -> bool {
    *v == AUTO_COLUMNS
}
```

An `#[allow]` with no comment is treated as an unfinished change. The lint fired
for a reason; the comment is where you say why that reason does not apply here.

## Workspace-wide allows

There is no deferred group any more — `pedantic` is enforced in full apart
from the three entries below. The documentation lints (`doc_markdown`,
`must_use_candidate`, `missing_errors_doc`, ~730 sites) were cleared under
TASK-1668, and `needless_pass_by_value` under TASK-1666.

### Style this codebase does not share

| Lint | Why it is off |
|---|---|
| `items_after_statements` | Local `const`, `type` and helper declarations are written at their point of use, which reads better than hoisting them |
| `struct_excessive_bools` | clap flag structs are legitimately bool-heavy |
| `similar_names` | High false-positive rate, no defect signal |

These are settled decisions, not deferrals. Re-litigate them only with a
concrete bug the lint would have caught.

## `clippy.toml`

```toml
cognitive-complexity-threshold = 25
too-many-arguments-threshold = 5
type-complexity-threshold = 250
msrv = "1.97"
```

`too-many-arguments-threshold = 5` is stricter than clippy's default of 7, which
is why `too_many_arguments` is the most common site-local allow (5 sites, mostly
in the runner's exec and parallel paths where the arguments are genuinely
independent).

### `msrv` earns its keep

It must stay equal to `rust-version` in the root `Cargo.toml`. It does two jobs:

- `clippy::incompatible_msrv` fails the gate on any standard-library call newer
  than the declared floor, so the declaration stops silently drifting.
- MSRV-aware lints gate their *suggestions* on it, proposing a newer API only
  once the floor can accept it.

`duration_suboptimal_units` shows the second job. It wants
`Duration::from_mins` / `from_hours`, stabilized in **1.92** — measured, not
guessed: with the floor at 1.90 it is silent, at 1.92 it fires. While the floor
sat at 1.88 the lint was suppressed and the timeout constants were written as
`from_secs(900)`; at the current 1.97 floor it is enforced and they read
`from_mins(15)`.

This is why no `duration_suboptimal_units = "allow"` entry exists. A
hand-written allow would need revisiting by hand every time the floor moved;
`msrv` states the constraint once and the lint follows it automatically.

**`msrv` does not cover language features** — only library APIs. The `unsafe
extern "C"` blocks (Rust 1.82) in
`crates/core/src/{test_utils.rs,config/edit.rs}` would not be flagged against a
lower floor by any lint. Only compiling on the floor catches those, which is
what the **MSRV** job in `.github/workflows/ci.yml` does: it reads
`rust-version` out of `Cargo.toml`, installs exactly that toolchain, and runs
`cargo check --all --all-features --all-targets`.

That job also asserts `clippy.toml`'s `msrv` equals `Cargo.toml`'s
`rust-version`. If the two disagree, the lint and the build disagree about the
floor and each lets through what the other rejects. It reads the version rather
than hardcoding it, so the workflow cannot itself become a second place to
drift.

## Current site-local allows

A rough census, useful for spotting when one category grows enough to deserve a
policy decision instead:

| Lint | Sites | Typical reason |
|---|---|---|
| `too_many_arguments` | 5 | Threshold is 5, below clippy's default |
| `trivially_copy_pass_by_ref` | 3 | serde `skip_serializing_if` requires `&T` |
| `unnecessary_wraps` | 2 | Signature fixed by a fn-pointer type or a sibling's contract |
| `too_many_lines` | 2 | 101 lines against a 100 limit |
| `case_sensitive_file_extension_comparisons` | 2 | Input is lowercased before comparison |
| `unnecessary_debug_formatting` | 3 | `{:?}` is deliberate — the value is not valid UTF-8, which is what the error reports |
| `needless_pass_by_value` | 1 | `expand_err_to_io` is used point-free as `map_err(f)` at four sites |
| `cast_*` | 1 | Documented saturating clamp in `format_duration` |
| others | 1 each | `option_option`, `module_inception`, `missing_fields_in_debug`, `match_wildcard_for_single_variants`, `match_same_arms` |

## Traps

### `cargo clippy --fix` is not safe to run unattended

It applies suggestions mechanically, including ones that change behavior. Both
of these were caught in review, not by the gate:

- **It deleted the `extern crate` lines** in `crates/cli/src/main.rs` under
  `unused_extern_crates`. The build stayed green and the tests stayed green; the
  `git` and `text-fixers` extensions just stopped being linked in. Verify with
  `ops extension list` after any change near those lines.
- **It raised the MSRV** by rewriting 9 timeout constants to
  `Duration::from_mins`.

Read the diff. `--fix` is a starting point for the mechanical bulk, not a
finished change.

### An arm-level `#[allow]` does not always apply

`match_wildcard_for_single_variants` is reported against the `match` expression,
not the arm, so an attribute on the arm is ignored. Put it on the enclosing
statement:

```rust
#[allow(clippy::match_wildcard_for_single_variants)]
let results = match spec {
    // ...
};
```

### `--all-features` matters

The gate uses `--all-features`; a plain `cargo clippy` misses feature-gated code
entirely. Run the same command the gate runs, or run `ops verify`.

## Adding a new crate

1. Add `[lints]` / `workspace = true` to its `Cargo.toml`. Without this the
   crate is outside the policy and nothing will tell you.
2. Add the `#![cfg_attr(test, allow(...))]` block to its crate root, copied from
   any existing crate.
3. Run `ops verify`.

## Changing the policy

Edit `[workspace.lints]` in the root `Cargo.toml` and leave a comment saying
why — every entry there has one, and the comments are the point. Then run
`ops verify` and fix or annotate whatever the change surfaces. A policy change
that leaves the gate red is not finished.

## See also

- [AGENTS.md](../AGENTS.md) — the Rust implementation guardrails
- The `code-review-rust` skill — the rule set used for formal Rust review
