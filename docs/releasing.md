# Releasing

This project uses automated release management with two tools:

- **[cocogitto](https://docs.cocogitto.io/)** - Handles version bumps, changelog generation, and git tags based on conventional commits
- **[cargo-dist](https://opensource.axo.dev/cargo-dist/)** - Builds binaries, creates GitHub releases, and publishes to package managers

## How It Works

```
┌────────────────────────────────────────────────────────────────────────────┐
│                              Release Workflow                              │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  1. Feature Branch + PR     2. CI (6 parallel checks)  3. Merge to Main    │
│  ────────────────────────   ──────────────────────────  ─────────────────  │
│                                                                            │
│  git checkout -b feat/x     Format ─┐                  PR merged to main   │
│  git commit (conventional)  Check  ─┤                       │              │
│  git push + open PR         Lint   ─┼─► all must pass       ▼              │
│       │                     Build  ─┤   to merge       4. Auto Bump        │
│       ▼                     Test   ─┤   ──────────────────────────────     │
│  Ruleset enforces:          Deps   ─┘   cog bump --auto                    │
│  • required status checks                • CHANGELOG update                │
│  • review thread resolution              • Cargo.toml bump                 │
│  • signed commits                        • Git tag (v*.*.*)                │
│                                               │                            │
│                                               ▼                            │
│                                          5. Binary Release                 │
│                                          ─────────────────                 │
│                                          cargo-dist                        │
│                                          • GitHub release                  │
│                                          • macOS/Linux binaries            │
│                                          • Shell installer                 │
│                                          • Homebrew formula                │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

## Conventional Commits

This project uses [Conventional Commits](https://www.conventionalcommits.org/) to automatically determine version bumps and generate changelogs.

### Commit Format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Commit Types

| Type | Description | Version Bump |
|------|-------------|--------------|
| `feat` | New feature | Minor (0.1.0 → 0.2.0) |
| `fix` | Bug fix | Patch (0.1.0 → 0.1.1) |
| `doc` | Documentation only | No bump (included in next changelog) |
| `docs` | Documentation only | No bump (included in next changelog) |
| `style` | Code style (formatting, etc.) | No bump (included in next changelog) |
| `refactor` | Code refactoring | No bump (included in next changelog) |
| `perf` | Performance improvement | No bump (included in next changelog) |
| `test` | Adding/updating tests | No bump (included in next changelog) |
| `build` | Build system changes | No bump (included in next changelog) |
| `ci` | CI configuration changes | No bump (included in next changelog) |
| `chore` | Maintenance tasks | No bump (included in next changelog) |

### Breaking Changes

For breaking changes, add `!` after the type or include `BREAKING CHANGE:` in the footer:

```bash
# Using ! suffix
feat!: remove deprecated config option

# Using footer
feat: change config format

BREAKING CHANGE: The config format has changed from YAML to TOML.
```

Breaking changes normally trigger a **major** version bump (e.g. 1.x.x → 2.0.0).

**0.y.z caveat:** With `cog bump --auto`, Cocogitto [does not move a 0.y.z project to 1.0.0 automatically](https://docs.cocogitto.io/guide/bump.html), even if commits include breaking changes. When you are ready to leave **0.x**, bump explicitly, for example `cog bump --version 1.0.0`.

Because of this, a breaking change on `0.x` lands as a **patch** release. The changelog still renders a `BREAKING` badge, but the version number gives no warning — so say what breaks, and how to fix it, in the PR description and in a note on the changelog entry. v0.36.1 is a worked example.

#### Squash merges drop your commit subjects

Both merge styles are enabled on this repo, and cocogitto reads them differently:

| Merged via | What cog parses | Does `!` on your commit count? |
|---|---|---|
| **Merge commit** | your individual commits — `ignore_merge_commits = true` in `cog.toml` makes cog skip the merge commit itself | **Yes.** `feat!:` or a trailing `BREAKING CHANGE:` footer works exactly as above. |
| **Squash** | the squash commit, whose subject is the **PR title**; your commit subjects become `*` bullets in its body | **No.** A `!` that appears only on an individual commit is not in the subject cog reads. |

So the guidance above holds for merge commits, and is not enough on its own for squash.

**Rule: put `!` in both the individual commit type and the PR title.** It costs nothing and is correct under either merge style, and you rarely control which one gets used at merge time.

```text
commit:    fix(runner)!: reject composite plans with conflicting scheduling flags
PR title:  fix!: composite scheduling correctness and CI supply-chain hardening
```

**Worked example — PR #14 ([`81e514d`](https://github.com/rsvalerio/ops/commit/81e514d2c5b0d2baac1564c92259aae5887e705f)).** The breaking commit was correctly written `fix(runner)!:` with a `BREAKING CHANGE:` footer, but the PR title was a plain `fix:`. Squash-merging made that title the commit subject and demoted the `!` into a body bullet. The changelog badge still rendered, so the break was flagged — but the `!` was not where it needed to be for the subject to carry it.

## Creating a Release

Releases are fully automated. The `main` branch is protected by a GitHub ruleset that requires all status checks to pass and review threads to be resolved before merging.

### 1. Create a Feature Branch and PR

Use conventional commit messages on a feature branch:

```bash
git checkout -b feat/my-feature
git commit -m "feat: add new theme option"
git push -u origin feat/my-feature
gh pr create
```

### 2. CI Status Checks (7 parallel jobs)

The [CI workflow](../.github/workflows/ci.yml) runs on every PR and produces seven status checks that must all pass before merge. **Workflow Guard** is new — if the branch ruleset names its required checks explicitly, add it there so it can block merge:

| Check | Command | Description |
|-------|---------|-------------|
| **Format** | `ops fmt` | Format all code |
| **Check** | `ops check` | Check all targets |
| **Lint** | `ops clippy` | Lint with clippy |
| **Build** | `ops build` | Build all targets |
| **Test** | `ops test` | Run all tests |
| **Deps** | `cargo deny check` | Check dependencies, advisories, licenses, bans and sources |
| **Workflow Guard** | `grep` over `.github/workflows/` | Fail if any action is not SHA-pinned, or any workflow uses `secrets: inherit` |

CI installs `cargo-edit`/`cargo-deny` via [`taiki-e/install-action`](https://github.com/taiki-e/install-action) (for the Deps job). The Deps job invokes `cargo deny` directly rather than through `ops`; it does not install `ops` itself.

All third-party actions are pinned to full commit SHAs with a trailing `# vX.Y.Z` comment. The **Workflow Guard** check enforces this, and also rejects `secrets: inherit`.

Both matter most for `release.yml`, which dist would otherwise emit with bare tags and blanket secret forwarding. Note that `dist plan` does not merely regenerate that file — it asserts the on-disk contents byte-match its own output and hard-fails the release otherwise. Since the hardening makes them differ permanently, `dist-workspace.toml` sets `allow-dirty = ["ci"]`, which takes `release.yml` out of dist's hands entirely.

**Consequence:** bumping `cargo-dist-version` no longer updates `release.yml` by itself. To pick up a new dist's CI changes, temporarily remove `allow-dirty`, run `dist init`, then re-apply the SHA pins and the explicit `secrets:` block and restore the key. Workflow Guard fails the build if you forget either.

### 3. Merge PR to Main

Once all checks pass and review threads are resolved, merge the PR into `main`.

### 4. Automatic Version Bump

The [Bump workflow](../.github/workflows/bump.yml) runs when the **CI** workflow completes successfully on `main` (`workflow_run`). It only runs for this upstream repo (`github.repository_owner == 'rsvalerio'`); forks do not auto-bump.

The job runs `cog bump --auto`, which:
1. Analyzes conventional commits since the last tag
2. Determines the appropriate version bump (major/minor/patch)
3. **pre_bump_hooks**: runs `cargo set-version` to update `Cargo.toml`
4. Updates `CHANGELOG.md`, creates a version commit and git tag (e.g., `v0.2.0`) — **locally only**, `cog.toml` has no post-bump hooks

The workflow then publishes that commit itself. `.github/workflows/bump.yml` is a thin wrapper around [forge's shared bump workflow](https://github.com/rsvalerio/forge/blob/v1/.github/workflows/bump.yml), which replays cog's local commit through the GraphQL `createCommitOnBranch` mutation and points the tag at the result. Only that mutation yields a signature — GitHub signs a commit solely when *it* builds the commit object, so `git push` and both REST endpoints all produce `verified=false`. The bump commit therefore lands **Verified**, attributed to the `my-cloud-ci[bot]` App. The tag ref itself is unsigned, which is normal and invisible in the UI; it uses the App token, which is what lets it trigger cargo-dist (`GITHUB_TOKEN` would not). See [forge's verified-bump notes](https://github.com/rsvalerio/forge/blob/v1/docs/verified-bump.md).

Bump commits carry `[skip ci]`, so they no longer re-trigger the CI + Bump cycle. `cog.toml` always defined `skip_ci`, but it applies only when `cog bump` is passed `--skip-ci`, which the old inline workflow never did.

If there is nothing to release (no `feat` / `fix` / breaking commits since the last tag), `cog bump --auto` does not create a new version commit or tag.

> **Note:** The bump bot pushes directly to `main` via the bypass actor in the ruleset (RepositoryRole actor_id 1 with `bypass_mode: always`).

### Manual Release (Emergency)

If you need to release manually:

```bash
# Install cocogitto and cargo-edit
cargo install cocogitto cargo-edit

# Bump automatically based on commits (commits and tags locally only)
cog bump --auto

# Or bump to a specific version
cog bump --version 0.2.0

# cog.toml has no post_bump_hooks, so push the commit and tag yourself.
# Note these will be Unverified unless your own commit signing is configured.
git push
git push origin "v$(cog get-version)"
```

## Supported Platforms

- macOS (Apple Silicon): `aarch64-apple-darwin`
- macOS (Intel): `x86_64-apple-darwin`
- Linux (ARM64): `aarch64-unknown-linux-gnu`
- Linux (x86_64): `x86_64-unknown-linux-gnu`

## GitHub Release workflow (cargo-dist)

[`release.yml`](../.github/workflows/release.yml) is generated by `dist generate`. It is triggered by **`workflow_dispatch` with a `tag` input**, not by pushing a tag — `dispatch-releases = true` in `dist-workspace.toml`. The Bump workflow dispatches it automatically once the version tag exists; to release by hand, `gh workflow run release.yml --ref main -f tag=v0.36.0`. The same workflow also runs on **pull requests** so `dist plan` can validate configuration without publishing.

The tag push cannot be the trigger, because bump commits carry `[skip ci]` and GitHub applies that marker per *push event* — a tag push included. Left tag-triggered, the version tag lands and nothing builds, which is exactly how `v0.36.0` was swallowed. Dispatching separates the two: the marker still suppresses CI, and the release is asked for explicitly.

The Homebrew formula is pushed to the tap by a **custom publish job**: `publish-jobs = ["./publish-homebrew"]` in `dist-workspace.toml` makes the generated `release.yml` call the user-owned reusable workflow [`publish-homebrew.yml`](../.github/workflows/publish-homebrew.yml) (as job `custom-publish-homebrew`, with the dist plan as input and `secrets: inherit`). That file is **not** managed by `dist generate`, so the GitHub App token-mint step it contains survives regeneration — no hand edits to `release.yml` needed.

> `dist` prints `WARN A Homebrew tap was specified but the Homebrew publish job is disabled` because it doesn't know the custom job handles the tap push. This is expected; do **not** add `"homebrew"` back to `publish-jobs` — that would reintroduce the built-in job using the retired `HOMEBREW_TAP_TOKEN` PAT.

## Installers Generated

- **Shell script** - `curl`-based installer for Unix systems
- **Homebrew formula** - `brew install rsvalerio/tap/ops` (repository: `rsvalerio/homebrew-tap`).
  - Alternative two step install:
    ```bash
       brew tap rsvalerio/tap; \
       brew install ops
    ```

## Setup Requirements

### GitHub Actions Permissions

The release workflow needs a `WORKFLOW_TOKEN` (Personal Access Token) with `contents: write` permission to push tags and version commits.

### Homebrew tap push (GitHub App token)

The custom publish job ([`publish-homebrew.yml`](../.github/workflows/publish-homebrew.yml)) mints a short-lived installation token from the **my-cloud-ci GitHub App** via `actions/create-github-app-token` — no static PAT, nothing to rotate. It requires:

- **Repository variable** `GH_APP_CLIENT_ID` — the GitHub App's **Client ID** (the `Iv23li…` string on the App's General settings page, *not* the numeric App ID). `actions/create-github-app-token` deprecated its `app-id` input in favour of `client-id`.
- **Repository secret** `GH_APP_PRIVATE_KEY` — the App's private key (PEM)
- The App installed on `rsvalerio/homebrew-tap` with **Contents: Read and write**

(The previous `HOMEBREW_TAP_TOKEN` fine-grained PAT is retired and can be deleted from repo secrets.)

## Configuration Files

### cog.toml

Controls version bumping, changelog generation, and tagging. The repo file also sets `ignore_merge_commits`, `skip_untracked`, `skip_ci`, a `[changelog]` block (remote GitHub template, authors), and `[commit_types]` for section titles—see [`cog.toml`](../cog.toml) for the full source of truth.

```toml
from_latest_tag = true
tag_prefix = "v"                   # Tag format: v0.2.0

# Runs cargo-edit to set version in Cargo.toml before bump commit
pre_bump_hooks = [
  "cargo set-version {{version}}",
]

# Empty on purpose: the Bump workflow publishes via the GitHub API so the
# commit and tag are Verified. A `git push` here would land an unsigned copy first.
post_bump_hooks = []
```

### dist-workspace.toml

Controls binary building and distribution. The workspace root file includes a `[workspace]` section and additional `[dist]` keys (`install-path`, `hosting`, `install-updater`, `formula` for the Homebrew name `ops`); see [`dist-workspace.toml`](../dist-workspace.toml) for the full file.

```toml
[dist]
cargo-dist-version = "0.31.0"
ci = "github"
installers = ["shell", "homebrew"]
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu"]
install-path = "CARGO_HOME"
hosting = "github"
install-updater = false
tap = "rsvalerio/homebrew-tap"
publish-jobs = ["./publish-homebrew"]  # custom job: .github/workflows/publish-homebrew.yml
formula = "ops"                    # brew install ops
```

To modify cargo-dist settings, edit `dist-workspace.toml` and run:

```bash
dist generate
```
