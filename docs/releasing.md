# Releasing

This project uses automated release management with two tools:

- **[cocogitto](https://docs.cocogitto.io/)** - Handles version bumps, changelog generation, and git tags based on conventional commits
- **[cargo-dist](https://opensource.axo.dev/cargo-dist/)** - Builds binaries, creates GitHub releases, and publishes to package managers

## How It Works

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Release Workflow                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. Conventional Commits    2. Release Gate          3. Binary Release      │
│  ────────────────────────   ──────────────────       ─────────────────      │
│                                                                             │
│  feat: add new command  ──► CI checks for feat/fix ──► cog bump --auto      │
│  fix: resolve crash         commits since last tag      • CHANGELOG update  │
│  docs: update readme        │                           • Cargo.toml bump   │
│  chore: update deps         │ feat/fix found?           • Git tag           │
│                             │  yes ──► bump + release       │               │
│                             │  no  ──► skip (commits        ▼               │
│                             │         accumulate for   cargo-dist           │
│                             │         next release)    • GitHub release     │
│                                                        • macOS binaries     │
│                                                        • Linux binaries     │
│                                                        • Shell installer    │
│                                                        • Homebrew formula   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
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

Breaking changes trigger a **major** version bump (0.x.x → 1.0.0 or 1.x.x → 2.0.0).

### Examples

Using `git commit`:

```bash
# Feature (minor bump)
git commit -m "feat: add parallel command execution"

# Feature with scope (minor bump)
git commit -m "feat(cli): add --verbose flag"

# Bug fix (patch bump)
git commit -m "fix: prevent crash on empty config"

# Documentation (no bump, included in next changelog)
git commit -m "docs: add installation instructions"

# Breaking change (major bump)
git commit -m "feat!: require explicit stack selection"
```

Or using `cog commit` for guided semantic commits (validates format automatically):

```bash
# Feature (minor bump)
cog commit feat "add parallel command execution"

# Feature with scope (minor bump)
cog commit feat(cli) "add --verbose flag"

# Bug fix (patch bump)
cog commit fix "prevent crash on empty config"

# Documentation (no bump, included in next changelog)
cog commit docs "add installation instructions"
```

## Creating a Release

Releases are fully automated:

### 1. Push Commits to Main

Use conventional commit messages:

```bash
git commit -m "feat: add new theme option"
git push origin main
```

### 2. Automatic Version Bump

After CI passes on `main`, cocogitto runs `cog bump --auto` which:
1. Analyzes conventional commits since the last tag
2. Determines the appropriate version bump (major/minor/patch)
3. **pre_bump_hooks**: runs `cargo set-version` to update `Cargo.toml`
4. Updates `CHANGELOG.md`, creates a version commit and git tag (e.g., `v0.2.0`)
5. **post_bump_hooks**: pushes the commit and tag to remote, which triggers cargo-dist

### Manual Release (Emergency)

If you need to release manually:

```bash
# Install cocogitto and cargo-edit
cargo install cocogitto cargo-edit

# Bump automatically based on commits
# post_bump_hooks handle git push + tag push
cog bump --auto

# Or bump to a specific version
cog bump --version 0.2.0
```

## Supported Platforms

- macOS (Apple Silicon): `aarch64-apple-darwin`
- macOS (Intel): `x86_64-apple-darwin`
- Linux (ARM64): `aarch64-unknown-linux-gnu`
- Linux (x86_64): `x86_64-unknown-linux-gnu`

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

The release workflow needs a `RELEASE_TOKEN` (Personal Access Token) with `contents: write` permission to push tags and version commits.

### HOMEBREW_TAP_TOKEN

The release workflow needs a GitHub Personal Access Token (PAT) to push the Homebrew formula.

#### Step 1: Create a Personal Access Token

1. Go to [GitHub Settings → Developer settings → Personal access tokens → Fine-grained tokens](https://github.com/settings/tokens?type=beta)
2. Click **"Generate new token"**
3. Configure the token:
   - **Token name**: `HOMEBREW_TAP_TOKEN`
   - **Expiration**: 90 days or longer
   - **Repository access**: Select **"Only select repositories"** → `rsvalerio/homebrew-tap`
   - **Permissions**:
     - **Contents**: Read and write
     - **Metadata**: Read-only (auto-selected)
4. Click **"Generate token"**
5. **Copy the token immediately**

#### Step 2: Add as Repository Secret

1. Go to `cargo-ops` repo → **Settings → Secrets and variables → Actions**
2. Click **"New repository secret"**
3. Name: `HOMEBREW_TAP_TOKEN`, Secret: paste token
4. Click **"Add secret"**

#### Rotating the Token

When your token expires:
1. Create a new token (Step 1)
2. Edit the `HOMEBREW_TAP_TOKEN` secret with the new value

## Configuration Files

### cog.toml

Controls version bumping, changelog generation, and tagging:

```toml
from_latest_tag = true
tag_prefix = "v"                   # Tag format: v0.2.0

# Runs cargo-edit to set version in Cargo.toml before bump commit
pre_bump_hooks = [
  "cargo set-version {{version}}",
]

# Pushes commit and tag to remote after bump
post_bump_hooks = [
  "git push",
  "git push origin {{version}}",
]
```

### dist-workspace.toml

Controls binary building and distribution:

```toml
[dist]
cargo-dist-version = "0.31.0"
ci = "github"
installers = ["shell", "powershell", "homebrew"]
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu"]
tap = "rsvalerio/homebrew-tap"
publish-jobs = ["homebrew"]
```

To modify cargo-dist settings, edit `dist-workspace.toml` and run:

```bash
dist generate
```

## Troubleshooting

### No version bump after push

- Check that commits use conventional commit format (`feat:`, `fix:`, etc.)
- Run `cog check` locally to validate commit history
- Check the `release` job logs in GitHub Actions

### Version not bumped correctly

Cocogitto uses these rules:
- `feat` → minor bump
- `fix`, `docs`, `refactor`, etc. → patch bump
- `BREAKING CHANGE` or `!` → major bump

### Homebrew formula not published

- Pre-releases don't publish to Homebrew by default
- Check the `publish-homebrew-formula` job logs
- Verify `HOMEBREW_TAP_TOKEN` is set and valid

### "Resource not accessible by integration" error

The `HOMEBREW_TAP_TOKEN` doesn't have write access. Verify:
- Token has **Contents: Read and write** permission
- Token has access to `rsvalerio/homebrew-tap`

### "Bad credentials" error

The token expired or was revoked. Create a new token and update the secret.
