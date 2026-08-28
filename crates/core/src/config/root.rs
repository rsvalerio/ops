//! The root [`Config`] type: its fields, construction, and validation.
//!
//! Command-spec validation lives here rather than in [`super::commands`]
//! because cycle and unknown-reference checks need the whole command map.

use super::commands::CommandSpec;
use super::sections::{AboutConfig, DataConfig, ExtensionConfig, OutputConfig};
use super::theme_types::ThemeConfig;
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
}

/// Mutable state threaded through [`Config::walk_composite`].
///
/// The two sets carry **opposite** meanings and neither may be conflated with
/// the other:
///
/// - `visiting` is the current DFS *path*: a node is inserted on entry and
///   removed on every exit path, so re-encountering one is the cycle signal
///   (ERR-1 / TASK-1221). It is cleared between sibling roots.
/// - `validated` maps each node whose subtree has fully validated to that
///   subtree's *height*, and is never cleared — it is the memo that keeps the
///   walk linear (SEC-33 / TASK-1832).
#[derive(Debug, Default)]
pub struct CompositeWalk<'a> {
    visiting: std::collections::HashSet<&'a str>,
    validated: std::collections::HashMap<&'a str, usize>,
}

#[cfg(test)]
impl CompositeWalk<'_> {
    /// ERR-1 / TASK-1221: lets the invariant tests assert that the DFS path
    /// set is empty on every exit, without making the field public.
    pub fn path_is_empty(&self) -> bool {
        self.visiting.is_empty()
    }
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
        }
    }

    /// Validate all command specs. Called after loading to fail fast on invalid config.
    ///
    /// This is the validation the shipped binary actually runs — the loader's
    /// `load_config_at` calls it on every `ops` invocation. It covers:
    ///
    /// - every exec spec ([`ExecCommandSpec::validate`]);
    /// - every theme ([`ThemeConfig::validate`], SEC-33 / TASK-1849);
    /// - alias hygiene against the config's own command names
    ///   ([`Config::validate_aliases`], SEC-31 / TASK-1818).
    ///
    /// **Composite reference, cycle, and depth checks are not run here.** A
    /// composite may reference a stack default or an extension-registered
    /// command that is not known at config load time, so those checks need an
    /// `externals` list the loader does not have; they live in
    /// [`Config::validate_commands`] and are re-caught at dispatch by the
    /// runner's `expand_inner`. Alias hygiene, by contrast, is *not*
    /// duplicated anywhere downstream, which is why it runs here (SEC-31 /
    /// TASK-1818): the externals it cannot see only make the check narrower,
    /// never wrong.
    ///
    /// # Errors
    ///
    /// If any exec spec fails [`ExecCommandSpec::validate`], any theme fails
    /// [`ThemeConfig::validate`], or alias hygiene fails. Composite reference /
    /// cycle / depth checks are not performed — see
    /// [`Config::validate_commands`].
    pub fn validate(&self) -> anyhow::Result<()> {
        for (name, spec) in &self.commands {
            if let CommandSpec::Exec(exec) = spec {
                exec.validate(name)?;
            }
        }
        // SEC-33 / TASK-1849: `[themes]` was the one config section nothing
        // screened, so an unbounded `left_pad` reached `" ".repeat(n)` and
        // aborted the process from a ~400-byte `.ops.toml`.
        for (name, theme) in &self.themes {
            theme.validate(name)?;
        }
        let own_names: std::collections::HashSet<&str> =
            self.commands.keys().map(String::as_str).collect();
        self.validate_aliases(&own_names)
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

        // SEC-33 / TASK-1832: `state.validated` is the third DFS colour and is
        // deliberately carried across the sibling roots — that is what makes
        // the whole pass O(V+E). `state.visiting` is reset per root; see
        // [`CompositeWalk`] and [`Config::walk_composite`].
        let mut state = CompositeWalk::default();
        for (name, spec) in &self.commands {
            if let CommandSpec::Composite(_) = spec {
                state.visiting.clear();
                self.walk_composite(name, &known, &mut state, 0)?;
            }
        }

        self.validate_aliases(&known)
    }

    /// ERR-1 / TASK-1181, TASK-1182: alias hygiene.
    ///
    /// The CLI's `External` dispatcher matches the literal command name first
    /// and only falls through to alias lookup when no command exists by that
    /// name, so an alias that collides with an existing command name is
    /// silently dead. Symmetrically, [`Config::resolve_alias`] does an
    /// order-dependent linear scan and would invisibly shadow whichever
    /// command happens to appear later in the `IndexMap` when two commands
    /// declare the same alias. Catch both up-front so misconfigurations fail
    /// loud at validate time rather than as ghost behaviour at invocation.
    ///
    /// SEC-31 / TASK-1818: this used to live inside
    /// [`Config::validate_commands`], which has no production caller — so the
    /// shipped binary ran neither rule and dispatched to whichever command sat
    /// earlier in the map. It is now reachable from [`Config::validate`], the
    /// one validation `load_config_at` performs, with `known` narrowed to the
    /// config's own command names. `validate_commands` still passes the wider
    /// set including `externals`; a narrower `known` only makes the
    /// collides-with-a-command-name rule miss external names, never
    /// false-positive, and the duplicate-alias rule does not depend on it at
    /// all.
    ///
    /// # Errors
    ///
    /// If an alias collides with a name in `known`, or two commands declare
    /// the same alias.
    fn validate_aliases(&self, known: &std::collections::HashSet<&str>) -> anyhow::Result<()> {
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
    ///
    /// SEC-33 / TASK-1832: `validated` is the **second, independent** set and
    /// carries the opposite meaning — `visiting` is the current DFS *path*
    /// (inserted on entry, removed on every exit; that is the cycle signal),
    /// while `validated` is the set of nodes whose entire subtree already
    /// returned `Ok` and is never cleared. Without it the walker re-descended
    /// into every shared subtree once per incoming edge, so a 31-command
    /// diamond chain (`c0 = ["c1","c1"]`, `c1 = ["c2","c2"]`, …) cost 2^30
    /// visits and hung `ops <anything>` — an unauthenticated local `DoS` from
    /// repo-supplied `.ops.toml`, well inside every existing size cap. Skipping
    /// a `validated` node is sound: a node that completed with no cycle cannot
    /// acquire one by being reached from a second parent, because a cycle
    /// through it would have been a cycle on the first descent too. Dropping
    /// `validated` in a refactor restores the exponential blowup silently — the
    /// diamond tests still pass, only the timing changes.
    pub(crate) fn walk_composite<'a>(
        &'a self,
        name: &'a str,
        known: &std::collections::HashSet<&'a str>,
        state: &mut CompositeWalk<'a>,
        depth: usize,
    ) -> anyhow::Result<usize> {
        // Memo hit. The stored value is the subtree's *height* (edges below
        // this node), not a bare "seen" flag: the depth limit is measured from
        // whichever root we entered by, so a node validated at depth 10 can
        // still blow the limit when reached again at depth 95. Re-checking
        // `depth + height` keeps the memoised path bit-identical to the
        // re-walked one instead of quietly failing open.
        if let Some(&height) = state.validated.get(name) {
            if depth.saturating_add(height) > MAX_COMPOSITE_DEPTH {
                anyhow::bail!(
                    "command '{name}': composite expansion exceeded depth limit {MAX_COMPOSITE_DEPTH}"
                );
            }
            return Ok(height);
        }
        if depth > MAX_COMPOSITE_DEPTH {
            anyhow::bail!(
                "command '{name}': composite expansion exceeded depth limit {MAX_COMPOSITE_DEPTH}"
            );
        }
        if !state.visiting.insert(name) {
            // `name` was already in the set: this is the cycle signal, and
            // the prior insertion belongs to an ancestor frame which is
            // responsible for removing it on its own way out.
            anyhow::bail!("command '{name}': cycle detected in composite command");
        }
        // From here we own the `visiting` entry for `name`. Drive the body
        // through a single result binding so the post-loop `remove` runs on
        // every path — including unknown-ref bail and recursive Err.
        let mut result: anyhow::Result<usize> = Ok(0);
        let mut height = 0usize;
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
                    let next = depth.saturating_add(1);
                    match self.walk_composite(sub_str, known, state, next) {
                        Ok(child_height) => {
                            height = height.max(child_height.saturating_add(1));
                        }
                        Err(e) => {
                            result = Err(e);
                            break;
                        }
                    }
                }
            }
        }
        state.visiting.remove(name);
        if result.is_ok() {
            // Only a subtree that fully validated may be memoised; a node whose
            // walk bailed must be re-walked so the same error surfaces from
            // every root that reaches it.
            state.validated.insert(name, height);
            result = Ok(height);
        }
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
