//! The root [`Config`] type: its fields, construction, and validation.
//!
//! Command-spec validation lives here rather than in [`super::commands`]
//! because cycle and unknown-reference checks need the whole command map.

use super::commands::CommandSpec;
use super::sections::{AboutConfig, DataConfig, ExtensionConfig, OutputConfig};
use super::theme_types::ThemeConfig;
use super::tools::ToolSpec;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Maximum recursion depth for composite expansion. Mirrors the runner's
/// `MAX_DEPTH` so the same configs that are accepted at load time are also
/// accepted at run time.
pub const MAX_COMPOSITE_DEPTH: usize = 100;

/// Root configuration structure.
///
/// TRAIT-4 / TASK-0872: `Default` is **gated to test/test-support builds**
/// so a buggy production CLI path cannot silently fall back to a blank
/// `Config` (no commands, no themes, etc.) instead of going through
/// [`load_config_or_default`]. Production code that genuinely needs a
/// blank-slate Config (the load-failure degradation, init-template
/// scaffolding) calls [`Config::empty`] explicitly so the choice is visible
/// at the call site. The user-visible defaults (theme = "classic", etc.)
/// come from `.default.ops.toml` via the loader.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub commands: IndexMap<String, CommandSpec>,
    #[serde(default, skip_serializing_if = "DataConfig::is_default")]
    pub data: DataConfig,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub themes: IndexMap<String, ThemeConfig>,
    #[serde(default, skip_serializing_if = "ExtensionConfig::is_default")]
    pub extensions: ExtensionConfig,
    #[serde(default, skip_serializing_if = "AboutConfig::is_default")]
    pub about: AboutConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub tools: IndexMap<String, ToolSpec>,
}

impl Config {
    /// Construct a blank `Config` for the documented degradation paths
    /// ([`load_config_or_default`] fallback, [`init_template`] scaffolding).
    /// Production code that wants user-visible defaults should call
    /// [`load_config`] / [`load_config_or_default`] instead.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            output: OutputConfig::default(),
            commands: IndexMap::default(),
            data: DataConfig::default(),
            themes: IndexMap::default(),
            extensions: ExtensionConfig::default(),
            about: AboutConfig::default(),
            stack: None,
            tools: IndexMap::default(),
        }
    }

    /// Validate all command specs. Called after loading to fail fast on invalid config.
    ///
    /// Validates exec specs unconditionally. Composite specs are not checked
    /// here because composite commands may reference stack defaults or
    /// extension-registered commands that are not known at config load time —
    /// see [`Config::validate_commands`] for full composite validation.
    ///
    /// # Errors
    ///
    /// If any exec spec fails [`ExecCommandSpec::validate`]. Composite specs are
    /// not checked here — see [`Config::validate_commands`].
    pub fn validate(&self) -> anyhow::Result<()> {
        for (name, spec) in &self.commands {
            if let CommandSpec::Exec(exec) = spec {
                exec.validate(name)?;
            }
        }
        Ok(())
    }

    /// Validate exec specs and every composite's references against the
    /// merged set of `config.commands` plus `externals` (stack defaults +
    /// registered extension command ids).
    ///
    /// Catches three failure modes that would otherwise only surface when
    /// the user invokes the affected command:
    /// - unknown reference (typo such as `commands = ["buidl"]`)
    /// - cycle (self-reference or indirect cycle)
    /// - depth violation (deeper than [`MAX_COMPOSITE_DEPTH`])
    ///
    /// Does not stand up a [`crate::runner::CommandRunner`]; the caller
    /// passes in the externally-known ids explicitly, so this can run from
    /// tests or from any setup path that already knows the extra command
    /// stores.
    ///
    /// # Errors
    ///
    /// If any exec spec is invalid, a composite references an unknown command,
    /// a composite cycles, expansion exceeds [`MAX_COMPOSITE_DEPTH`], or an
    /// alias collides with a command name or with another command's alias.
    pub fn validate_commands(&self, externals: &[&str]) -> anyhow::Result<()> {
        self.validate()?;

        let known: std::collections::HashSet<&str> = self
            .commands
            .keys()
            .map(String::as_str)
            .chain(externals.iter().copied())
            .collect();

        for (name, spec) in &self.commands {
            if let CommandSpec::Composite(_) = spec {
                let mut visiting = std::collections::HashSet::new();
                self.walk_composite(name, &known, &mut visiting, 0)?;
            }
        }

        // ERR-1 / TASK-1181, TASK-1182: alias hygiene. The CLI's `External`
        // dispatcher matches the literal command name first and only falls
        // through to alias lookup when no command exists by that name, so
        // an alias that collides with an existing command name is silently
        // dead. Symmetrically, `resolve_alias` does an order-dependent
        // linear scan and would invisibly shadow whichever command happens
        // to appear later in the IndexMap when two commands declare the
        // same alias. Catch both up-front so misconfigurations fail loud
        // at validate time rather than as ghost behaviour at invocation.
        let mut alias_owner: std::collections::HashMap<&str, &str> =
            std::collections::HashMap::new();
        for (name, spec) in &self.commands {
            for alias in spec.aliases() {
                let alias_str = alias.as_str();
                if known.contains(alias_str) {
                    anyhow::bail!(
                        "command '{name}': alias '{alias_str}' collides with an existing \
                         command name; the alias would be silently dead because the literal \
                         name takes precedence at dispatch"
                    );
                }
                if let Some(prior) = alias_owner.insert(alias_str, name.as_str()) {
                    anyhow::bail!(
                        "alias '{alias_str}' is declared by both commands '{prior}' and \
                         '{name}'; alias resolution would silently pick whichever command \
                         appears first in config order"
                    );
                }
            }
        }
        Ok(())
    }

    /// Recursive composite walker.
    ///
    /// ERR-1 / TASK-1221: `visiting` must be left in a consistent state on
    /// every exit path, including the early-`Err` short-circuits inside the
    /// loop. The previous shape used `?` then a tail `visiting.remove(name)`
    /// that only ran on success, so a future refactor hoisting `visiting` to
    /// an outer scope (an obvious optimisation across sibling composite roots)
    /// would silently produce false-positive cycle errors on re-validation.
    /// The invariant is now: if this function inserted `name` into `visiting`,
    /// it removes it before returning, regardless of outcome.
    pub(crate) fn walk_composite<'a>(
        &'a self,
        name: &'a str,
        known: &std::collections::HashSet<&'a str>,
        visiting: &mut std::collections::HashSet<&'a str>,
        depth: usize,
    ) -> anyhow::Result<()> {
        if depth > MAX_COMPOSITE_DEPTH {
            anyhow::bail!(
                "command '{name}': composite expansion exceeded depth limit {MAX_COMPOSITE_DEPTH}"
            );
        }
        if !visiting.insert(name) {
            // `name` was already in the set: this is the cycle signal, and
            // the prior insertion belongs to an ancestor frame which is
            // responsible for removing it on its own way out.
            anyhow::bail!("command '{name}': cycle detected in composite command");
        }
        // From here we own the `visiting` entry for `name`. Drive the body
        // through a single result binding so the post-loop `remove` runs on
        // every path — including unknown-ref bail and recursive Err.
        let mut result: anyhow::Result<()> = Ok(());
        if let Some(CommandSpec::Composite(c)) = self.commands.get(name) {
            for sub in &c.commands {
                let sub_str = sub.as_str();
                if !known.contains(sub_str) {
                    result = Err(anyhow::anyhow!(
                        "command '{name}': references unknown command '{sub_str}'"
                    ));
                    break;
                }
                // Only recurse into config-defined composites; externals are
                // opaque from this side and may be exec or composite — their
                // internal cycles, if any, would be caught by their own
                // validate path, not this one.
                if let Some(CommandSpec::Composite(_)) = self.commands.get(sub_str) {
                    if let Err(e) = self.walk_composite(sub_str, known, visiting, depth + 1) {
                        result = Err(e);
                        break;
                    }
                }
            }
        }
        visiting.remove(name);
        result
    }

    /// Find the canonical command name for an alias.
    /// Returns `Some(command_name)` if the alias matches a command's aliases list.
    ///
    /// O(N·M) over commands × aliases. The alias lookup is called once per
    /// CLI invocation so an inline scan is still cheap in practice — each
    /// user has tens of commands and a handful of aliases.
    #[must_use]
    pub fn resolve_alias(&self, alias: &str) -> Option<&str> {
        for (name, spec) in &self.commands {
            if spec.aliases().iter().any(|a| a == alias) {
                return Some(name.as_str());
            }
        }
        None
    }
}

/// TRAIT-4 / TASK-0872: `Default` is intentionally test-only. Production
/// code uses [`Config::empty`] (explicit blank slate) or
/// [`load_config_or_default`] (user-visible defaults). The serde defaults
/// on individual fields do not require `Config: Default`.
#[cfg(any(test, feature = "test-support"))]
impl Default for Config {
    fn default() -> Self {
        Self::empty()
    }
}
