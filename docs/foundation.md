# Rust foundation

ops is the single source for the config every Rust repo starts from:

| File | What it carries |
|------|-----------------|
| `clippy.toml` | Complexity thresholds, short-ident allow list, and the four `allow-*-in-tests` keys |
| `deny.toml` | The cargo-deny baseline: advisory DB, permissive license allow list, bans, sources |
| `rustfmt.toml` | Stable rustfmt settings (`max_width = 100`, `use_small_heuristics = "Max"`) |
| `.config/nextest.toml` | `leak-timeout = "2s"` and a `ci` profile that writes a JUnit report |
| `mise.toml` | The tool pins CI and laptops share, at forge's versions: the Rust toolchain (with `rustfmt,clippy`), ops (0.77.0 or later, the first check-only `verify`), cargo-deny, cargo-machete, cargo-nextest, cargo-edit, trivy and cargo-llvm-cov, plus the `[tool_alias]` entries mise's registry lacks |
| `Cargo.toml` | The lint policy: `[workspace.lints]` (or `[lints]` for a single package), plus `[lints] workspace = true` in every member |

The templates are embedded in the ops binary (`extensions-rust/foundation/templates/`), so
they update with ops releases. There is nothing to vendor or sync.

## Scaffold: `ops init --rust`

Run from the root of a Rust project:

```bash
ops init --rust           # write whatever is missing
ops init --rust --force   # replace the files and the lint table with the templates
```

Without `--force`, an existing file or lint table is kept and reported as `kept`. With it,
the five files and the root lint table are replaced. Member `[lints]` tables are only
ever added: a member that already has one is left for the check to report.

`clippy.toml` gets `msrv` set to the root's `rust-version` when one is declared. `msrv`
is the repo's own value and not part of the baseline.

`--rust` writes only the foundation. Run `ops init` separately for `.ops.toml`.

## Check: `ops init --rust --check`

Compares the project with the templates in the running ops and exits 1 on drift. It
writes nothing, so it is safe in CI and under `--dry-run`.

`ops foundation` is the same check as a built-in command, so a composite can name it
with no definition of its own, through `[extend.qa] commands = ["foundation"]` for
instance. It re-invokes the running ops, so the version a repo pins decides the
templates, not whatever `ops` is on `PATH`. A `[commands.foundation]` entry left over from
before the built-in is reported as shadowed; delete it.

The comparison is semantic, not textual. Each template is a baseline:

- Every key the template sets must be present with the same value. Comments and
  formatting never count.
- Keys the repo adds are its own, such as `advisories.ignore` entries, a
  `[[licenses.clarify]]` table, or an extra lint.
- Arrays compare as sets. Reordering is fine, but adding or removing an item is drift,
  so a widened license allow list is reported. Admit one crate's license with a
  `[[licenses.exceptions]]` entry instead.
- In `mise.toml`, every tool the template pins must keep its exact version (drift is
  reported at `mise.toml:tools.<tool>`), and tools the repo adds, such as `cocogitto`,
  are its own. Hold a pin back with a waiver on that location. The ops pin is the one
  exception: the template's version is a floor, so any `x.y.z` release from it on
  passes, including the ops running the check, and a leading `v` (`v0.90.1`) is read
  the same way `ops config check` reads it. A pin below the floor, or one that is
  not a plain version such as `latest`, is drift at `mise.toml:tools.ops`.
- A config field newer than the ops pin is drift at `<file>:<field>`, such as
  `.ops.toml:commands.build.raw`, because CI runs the pin and fails on it. This is the
  same check as [`ops config check`](configuration.md#checking-a-config-against-an-ops-version).
- Lint levels only need to be at least as strict: `deny` or `forbid` satisfies the
  template's `warn`.
- `unsafe_code` is policed. The template sets it to `warn`, so a repo with no
  mechanical unsafe policy is drift, while `deny` and `forbid` pass as stricter.
  Scope FFI exceptions with a local `#[allow(unsafe_code, reason = "..")]`, or
  put `#![forbid(unsafe_code)]` at the root of a crate with no unsafe at all.
- In a workspace, every member must have `[lints] workspace = true`, or the policy
  silently does not apply to it.

## Cargo workspaces

The check also compares `[cargo] workspaces` in `.ops.toml` (see
[Several cargo workspaces](configuration.md#several-cargo-workspaces)) with the
cargo workspaces nested in the tree, such as cargo-fuzz's `fuzz/`:

- A nested workspace missing from the list is drift at
  `.ops.toml:cargo.workspaces.<dir>`, because the Rust built-ins skip it. This
  includes a repo that never set the list. Leaving one out on purpose is a
  waiver on that location.
- A listed directory with no `Cargo.toml`, or one that is only a member of the
  root workspace, is drift at the same kind of location.

`ops init` writes the list it finds, so a fresh `.ops.toml` starts with no drift
here.

## Waivers

A deliberate divergence is recorded in `.ops.toml` with its reason:

```toml
[foundation.waivers]
"rustfmt.toml" = "formats with rustfmt defaults until the reformat lands"
"clippy.toml:cognitive-complexity-threshold" = "tightened to 10 to match our guideline"
"Cargo.toml:workspace.lints.clippy.cargo" = "..."
```

A key names a drift location exactly as the check prints it, or a file or table that
contains it. Waived drift is still listed, with its reason. A waiver that matches no
drift is reported as `unused` so it can be removed before it hides something.
