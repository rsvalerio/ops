# Verified bump commits

Spec for making the automated **bump commit and tag** show GitHub's green **Verified**
badge, while remaining authored by the **my-cloud-ci** GitHub App bot rather than
`github-actions[bot]`.

Status: **implemented** — Option A, in [`.github/workflows/bump.yml`](../../.github/workflows/bump.yml)
and [`.github/scripts/api-commit.sh`](../../.github/scripts/api-commit.sh).

## Background

The [Bump workflow](../../.github/workflows/bump.yml) runs `cog bump --auto`
([cocogitto](https://docs.cocogitto.io/)) on `main` after CI passes. cocogitto:

1. computes the next version from conventional commits,
2. runs `cargo set-version` (pre-bump hook),
3. updates `CHANGELOG.md`, **creates a local `git commit`** and an annotated tag,
4. pushes the commit and tag (post-bump hooks).

Authenticating that push with the App installation token changes *who pushed*, not
*whether the commit object is signed*. GitHub renders **Verified** only when the commit
carries a valid signature it can attribute to a key it knows.

## Why the current flow can't be Verified

GitHub marks a commit Verified in exactly two situations:

| How the commit is created | Signed by | Verified? |
|---|---|---|
| Local `git commit`, pushed over HTTPS/SSH | nothing (unless you configure GPG/SSH signing) | ❌ |
| Local `git commit` with a GPG/SSH key whose public half is registered to the committer's GitHub account | that key | ✅ |
| Created through the **GitHub API** (Git Data / Contents) using a token | GitHub's internal key, automatically | ✅ |

cocogitto takes the first row: a plain local commit. A GitHub **App has no private key
you can hand to `git commit`** — the App's auto-signing only happens for commits the App
*creates through the API*. So there is no "just flip a flag on cog" path. We must change
*how the commit object is produced*.

## Options

### Option A — Re-create the bump commit through the GitHub API (recommended)

Let cog compute the version and stage the file changes, but produce the **commit object**
via the Git Data API so GitHub auto-signs it. Tag the same way.

Flow inside `bump.yml`, after minting the App token:

1. **Compute, don't commit.** Run cog so it edits `Cargo.toml` / `CHANGELOG.md` but does
   **not** commit or push. cocogitto has no "stage only" mode, so the simplest reliable
   approach is to let cog make its local commit, then **capture the tree and message and
   re-create the commit via the API**, discarding cog's local commit object. Concretely:
   - `cog bump --auto` (local commit + tag created locally; **drop the `git push` post-bump
     hooks** for the CI path — see "cog config" below).
   - Read back what cog produced:
     - `version="$(cog get-version)"` (or parse the new tag),
     - `msg="$(git log -1 --format=%B)"`,
     - the set of changed paths (`git show --name-only`).
2. **Build a tree + commit via the API** on top of the current `main` head, using the App
   token, so GitHub signs it. Two equivalent implementations:
   - **`gh api` + Git Data API** (`POST /repos/{o}/{r}/git/blobs`, `.../git/trees`,
     `.../git/commits`, then `PATCH .../git/refs/heads/main`). Most control; ~4 calls.
   - **Contents API** (`PUT /repos/{o}/{r}/contents/{path}`) one call per changed file.
     Simpler but each call is a separate commit, so only viable if exactly one file
     changes — not our case (Cargo.toml + Cargo.lock + CHANGELOG.md). **Prefer Git Data.**
3. **Create the tag via the API** (`POST /repos/{o}/{r}/git/refs` with
   `ref=refs/tags/vX.Y.Z` pointing at the new commit SHA). A lightweight tag is enough to
   trigger `release.yml`; if an annotated tag object is wanted, `POST .../git/tags` first
   (those tag objects are signed too) then point the ref at it.
4. **Pushing the ref via the API trigger downstream.** A ref update made with the **App
   token** triggers `release.yml` (unlike `GITHUB_TOKEN`), preserving today's behaviour.

Result: the commit and tag are authored/committed by `my-cloud-ci[bot]`, signed by
GitHub, and render **Verified**.

#### cog config changes

- Drop the `post_bump_hooks` `git push` lines **for the CI path** (the API ref update
  replaces them). Keep them for the manual-release path, or guard with an env check.
  Cleanest: move pushing out of `cog.toml` and into the workflow so cog only computes +
  commits locally.
- Everything else in `cog.toml` (changelog template, commit types, tag prefix) is
  unchanged.

#### Sketch (`bump.yml` step, illustrative — not load-bearing)

```bash
# after: app token minted, cog has made the local bump commit + computed version
ver="$(cog get-version)"            # e.g. 0.4.0
tag="v${ver}"
base="$(git rev-parse HEAD^)"       # main head before cog's local commit
msg="$(git log -1 --format=%B HEAD)"

# Re-create each changed file as a blob, assemble a tree on top of $base, commit via API.
# (Loop over `git diff --name-only ${base} HEAD`, POST blobs+tree, POST commit, PATCH ref.)
# Then: POST /git/refs  refs/tags/${tag} -> new commit sha
```

A small helper script (`ops/scripts/api-commit.sh` or a tiny Rust/JS step) is worth it so
the YAML stays readable. Spec the helper separately if we adopt this.

#### Trade-offs

- ➕ Fully Verified, App-bot authored, downstream release still triggers.
- ➖ More moving parts than a `git push`; the "let cog commit then re-create via API" dance
  is a little awkward. Mitigated by isolating it in a helper script with tests.
- ➖ Must keep the API tree in lockstep with whatever files cog touches (today:
  `backend/Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`). Deriving the file list from
  `git diff --name-only` rather than hardcoding avoids drift.

### Option B — GPG-sign cog's local commit

Generate a GPG key, register its public half on a **real GitHub account** (the App's bot
account cannot hold GPG keys), import the private key into the runner with
[`crazy-max/ghaction-import-gpg`](https://github.com/crazy-max/ghaction-import-gpg), and
set `git config commit.gpgsign true` + `user.signingkey`.

- ➕ cog flow stays as-is (still a local commit + `git push`).
- ➖ Verified badge attributes to **the human account that owns the GPG key**, not the App
  bot — directly conflicts with the "not a person, use the bot" goal.
- ➖ A long-lived private key to store as a secret and rotate. The whole point of the App
  token was to retire static secrets.

**Rejected** for those two reasons; documented only so we don't re-discover it.

### Option C — Accept Unverified

Keep today's flow (now correctly attributed to `my-cloud-ci[bot]` after the author change).
Commits stay "Unverified". Zero additional work.

- ➕ Nothing to build or maintain.
- ➖ No green badge; if the `main` ruleset ever requires signed commits, the bypass actor
  must continue to cover the bot (it does today).

## What was implemented

Option A — the only path that satisfies both goals (App-bot author **and** Verified)
without introducing a static signing secret.

1. `cog.toml` has **no `post_bump_hooks`**: cog computes the version, updates the files,
   and creates a local commit + annotated tag, but pushes nothing.
2. [`.github/scripts/api-commit.sh`](../../.github/scripts/api-commit.sh) replays a local
   commit through the Git Data API: one blob per changed path, a tree on top of the base
   commit's tree, a commit object, then a `PATCH` of the branch ref. `author`/`committer`
   are omitted so GitHub stamps the token owner (the App bot) and signs the result.
   Deletions are sent as a tree entry with a `null` sha; the ref update uses `force: false`
   so a branch that moved underneath the job fails loudly.
3. `bump.yml` runs cog, then calls the helper with `HEAD^ HEAD <branch>` and creates
   `refs/tags/vX.Y.Z` pointing at the returned signed SHA. Because the ref update uses the
   App token rather than `GITHUB_TOKEN`, `release.yml` still fires on the tag.

### Verifying a change to this flow

The next bump commit on `main` should render **Verified**, be authored by
`my-cloud-ci[bot]`, and be followed by a `release.yml` run on the new tag. When changing
`api-commit.sh`, exercise it against a stub `gh` on a scratch repo first — the tree
assembly has to survive added/modified/deleted paths, executable (`100755`) modes, binary
content, and paths containing spaces.

### Known sharp edge: `--skip-ci`

`cog.toml` defines `skip_ci = "[skip ci]"` but `bump.yml` deliberately does **not** pass
`--skip-ci`. GitHub honours `[skip ci]` in the head commit message of *any* push including
**tag** pushes, so a bump commit carrying the marker would suppress `release.yml` and no
binaries would ship. The re-triggered CI run on the bump commit is the accepted cost: the
follow-up bump job finds no commits after the new tag and stops there.

## Related

- Author attribution: the bump commit is authored and committed by the App bot because the
  Git Data API stamps the token's owner. The `git config user.*` step in `bump.yml` only
  labels cog's throwaway local commit.
- Human commits land Verified via your own SSH signing key — but GitHub's **"Rebase and
  merge"** button re-creates commits server-side and drops their signatures. The repo has
  `allow_rebase_merge` disabled for that reason; use squash or a merge commit.
- See [`../releasing.md`](../releasing.md) for the end-to-end release flow.
