---
id: TASK-1847
title: >-
  SEC-14: a symlinked global config aborts the entire layered load, silently
  discarding the repo's own .ops.toml
status: To Do
assignee:
  - TASK-1983
created_date: '2026-08-27 15:24'
updated_date: '2026-08-28 14:09'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/core/src/config/loader/global.rs
  - crates/core/src/config/loader/mod.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader/global.rs:227-237` (`load_global_config_at`), `crates/core/src/config/loader/mod.rs:103` (`read_capped_toml_file_with`), `crates/core/src/config/loader/mod.rs:213` (`load_config_at`)

**What**: The SEC-25 / TASK-1468 symlink refusal is scoped in its own comment to the *untrusted repo* threat:

```rust
// SEC-25 (TASK-1468): refuse to follow symlinks at config paths. An
// adversarial repo planting `.ops.toml -> /etc/shadow` would otherwise
// be slurped into the TOML parser and echoed back through diagnostics.
let mut file = match open_refusing_symlinks(path) {
```

But `read_config_file` is also the reader for the **global** config at `~/.config/ops/config.toml`, and `open_refusing_symlinks` applies `O_NOFOLLOW` unconditionally — so *any* symlink there, malicious or not, returns `InvalidInput` rather than `NotFound`. `load_global_config_at` converts that into a hard error:

```rust
for path in &[toml_path, bare_path] {
    match super::read_config_file(path) {
        Ok(Some(overlay)) => { ...; return Ok(()); }
        Ok(None) => {}
        Err(e) => return Err(e),      // <-- symlink refusal lands here
    }
}
```

and `load_config_at` propagates it with `?` **before** the local layers are read at all:

```rust
global::load_global_config(&mut config).context("loading global config")?;

let local_path = workspace_root.join(".ops.toml");
if let Some(overlay) = read_config_file(&local_path)... { merge_config(&mut config, overlay); }
conf_d::merge_conf_d(&mut config, workspace_root)...;
```

`load_config_or_default_with` then substitutes `Config::empty()`. Reproduced against the built binary with a symlinked global config:

```
$ XDG_CONFIG_HOME=$D/xdg ops --help
ops: warning: failed to load config (early): loading global config: failed to open
  config file: ".../xdg/ops/config.toml": refusing to follow symlink at ".../xdg/ops/config.toml"
ops: warning:     continuing with an empty config (no commands, themes, or stack)
```

**Why it matters**: `~/.config/**` being a symlink is the *normal* state for anyone using GNU Stow, chezmoi, or nix home-manager — the mainstream ways of managing dotfiles. And the failure is not "skip the global layer": the `?` aborts the chain, so the user loses every command, theme, and stack setting from the repo's own `.ops.toml` and `.ops.d` because of a symlink in their home directory. The two paths are not the same trust boundary — the repo-local path is attacker-controlled, `$HOME` is the user's own and is not a privilege boundary — so applying one policy to both trades a real regression for no security gain.

Fix direction: scope the refusal to workspace-relative config paths, or (weaker but sufficient) have the global layer skip-with-`tracing::warn!` instead of failing the whole chain, so a bad global layer can never take the local ones down with it.

<!-- scan confidence: verified by reading global.rs:227-237 + loader/mod.rs:103,213, and reproduced against the built release binary -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A symlinked ~/.config/ops/config.toml no longer aborts the load; the repo's .ops.toml and .ops.d are still read and merged
- [ ] #2 The symlink refusal is still enforced for workspace-relative config paths (.ops.toml, .ops.d/*.toml), with the existing SEC-25 tests unchanged
- [ ] #3 The distinction between the two trust boundaries is documented where open_refusing_symlinks is called, so the next reader does not re-widen the policy
- [ ] #4 A regression test symlinks the global config to a real file and asserts a repo .ops.toml command is present in the loaded Config
<!-- AC:END -->
