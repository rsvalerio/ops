# Releasing

This project uses automated release management with two tools:

- **[release-plz](https://release-plz.dev/)** - Handles version bumps, changelog generation, and git tags
- **[cargo-dist](https://opensource.axo.dev/cargo-dist/)** - Builds binaries and publishes to package managers

## How It Works

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Release Workflow                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. Conventional Commits    2. Release PR         3. Binary Release         │
│  ────────────────────────   ─────────────────     ─────────────────         │
│                                                                             │
│  feat: add new command  ──► release-plz creates ──► Merge PR ──► Tag pushed │
│  fix: resolve crash         PR with:                             │          │
│  docs: update readme        • Version bump                       ▼          │
│                             • CHANGELOG update         cargo-dist builds:   │
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
| `docs` | Documentation only | Patch |
| `style` | Code style (formatting, etc.) | Patch |
| `refactor` | Code refactoring | Patch |
| `perf` | Performance improvement | Patch |
| `test` | Adding/updating tests | Patch |
| `build` | Build system changes | Patch |
| `ci` | CI configuration changes | Patch |
| `chore` | Maintenance tasks | Patch |

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

```bash
# Feature (minor bump)
git commit -m "feat: add parallel command execution"

# Feature with scope (minor bump)
git commit -m "feat(cli): add --verbose flag"

# Bug fix (patch bump)
git commit -m "fix: prevent crash on empty config"

# Documentation (patch bump)
git commit -m "docs: add installation instructions"

# Breaking change (major bump)
git commit -m "feat!: require explicit stack selection"
```

## Creating a Release

Releases are automated. Here's the workflow:

### 1. Merge Changes to Main

Push commits with conventional commit messages to the `main` branch:

```bash
git commit -m "feat: add new theme option"
git push origin main
```

### 2. Review the Release PR

After pushing to `main`, release-plz automatically creates or updates a Release PR with:
- Updated version in `Cargo.toml`
- Updated `CHANGELOG.md`

Review the PR to verify:
- The version bump is correct
- The changelog entries are accurate

### 3. Merge the Release PR

When you merge the Release PR:
1. release-plz creates a git tag (e.g., `v0.2.0`)
2. The tag triggers cargo-dist's release workflow
3. cargo-dist builds binaries and publishes everywhere

### Manual Release (Emergency)

If you need to release manually without the PR workflow:

```bash
# Update version in Cargo.toml
# Update CHANGELOG.md
git add -A
git commit -m "chore(release): prepare v0.2.0"
git tag v0.2.0
git push origin main --tags
```

## Supported Platforms

- macOS (Apple Silicon): `aarch64-apple-darwin`
- macOS (Intel): `x86_64-apple-darwin`
- Linux (ARM64): `aarch64-unknown-linux-gnu`
- Linux (x86_64): `x86_64-unknown-linux-gnu`

## Installers Generated

- **Shell script** - `curl`-based installer for Unix systems
- **Homebrew formula** - Published to `rsvalerio/homebrew-tap` (install with `brew install ops`)

## Setup Requirements

### GitHub Actions Permissions

For release-plz to create Release PRs:

1. Go to repo **Settings → Actions → General**
2. Under "Workflow permissions":
   - Select "Read and write permissions"
   - Enable "Allow GitHub Actions to create and approve pull requests"

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

### release-plz.toml

Controls version bumping and changelog generation:

```toml
[workspace]
publish = false              # Don't publish to crates.io
changelog_update = true      # Update CHANGELOG.md
git_release_enable = true    # Create GitHub releases
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

### Release PR not created

- Check that commits use conventional commit format
- Verify GitHub Actions has permission to create PRs
- Check the `release-plz-pr` job logs

### Version not bumped correctly

release-plz uses these rules:
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
