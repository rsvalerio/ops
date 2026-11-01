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

## Checking a config against an ops version

CI usually installs the `ops` pinned in `mise.toml`, while a local `ops` built from source
can be newer. A field newer than the pin then passes every local gate and fails every `ops`
step in CI. `ops config check` finds that without installing the pinned version:

```console
$ ops config check --as 0.85.1
.ops.toml: commands.lint-all.args (beside clone) needs ops 0.86.0
ops 0.85.1 (--as) rejects 1 field(s).
Minimum ops version for this config: 0.86.0
```

Without `--as` it targets the `ops` pin in the project's mise configuration: `[tools]` in
`mise.toml`, `.mise.toml`, `mise/config.toml`, `.mise/config.toml`, `.config/mise.toml` or
`.config/mise/config.toml` — in mise's precedence order, the last file naming a pin wins —
under the `ops` key or, when the project has no `[tool_alias]`, its backend key
`github:rsvalerio/ops`. Local (`*.local.toml`), `conf.d`, parent-directory, global and
`.tool-versions` pins are not read: they are not committed, so CI never installs them. A pin
that names no exact release, such as `latest` or `0.85`, is an error until you pass `--as`.
With no pin and no `--as` it prints only the minimum. It exits non-zero when the target
rejects a field.

The check reads `.ops.toml` and `.ops.d/*.toml`, the files a project commits. It knows the
release each field became loadable, including a field that gained a new place (`args` beside
`clone`), and the release a value arrived where its field is older (`theme = "flux"` needs
0.17.0; a theme your own `[themes]` defines is yours and needs nothing). Only the value in
effect counts: a later file replaces an earlier one's setting — a fragment's
`theme = "classic"` makes `.ops.toml`'s `"flux"` the loader's, and the check's too. A
behaviour change that no field or value marks is deliberately out of the check's scope: no
record of one exists to read, so where behaviour matters, run the pinned `ops` itself.

`ops` records each field's release in `crates/core/src/config/history.rs`, and a test fails
until a new field is listed there.

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

### `ops check-workspaces`

A nested workspace is a workspace root of its own and inherits nothing: it
resolves its own `Cargo.lock` from scratch and copies the root lint policy by
hand, and nothing makes cargo keep either in step. `ops check-workspaces`
holds each one to the root, read-only:

- **Lock agreement.** Every `(name, version, source)` the nested `Cargo.lock`
  pins for a crate the root `Cargo.lock` also holds must be the triple the
  root pins — the nested graph may be smaller and carry crates the root has
  none of, but not resolve a different release of a shared crate. Beyond
  versions, the dependency edges of a shared package must agree: an edge the
  nested lockfile records and the root does not is drift, and for a crate
  whose manifest is in the repo every required (non-optional, non-dev)
  dependency the manifest declares must be an edge in both lockfiles.
- **Lints sync.** The nested `[lints]` tables must be the root
  `[workspace.lints]` tables, in both directions, compared as policies so a
  reflowed copy of a table is the same table.

A deliberate difference is recorded, with its reason, under
`[cargo] lint_exemptions` — an exemption licenses the named workspace
*omitting* the lint, never setting it to something else, and never keeping a
lint the root has dropped:

```toml
[cargo.lint_exemptions.fuzz]
"rust.unsafe_code" = "libfuzzer's fuzz_target! expands to an extern \"C\" entry point that is unsafe by construction"
```

Lockfile drift is fixed by re-seeding: `cp Cargo.lock fuzz/Cargo.lock`, then
any cargo command inside `fuzz/` to re-resolve. The command is not part of
the default gates; add `check-workspaces` to `verify` or `qa` as a step, the
way any other check is wired.

You rarely write the list by hand. `ops init` in a Rust project looks for
nested workspaces and writes the list with a comment on each entry, and
`ops init --rust --check` reports when the list and the tree disagree (see
[Rust foundation](foundation.md#cargo-workspaces)). A `Cargo.toml` below the
root counts as a workspace when it declares its own `[workspace]` table, or
when it is a package that its nearest enclosing workspace (a nested one, else
the root) names in `[workspace].exclude` (matched literally, not as a glob).
A package inside a nested workspace that does not exclude it is that
workspace's member. The search honours `.gitignore`, skips hidden
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
- Scalar fields beside `clone` (`help`, `category`, `aliases`, `raw`, and for exec sources `env`, `cwd`, `timeout_secs`, `exclusive`, `strategy`, `after`, `wrap`) override the copy; fields left unset keep the source's value. Given maps and lists replace the copy (`env` replaces, it does not merge; an empty `after` or `wrap` clears the copied one). `aliases` are the exception: they are never inherited — a clone with no `aliases` has none, because inheriting the source's would either collide at load or silently redirect the source's alias to the clone. `program` and `commands` beside `clone` are load errors — extra composite entries go through `[extend.<name>] commands`.
- A clone copies the source **before** the source's own `[extend.<source>]` applies: extends stay per-name, so the clone never inherits them. `[extend.<clone>]` applies to the materialized copy.
- Unknown sources, clone cycles (including self-clones; non-cyclic clone-of-clone chains do resolve), cloning into an existing stack-default name, and exec-only fields beside a composite source are load errors naming the command and the source.

## Command categories

`category = "..."` sets the heading a command appears under in `ops --help`; a command
without one appears under **Commands**. It affects nothing else.

There is deliberately no default category, neither a `[commands]`-level or per-section
default nor one inherited from a composite. `[commands]` keys are command names, so a
default there needs a reserved name or a new section whose scope across config layers
would have to be defined for a help heading. Inheritance from a composite is ambiguous:
one command can sit in several composites, as `clippy` sits in both `verify` and
`verify-fix`, and a heading that changes when a composite gains a member is worse than a
repeated line. Writing `category` on each command stays explicit and greppable, and
`clone` already carries it to a copy.

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

## Ordering steps with `after`

`exclusive` means "nothing else at the same time". To say only "not before that
step", give the step an `after` list. It starts once every named step in the
same parallel plan has finished, and nothing else waits:

```toml
[commands.next]
program = "cargo"
args = ["nextest", "run"]
after = ["build"]    # reuse build's artifacts instead of racing it for the cargo lock

[commands.verify]
commands = ["fmt-check", "clippy", "build", "next", "lint-sync"]
parallel = true
# runs: fmt-check | clippy | build | lint-sync, and next once build has finished
```

- A name that is not in the plan is no constraint, so a step can share its
  `after` across plans. Under `fail_fast = true` a failed dependency skips the
  steps waiting on it, directly or through a chain. Under `fail_fast = false`
  they run once it has finished, like every other step.
- `exclusive` still splits the stages. A dependency in an earlier stage has
  already finished. A dependency in a later stage is an error: list it before
  the exclusive step between them.
- A step moves ahead of the steps listed before it only as far as `after`
  needs, within its stage, so `--raw` and `ops --dry-run` show a valid order.
  `after` has no effect in a sequential group, where the list is the order, or
  on a step run alone.
- A matrix step is one step here too: it waits as one, and its dependents wait
  for every cell.
- Each name must be an exec command; a group is not a step, so name its steps.
  Cycles fail the config load, naming the commands. Unknown names and groups
  fail any plan expansion (`ops <cmd>`, `--dry-run`, `ops explain`) before a
  step starts, because extension commands only register after the config loads.
  Commands that expand no plan (`ops about`, `ops config`, …) do not check
  `after` names at all — a deliberate limit: validating the names needs the
  extension and builtin stores, which those commands never load, and a typo in
  one step's list would otherwise fail commands that run nothing from the
  plan. The typo surfaces on the first plan-expanding invocation instead.
- `ops explain` prints the edges (`next after build`), and `--json` lists them
  as `edges` on the stage.

## Running a command inside a wrapper

Some commands need services or environment set up around them. `wrap` runs an
exec command inside a wrapper script, keeping the real command in `program` and
`args` rather than burying it after a `--`:

```toml
[commands.next-all]
clone = "next"
args = ["--run-ignored", "all"]
wrap = ["./scripts/e2e-services.sh", "--pg", "--bao", "--"]
```

`wrap` is prepended when the command spawns, after `clone`, `[extend]` and every
other load step have settled the args, so the line above runs
`./scripts/e2e-services.sh --pg --bao -- cargo nextest run ... --run-ignored all`.
The wrapper is an ordinary command line: the first entry is spawned, `${VAR}`
expands as in `args`, and `timeout_secs` covers the wrapper. Each matrix cell
runs inside its own wrapper, and `${matrix.<key>}` in `wrap` is a load error.
`ops --dry-run` shows `wrap` apart from `program` and `args`, and `ops explain`
prints `next-all wrapped by ...` (`wrap` in `--json`). Step lines show the full
command, wrapper first.

`wrap` is exec-only: a group has no single process to wrap. To keep one service
lifetime across several steps, wrap an `ops` invocation of a sequential group:

```toml
[commands.release-steps]
commands = ["release-nextest", "embedded-example"]

[commands.release-tests]
program = "${OPS_BIN}"
args = ["release-steps"]
wrap = ["./scripts/e2e-services.sh", "--pg", "--bao", "--"]
```

`${OPS_BIN}` is a builtin variable (like `OPS_ROOT`) that expands to the
absolute path of the *running* `ops` binary — the same resolution ops uses for
its own internal re-invocations. A bare `program = "ops"` would instead resolve
through `PATH` at spawn time, so a stale `~/.cargo/bin` copy, a direnv shim, or
a wrapper that prepends directories to `PATH` before re-invoking it silently
substitutes a different ops version; `${OPS_BIN}` names this one, and it cannot
be overridden from the environment. `${VAR}` expands in `program` as in `args`
(spawn and `--dry-run` both show the resolved path; step lines show the
reference).

The services outlive both steps, and each step stays a real command that can be
cloned and run on its own. ops deliberately has no `setup`/`teardown` commands
on a group: teardown has to survive failures, timeouts and cancellation, which
a wrapper script's `trap` already does, and the wrapper is the one place that
owns the services' lifetime.

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
