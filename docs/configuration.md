# Configuration

How `.ops.toml` defines, extends, clones, groups and schedules commands. For the
commands each stack ships out of the box, see [Commands](commands.md) and
[Stack command mappings](command-mappings.md).

Create a `.ops.toml` file in your project root (or run `ops init`):

```toml
[output]
theme = "classic"        # "classic" (default) or "compact"
columns = 80             # line width for step lines (omit to auto-size: 90% of terminal width, 80 without one)
show_error_detail = true # show error details below failed steps

[commands.build]
program = "cargo"
args = ["build", "--all-targets"]

[commands.test]
program = "cargo"
args = ["test"]

[commands.verify]
commands = ["fmt", "check", "clippy", "build"]
parallel = true
fail_fast = true

[commands.qa]
commands = ["test", "deps"]
parallel = true
fail_fast = true
```

Config is merged in order (later overrides earlier): built-in defaults → global config (`~/.config/ops/config.toml`) → local `.ops.toml` → `.ops.d/*.toml` fragments (sorted by filename) → `OPS__*` environment variables. When run inside a project with a detected stack (e.g. Rust), `ops init` pre-fills stack-specific commands.

## Extending existing commands

To add steps to a command that already exists — typically a stack default like the Rust `verify` — without copying (and going stale on) its whole `commands` list, use an `[extend.<name>]` section:

```toml
[commands.coverage]
program = "cargo"
args = ["llvm-cov"]

[extend.verify]
commands = ["coverage"]   # appended to the end of verify's commands list
```

Exec commands extend the same way, with `args` instead of `commands`:

```toml
[extend.clippy]
args = ["--locked"]       # added to clippy's args, before any `--` separator
```

The extras are appended at load time. Rules:

- Composites (`commands = [...]`) extend with `commands`; exec commands extend with `args`, and matrix commands also with `matrix.<key> = [...]` (see [Running one command over a matrix](#running-one-command-over-a-matrix)). Using the wrong key for the target's kind, extending an undefined name, or an entry that sets none of `commands`, `args`, `help`, `category`, `matrix` is a load error naming the target.
- `help = "..."` replaces the target's help text, and `category = "..."` replaces its category (either kind of target). Given across config layers, the last layer that sets one wins.
- Without a `help` override, appending `commands` to a composite that has help extends the help to name the appended commands (`"...; then extra-a, extra-b"`), so `ops --help` can never quietly understate what `ops <cmd> --dry-run` runs. The same guard applies across layers: a layer that appends commands without setting `help` gets its commands named in whatever help is in effect, so a later command-only layer cannot hide behind an earlier override. A composite without help needs nothing — its help fallback already renders the materialized command list.
- Appended `args` land **before the target's first `--` separator** when one is present, otherwise at the end of the args. Cargo commands like the Rust `clippy ... -- -D warnings` pass everything after `--` to the wrapped tool, so inserting before it keeps `--locked` a cargo flag instead of silently turning it into a lint flag.
- A locally redefined command wins: `[extend.verify]` appends to *your* `[commands.verify]` if you defined one, otherwise to the stack default.
- Extends concatenate across config layers, so `.ops.d/*.toml` fragments stack on top of `.ops.toml` appends.
- Extending controls list order only, not execution order. Each appended command keeps the `exclusive` flag of its own definition. In a sequential group it runs after the earlier steps. In a parallel group (see below), an appended non-exclusive command joins the final stage and may run concurrently with the earlier non-exclusive steps. Mark it `exclusive = true` if it must not overlap them.

## Locked cargo commands

CI should build against the committed `Cargo.lock`. One switch adds `--locked` to every cargo command that resolves the lockfile (`build`, `check`, `clippy`, `doc`, `test`, `nextest`, `run`, `bench`) — the Rust stack defaults and your own `[commands]` alike:

```toml
[cargo]
locked = true
```

Like any key it can come from the environment instead, which keeps local runs free to update the lockfile: `OPS__CARGO__LOCKED=true`. The flag goes before any `--` separator, commands already passing `--locked` or `--frozen` are left alone, and it applies after `[extend]`. It is off by default because `--locked` refuses to create a missing `Cargo.lock`.

## Several cargo workspaces

A repository can hold more than one cargo workspace: cargo-fuzz's `fuzz/` is
its own `[workspace]`, listed in the root's `exclude`, so `cargo fmt --all` and
`cargo clippy --workspace` never reach it. Name every workspace the Rust
defaults should cover, as directories relative to the project root (`.` is the
root):

```toml
[cargo]
workspaces = [".", "fuzz"]
```

- `fmt`, `fmt-check`, `check`, `clippy` and `build` each become a
  [matrix](#running-one-command-over-a-matrix) over a `workspace` axis, with
  `--manifest-path ${matrix.workspace}/Cargo.toml` added before any `--`
  separator. Each stays one step in `verify` and `verify-fix`, and runs once
  per workspace in rows labelled `clippy [workspace=.]`,
  `clippy [workspace=fuzz]`. `ops explain verify` names every cell, and
  `ops --dry-run clippy` shows each one's args.
- `ops deps` (and `ops deps --check`) runs its tools in each workspace's
  directory and prints one report per workspace, titled with it. cargo-deny
  finds the root `deny.toml` from a nested workspace by walking up. Every
  workspace is reported before the command fails, naming the ones with issues.
- The test commands (`test`, `test-doc`, `next`, …) and `doc` stay root-only.
- A command you define yourself under one of those names is left alone,
  because its args may already name a package or manifest. A verbatim copy of
  the default, as `ops init --commands` writes it, still counts as the default
  and is expanded. A stack default you extend with `[extend.<name>] args` is
  expanded with your args kept. `[cargo] locked` applies to every cell.

An entry must be a relative directory inside the project: an empty entry, an
absolute path, a `..` component, or a directory or `Cargo.toml` that resolves
outside the project through a symlink fails the load. A list that is empty or
names only `.` changes nothing.

You rarely write the list by hand. `ops init` in a Rust project looks for
nested workspaces and writes the list with a comment on each entry, and
`ops init --rust --check` reports when the list and the tree disagree (see
[Rust foundation](foundation.md#cargo-workspaces)). A `Cargo.toml` below the
root counts as a workspace when it declares its own `[workspace]` table, or
when it is a package that the root's `[workspace].exclude` names (matched
literally, not as a glob). The search honours `.gitignore`, skips hidden
directories, `target/` and `vendor/`, and goes 8 levels deep. Other nested
manifests are root members or fixtures. ops only searches when you run those
two commands, never on an ordinary run: the plan changes when `.ops.toml`
changes, not when a directory appears.

## Cloning existing commands

To define a command as a variant of an existing one — typically a stack default — without copying (and going stale on) its whole spec, use `clone`:

```toml
[commands.fuzz-clippy]
clone = "clippy"
args = ["--manifest-path", "fuzz/Cargo.toml"]
```

`fuzz-clippy` is a copy of the resolved Rust `clippy` default (program and args included), and `args` adds the fuzz-specific flag — so the variant tracks the default's flags as they evolve. Composites clone the same way (`clone = "verify"` copies the `commands` list). The extras are materialized at load time, so `ops --dry-run fuzz-clippy` shows the resolved program and args. Rules:

- The source resolves like an `[extend]` target: your `[commands]` entry if you defined one, otherwise the detected stack's default. Extension-registered commands cannot be cloned — they register after config load — and naming one is an unknown-source load error.
- `args` beside `clone` is appended to the source's args, not substituted for them, inserted before any `--` exactly like `[extend.<name>] args` (exec sources only). `[extend.<clone>] args` still works too; if both are set, the inline `args` come first and the extend's after them.
- Scalar fields beside `clone` (`help`, `category`, `aliases`, `raw`, and for exec sources `env`, `cwd`, `timeout_secs`, `exclusive`, `strategy`) override the copy; fields left unset keep the source's value. Given maps and lists replace the copy (`env` replaces, it does not merge). `aliases` are the exception: they are never inherited — a clone with no `aliases` has none, because inheriting the source's would either collide at load or silently redirect the source's alias to the clone. `program` and `commands` beside `clone` are load errors — extra composite entries go through `[extend.<name>] commands`.
- A clone copies the source **before** the source's own `[extend.<source>]` applies: extends stay per-name, so the clone never inherits them. `[extend.<clone>]` applies to the materialized copy.
- Unknown sources, clone cycles (including self-clones; non-cyclic clone-of-clone chains do resolve), cloning into an existing stack-default name, and exec-only fields beside a composite source are load errors naming the command and the source.

## Command groups and scheduling

A command with a `commands = [...]` list is a *group* (composite). Groups may
reference other groups, and each group runs under its own `parallel` flag:

- **Sequential group (`parallel = false`, the default):** each entry runs as its
  own plan, one after another, under that entry's own schedule — exactly what
  typing the entries on the command line does. A parallel child group runs its
  steps concurrently (split into stages at `exclusive` steps, below); a
  sequential child runs its own entries one at a time. So
  `pre-release = ["verify", "deps", ...]` means `ops verify deps ...`: `verify`
  keeps the staged parallel schedule it gets when invoked directly, and a hook
  group such as `run-before-commit = ["verify", ...]` runs `verify` in parallel
  rather than one step at a time.
- **Parallel group (`parallel = true`):** the whole tree under it is one flat
  plan. Every group inside it must also declare `parallel = true` — a nested
  `parallel = false` group's steps would run concurrently despite the flag, so
  the config is rejected with an error naming both groups:

```toml
[commands.fixers]
commands = ["ruff", "black"]
parallel = false

[commands.verify]
commands = ["fixers", "pyright"]
parallel = true          # error: conflicts with fixers.parallel = false
```

```console
$ ops verify
error: conflicting `parallel` in the plan for `verify`: `verify` sets parallel = true,
but `fixers` sets parallel = false
```

To fix, set `verify.parallel = false` so each entry keeps its own schedule (the
nested `fixers` group then runs in whatever order it declares itself), or keep
it parallel and mark the steps that must not overlap `exclusive = true` (below).
Inside one parallel plan every group must also declare the same `fail_fast`.

`fail_fast` follows the same boundary rule:

- Within one parallel plan, every group must agree on `fail_fast` (the plan is
  scheduled as one unit).
- Across a sequential group's entries, values may differ: each entry's own
  `fail_fast` governs its steps, and the sequential group's own `fail_fast`
  governs the sequence — when false, every entry runs regardless of failures;
  when true (the default), a failing entry stops the entries after it, unless
  that entry itself declares `fail_fast = false` (it asked to run through its
  own failures, so the sequence continues past it). This mirrors what naming
  several commands on one invocation (`ops run verify qa`) does, where each
  name keeps its own flags.

## Exclusive steps in a parallel group

Running every step of a parallel group at once is wrong for a step that
rewrites files the others read. Mark such an exec command `exclusive = true`.
The plan is then split into ordered stages at each exclusive step: the
exclusive step runs alone, and each run of consecutive non-exclusive steps
between them runs concurrently. Stages follow the order of `commands`.

```toml
[commands.fmt]
program = "cargo"
args = ["fmt", "--all"]
exclusive = true

[commands.verify]
commands = ["fmt", "clippy", "build", "doc"]
parallel = true
# runs: fmt → (clippy | build | doc)
```

The list order is the schedule. With `a` and `c` exclusive,
`["a", "b", "c", "d"]` runs `a → b → c → d`, while `["a", "c", "b", "d"]` runs
`a → c → (b | d)`. Under `fail_fast = true` a failing stage stops the plan and
later stages never start. `exclusive` has no effect in a sequential group or
under `--raw`, which always runs sequentially.

## Raw commands

Some commands need the terminal: an interactive confirmation, a password
read, a pager. The captured-output display path breaks those — the prompt
never reaches your tty. Mark such a command `raw = true` and `ops` runs it
as if you had passed `--raw`: child stdio is inherited directly (the prompt
works), execution is sequential, and no progress display, `--tap` file or
`--junit` report is produced:

```toml
[commands.cf-apply]
program = "scripts/cf-zone.sh"
args = ["apply"]
help = "Apply the drift cf-check found: dry-run, confirm, write (never deletes)"
raw = true          # the confirmation prompt needs the terminal
category = "Cloudflare"
```

Raw mode is a property of the whole run, not of one step: `raw = true` on
any command in the invocation's composite tree — including one reached
through nested groups — promotes every named command of that run to raw
mode. `--tap` and `--junit` beside it are ignored with a warning, and
`parallel = true` groups run sequentially, exactly as under the `--raw`
flag. A group can set `raw = true` itself to promote anything that names it.
No effect under `--dry-run`, which never executes children.

## Running one command over a matrix

To run one exec command once per value — say `cargo doc` per published crate,
each invocation separate so cargo resolves every crate's default features on
its own — give it a `strategy`, modelled on GitHub Actions:

```toml
[commands.doc-default]
program = "cargo"
args = ["doc", "--no-deps", "-p", "${matrix.crate}"]
env = { CARGO_TARGET_DIR = "${CARGO_TARGET_DIR:-target}/doc-default" }

[commands.doc-default.strategy]
matrix = { crate = ["dbsec-core", "dbsec", "dbsec-pgwire", "dbsec-vault", "dbsec-derive"] }
max_parallel = 1     # the cells share a target dir; running them together only queues on its lock
fail_fast = false    # run every crate and report every failure

[extend.verify]
commands = ["doc-default"]
```

`ops doc-default` runs `cargo doc --no-deps -p dbsec-core`, then `-p dbsec`, and
so on, and shows one row per cell, labelled by its values
(`doc-default [crate=dbsec-core]`). When cells fail, each failed row shows its
error and the step fails naming every failed cell. `ops --dry-run doc-default`
lists every cell with its expanded program and args.

A matrix command is **one step** to the plan around it:

- `exclusive` covers the whole matrix: no sibling step overlaps any cell.
- It succeeds only when every cell succeeds.
- `strategy.max_parallel` (default: every cell at once; cells always count
  against the same `OPS_MAX_PARALLEL` process cap as the plan's other steps)
  and `strategy.fail_fast` (default `true`: the first
  failing cell cancels the rest) govern only the cells. They never conflict
  with the enclosing group's flags. The `fail_fast` agreement rule applies to
  groups, not to a matrix, so the `fail_fast = false` matrix above can sit
  inside the parallel, fail-fast `verify`. It runs all five crates, and
  `verify` treats the matrix's overall failure like any failing step.
- `--raw` runs the cells one after another, like everything else.

Cells follow GitHub Actions semantics:

- Several keys form the Cartesian product. Cells are ordered by key name, then
  by value order.
- `exclude = [{ os = "mac", crate = "dbsec" }]` drops every cell that matches
  all of an entry's pairs.
- Each `include` entry is merged into every cell it does not contradict on a
  matrix key. When it fits none, it becomes a cell of its own. `include` and
  `exclude` live inside `matrix`, as in GitHub Actions, so neither name can be
  a key.

`${matrix.<key>}` is substituted in `args`, `env` values and `cwd`, before
`${VAR}` expansion. It never falls back to the environment. These are load
errors naming the command: a key that some cell does not define
(`${matrix.crat}`), a `${matrix.*}` reference on a command with no `strategy`,
and a reference in `program`. A matrix is capped at 256 cells.

A clone copies the source's `strategy`, and a `strategy` beside `clone`
replaces the copied one wholesale. `[extend.<name>] matrix.<key> = [...]`
appends values to an existing key (a key the matrix does not have is a load
error). `matrix.include` / `matrix.exclude` there append entries. A matrix
over a composite is not supported.
