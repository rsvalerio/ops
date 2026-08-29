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
| Each member's `Cargo.toml` → `[lints] workspace = true` | Opt-in. Every workspace member has it |
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

Some lints are worth denying in production and not worth denying in tests: the
cast lints, for instance, where `(i % 256) as u8` in a fixture generator carries
no risk and no information. Cargo's `[lints]` table cannot be `cfg`-gated, so
that relaxation lives in the crate root:

```rust
#![cfg_attr(
    test,
    allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]
```

**A crate root lists only the lints it actually triggers.** There is no standard
block to paste in: write the entry when the crate has a test that fires the
lint, and delete it when that test goes away. An entry that excuses nothing is
not harmless: it tells the next reader the crate does something it does not,
and rewriting it as `#[expect]` to check would report it unfulfilled.

Two consequences follow, and both are normal outcomes rather than gaps:

- **`unwrap` needs no entry.** Layer 1's `allow-unwrap-in-tests = true` in
  `clippy.toml` already relaxes `unwrap_used` for test code across the whole
  workspace, along with `expect`, `panic` and `indexing_slicing`. A crate root
  listing `clippy::unwrap_used` is a leftover; the READ-10 tasks are removing
  them as they are found (TASK-1914, TASK-1968).
- **A crate root may carry no block at all.** `extensions/tokei` and
  `extensions-rust/create-review-tasks` are the worked examples: their tests use
  `unwrap`/`expect`/indexing, all covered by `clippy.toml`, and cast nothing —
  so the block was deleted outright and replaced by a comment saying why.

**Integration test targets under `tests/` are separate crates** and are not
covered by the library's crate root — if one ever needs its own relaxation, it
needs an inner attribute at the top of that file.
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
is why `too_many_arguments` accounts for 5 site-local allows, mostly in the
runner's exec and parallel paths where the arguments are genuinely independent.
Only `expect_used` is more common — see the census below.

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

A census, useful for spotting when one category grows enough to deserve a
policy decision instead. It is a snapshot — regenerate it rather than trusting
it after a batch of fixes:

```bash
grep -rhoE '#!?\[allow\(clippy::[a-z_, :]+\)\]' --include='*.rs' crates/ \
  | sed -E 's/.*allow\((.*)\)\]/\1/' | tr ',' '\n' \
  | sed 's/clippy:://; s/ //g' | sed '/^$/d' | sort | uniq -c | sort -rn
```

(It misses attributes written across several lines — there is one, on
`format_duration`'s clamp in `crates/theme/src/step_line_theme.rs`, counted by
hand below.)

Counts below are as of the temporary-allow drain (TASK-1671..TASK-1682) and the
waves that followed it. Draining the block moved each lint's exception from
layer 1 to layer 3 wherever the code genuinely needs it, which is why the
restriction lints now appear here at all.

| Lint | Sites | Typical reason |
|---|---|---|
| `expect_used` | 9 | Invariant violations with no runtime error channel — see the note below |
| `too_many_arguments` | 5 | Threshold is 5, below clippy's default |
| `trivially_copy_pass_by_ref` | 3 | serde `skip_serializing_if` requires `&T` |
| `option_if_let_else` | 3 | `map_or_else` closures cannot each move the same future, hold the same `&mut` borrow, or read better than the match they replace |
| `literal_string_with_formatting_args` | 3 | `{spinner}` / `{msg}` / `{elapsed}` are `indicatif` template placeholders, not Rust format arguments |
| `as_conversions` | 3 | `u64` ↔ `f64` at the clamp bound: no `From`/`TryFrom` pair expresses it (1 production site, 2 tests asserting the same bound) |
| `unnecessary_debug_formatting` | 2 | `{:?}` is deliberate — the value is not valid UTF-8, which is what the error reports |
| `future_not_send` | 2 | The runner is driven on a current-thread runtime and its event sink is non-`Send` `indicatif` state |
| `case_sensitive_file_extension_comparisons` | 2 | Input is lowercased before comparison |
| `cast_*` | 1 | Documented saturating clamp in `format_duration` (same attribute as its `as_conversions`) |
| others | 1 each | `unnecessary_wraps`, `needless_pass_by_value`, `needless_collect`, `option_option`, `module_inception`, `match_wildcard_for_single_variants`, `match_same_arms` |

`too_many_lines` and `missing_fields_in_debug` have left the census entirely;
the code that needed them was reshaped rather than annotated.

### `expect_used` is the one category worth a policy answer

Nine sites is past "a handful", and it is the only category the drain grew that
far, so it gets an explicit decision rather than an implicit one: **it stays at
layer 3.** The nine are not one pattern repeated by habit; they are four
distinct shapes, none of which has an honest error channel to return into:

- **Compile-time inputs** (4) — a header array literal, a tracing directive, a
  config compiled into the binary, an option cloned out of the collection being
  searched. A miss is an editing mistake caught by the next test run, not a
  runtime condition.
- **Test-harness helpers** (1) — the file-level allow on
  `crates/cli/tests/integration.rs`, which exists only because helper functions
  sit outside `#[test]` bodies and so miss `allow-expect-in-tests` (layer 2).
- **Poisoned locks** (2) — another thread panicked holding `GLOBAL_CONFIG_PATH`;
  no `Option` or `Err` honestly represents "the cache state is unknown".
- **Populated by construction** (2) — a cache built from `Self::iter()`, and an
  `indicatif` template that is the literal `{msg}`.

A workspace-wide `expect_used = "allow"` would erase exactly the distinction
those comments draw, and rule 2 of the drained block forbids it anyway. The
number to watch is the first shape: if compile-time-input `expect`s keep
accumulating, the answer is a small helper that turns "this literal is in this
table" into a type-level guarantee, not a policy relaxation. Revisit at roughly
double the current count.

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
2. Run `ops verify`.
3. Only if the gate is red on test code, add a `#![cfg_attr(test, allow(...))]`
   block to the crate root listing exactly the lints that fired — do not copy
   another crate's block. Most crates need none: `clippy.toml` already relaxes
   `unwrap`/`expect`/`panic`/indexing in tests workspace-wide (layer 2).

## Changing the policy

Edit `[workspace.lints]` in the root `Cargo.toml` and leave a comment saying
why — every entry there has one, and the comments are the point. Then run
`ops verify` and fix or annotate whatever the change surfaces. A policy change
that leaves the gate red is not finished.

## See also

- [AGENTS.md](../AGENTS.md) — the Rust implementation guardrails
- The `code-review-rust` skill — the rule set used for formal Rust review
