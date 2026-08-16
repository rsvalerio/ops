# Clippy and Lint Policy

Lint policy for this workspace is centralized. A crate does not choose its own
lint levels — it opts into the shared policy and, where it genuinely needs an
exception, records that exception next to the code with a reason.

`ops verify` runs the gate:

```bash
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

Warnings are errors. There is no "fix it later" tier — either the code changes
or the exception is written down.

## Where policy lives

| File | Owns |
|---|---|
| `Cargo.toml` → `[workspace.lints]` | Which lints are on, and which are allowed workspace-wide |
| `clippy.toml` | Numeric thresholds for lints that take one |
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
| `all` | warn | priority `-1` so specific entries can override |
| `pedantic` | warn | priority `-1`, same reason |
| `unwrap_used` | warn | Production code has no `unwrap`. See below |

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
`crates/cli/tests/integration.rs` currently uses `expect` throughout, so it
needs nothing.

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

Each of these is off everywhere. They fall into three groups.

### Deferred — should eventually be enabled

| Lint | Sites |
|---|---|
| `doc_markdown` | 409 |
| `must_use_candidate` | 210 |
| `missing_errors_doc` | 113 |

Just over 700 sites across 28 crates, almost all of it doc comments and
`#[must_use]` attributes. They are off so the *rest* of `pedantic` could be
enforced now instead of waiting on a documentation sweep. Turning any one of
them on is a self-contained piece of work.

To re-measure before picking one up:

```bash
cargo clippy --workspace --all-features --all-targets -- -W clippy::doc_markdown
```

`needless_pass_by_value` is also deferred, but for a different reason — its 19
sites need signature *and* call-site changes, several constrained by the
`'static` bounds on the runner's tokio spawns. Tracked as **TASK-1666**, which
also removes the allow.

### Constrained by the toolchain

| Lint | Why it is off |
|---|---|
| `duration_suboptimal_units` | Its fix is `Duration::from_mins` / `from_hours`, stabilized around Rust 1.92. The workspace declares `rust-version = "1.80"`, so taking the suggestion raises the MSRV |

Worth knowing: **`rust-version = "1.80"` is already inaccurate.** The tree uses
`unsafe extern "C"` blocks, which need 1.82+. Nothing in CI or `clippy.toml`
enforces MSRV, so the drift is invisible. If that ever gets fixed, re-check this
allow — it may no longer be needed.

### Style this codebase does not share

| Lint | Why it is off |
|---|---|
| `items_after_statements` | Local `const`, `type` and helper declarations are written at their point of use, which reads better than hoisting them |
| `struct_excessive_bools` | clap flag structs are legitimately bool-heavy |
| `similar_names` | High false-positive rate, no defect signal |

These are settled decisions, not deferrals. Re-litigate them only with a
concrete bug the lint would have caught.

## Thresholds

`clippy.toml` tunes lints that take a number:

```toml
cognitive-complexity-threshold = 25
too-many-arguments-threshold = 5
type-complexity-threshold = 250
```

`too-many-arguments-threshold = 5` is stricter than clippy's default of 7, which
is why `too_many_arguments` is the most common site-local allow (5 sites, mostly
in the runner's exec and parallel paths where the arguments are genuinely
independent).

## Current site-local allows

A rough census, useful for spotting when one category grows enough to deserve a
policy decision instead:

| Lint | Sites | Typical reason |
|---|---|---|
| `too_many_arguments` | 5 | Threshold is 5, below clippy's default |
| `trivially_copy_pass_by_ref` | 3 | serde `skip_serializing_if` requires `&T` |
| `unnecessary_wraps` | 2 | Signature fixed by a fn-pointer type or a sibling's contract |
| `unnecessary_debug_formatting` | 2 | `{:?}` is deliberate — the path is not valid UTF-8, which is what the error reports |
| `too_many_lines` | 2 | 101 lines against a 100 limit |
| `case_sensitive_file_extension_comparisons` | 2 | Input is lowercased before comparison |
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
