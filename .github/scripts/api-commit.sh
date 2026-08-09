#!/usr/bin/env bash
#
# Replay a local commit as a GitHub-signed commit via the Git Data API.
#
# GitHub renders the green "Verified" badge only for commits carrying a
# signature it can attribute to a key it knows. A commit produced by a plain
# `git commit` and pushed over SSH/HTTPS carries no signature, and a GitHub App
# has no private key we could hand to `git commit` — an App's auto-signing only
# happens for commit objects GitHub itself creates through the API.
#
# So the bump commit is built locally by cog, then re-created here through the
# API (blobs -> tree -> commit -> ref), which makes GitHub sign it and attribute
# it to the App bot. See docs/verified-bump/README.md.
#
# Usage:  api-commit.sh <base-ref> <head-ref> <branch>
#           base-ref   commit to build on top of (becomes the sole parent)
#           head-ref   local commit whose tree changes and message are replayed
#           branch     branch to fast-forward onto the signed commit
#
# Env:    GH_TOKEN  App installation token
#         GH_REPO   owner/repo
#
# Stdout: the new signed commit SHA.

set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 <base-ref> <head-ref> <branch>" >&2
  exit 2
fi

: "${GH_TOKEN:?GH_TOKEN is required}"
: "${GH_REPO:?GH_REPO is required}"

base="$(git rev-parse "$1")"
head="$(git rev-parse "$2")"
branch="$3"

message="$(git log -1 --format=%B "$head")"
base_tree="$(git rev-parse "${base}^{tree}")"

# One tree entry per changed path. --no-renames keeps the status set to A/M/D so
# every case below is handled explicitly rather than silently skipped.
entries='[]'
# -z makes --name-status emit `status NUL path NUL`, so each record is two reads.
while read -r -d '' status && read -r -d '' path; do
  case "$status" in
    A | M)
      mode="$(git ls-tree "$head" -- "$path" | awk '{print $1}')"
      blob="$(jq -n --arg c "$(git cat-file blob "${head}:${path}" | base64 | tr -d '\n')" \
        '{content: $c, encoding: "base64"}' |
        gh api "repos/${GH_REPO}/git/blobs" --input - --jq .sha)"
      entries="$(jq --arg p "$path" --arg m "$mode" --arg s "$blob" \
        '. + [{path: $p, mode: $m, type: "blob", sha: $s}]' <<<"$entries")"
      ;;
    D)
      # A null sha removes the path from the new tree.
      mode="$(git ls-tree "$base" -- "$path" | awk '{print $1}')"
      entries="$(jq --arg p "$path" --arg m "$mode" \
        '. + [{path: $p, mode: $m, type: "blob", sha: null}]' <<<"$entries")"
      ;;
    *)
      echo "unsupported diff status '$status' for '$path'" >&2
      exit 1
      ;;
  esac
done < <(git diff --no-renames --name-status -z "$base" "$head")

if [[ "$(jq 'length' <<<"$entries")" -eq 0 ]]; then
  echo "no file changes between $1 and $2 — nothing to commit" >&2
  exit 1
fi

tree="$(jq -n --arg b "$base_tree" --argjson t "$entries" '{base_tree: $b, tree: $t}' |
  gh api "repos/${GH_REPO}/git/trees" --input - --jq .sha)"

# author/committer are deliberately omitted: GitHub stamps the identity of the
# token's owner (the App bot) and signs the commit with its own key.
commit="$(jq -n --arg m "$message" --arg t "$tree" --arg p "$base" \
  '{message: $m, tree: $t, parents: [$p]}' |
  gh api "repos/${GH_REPO}/git/commits" --input - --jq .sha)"

# force=false so a branch that moved underneath us fails loudly instead of
# discarding whatever landed in the meantime.
jq -n --arg s "$commit" '{sha: $s, force: false}' |
  gh api -X PATCH "repos/${GH_REPO}/git/refs/heads/${branch}" --input - >/dev/null

echo "$commit"
