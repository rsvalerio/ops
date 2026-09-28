# Rust foundation

ops is the single source for the config every Rust repo starts from:

| File | What it carries |
|------|-----------------|
| `clippy.toml` | Complexity thresholds, short-ident allow list, and the four `allow-*-in-tests` keys |
| `deny.toml` | The cargo-deny baseline: advisory DB, permissive license allow list, bans, sources |
| `rustfmt.toml` | Stable rustfmt settings (`max_width = 100`, `use_small_heuristics = "Max"`) |
| `.config/nextest.toml` | `leak-timeout = "2s"` and a `ci` profile that writes a JUnit report |
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
the four files and the root lint table are replaced. Member `[lints]` tables are only
ever added: a member that already has one is left for the check to report.

`clippy.toml` gets `msrv` set to the root's `rust-version` when one is declared. `msrv`
is the repo's own value and not part of the baseline.

`--rust` writes only the foundation. Run `ops init` separately for `.ops.toml`.

## Check: `ops init --rust --check`

Compares the project with the templates in the running ops and exits 1 on drift. It
writes nothing, so it is safe in CI and under `--dry-run`.

The comparison is semantic, not textual. Each template is a baseline:

- Every key the template sets must be present with the same value. Comments and
  formatting never count.
- Keys the repo adds are its own, such as `advisories.ignore` entries, a
  `[[licenses.clarify]]` table, or an extra lint.
- Arrays compare as sets. Reordering is fine, but adding or removing an item is drift,
  so a widened license allow list is reported. Admit one crate's license with a
  `[[licenses.exceptions]]` entry instead.
- Lint levels only need to be at least as strict: `deny` or `forbid` satisfies the
  template's `warn`.
- In a workspace, every member must have `[lints] workspace = true`, or the policy
  silently does not apply to it.

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
