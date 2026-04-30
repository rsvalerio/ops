//! Command resolution: lookups across config / stack / extension stores,
//! alias resolution, and composite expansion.
//!
//! Split out of `command/mod.rs` (ARCH-1 / TASK-0303) so the orchestrator
//! file is purely about *running* plans, not naming them.

use super::{CommandRunner, ExpandError, ResolveExecError};
use indexmap::IndexMap;
use ops_core::config::{CommandId, CommandSpec, ExecCommandSpec};

impl CommandRunner {
    /// Iterator over all command keys across config → stack → extension.
    pub(super) fn all_command_keys(&self) -> impl Iterator<Item = &str> {
        self.config
            .commands
            .keys()
            .map(|s| s.as_str())
            .chain(self.stack_commands.keys().map(|k| k.as_str()))
            .chain(self.extension_commands.keys().map(|k| k.as_str()))
    }

    /// Look up a command by ID across all stores (config → stack → extension).
    fn find_in_stores(&self, id: &str) -> Option<&CommandSpec> {
        self.config
            .commands
            .get(id)
            .or_else(|| self.stack_commands.get(id))
            .or_else(|| self.extension_commands.get(id))
    }

    /// Check if a command ID exists in any store.
    fn exists_in_stores(&self, id: &str) -> bool {
        self.config.commands.contains_key(id)
            || self.stack_commands.contains_key(id)
            || self.extension_commands.contains_key(id)
    }

    /// Resolve a command by ID or alias (config first, then stack defaults, then extension, then aliases).
    pub fn resolve(&self, id: &str) -> Option<&CommandSpec> {
        self.find_in_stores(id).or_else(|| self.resolve_alias(id))
    }

    /// Return the canonical command name for a given ID or alias.
    /// If the ID is already a direct command name, returns it as-is.
    /// If it matches an alias, returns the canonical name.
    ///
    /// OWN-6 / TASK-0200: alias search is O(1) via `non_config_alias_map`
    /// instead of scanning every spec's aliases list.
    pub(super) fn canonical_id<'a>(&'a self, id: &'a str) -> &'a str {
        if self.exists_in_stores(id) {
            return id;
        }
        if let Some(name) = self.config.resolve_alias(id) {
            return name;
        }
        if let Some(name) = self.non_config_alias_map.get(id) {
            return name.as_str();
        }
        id
    }

    /// Look up a command by alias across all command sources.
    fn resolve_alias(&self, alias: &str) -> Option<&CommandSpec> {
        // Config aliases use a dedicated method (separate alias map)
        if let Some(name) = self.config.resolve_alias(alias) {
            return self.config.commands.get(name);
        }
        let canonical = self.non_config_alias_map.get(alias)?;
        self.stack_commands
            .get(canonical.as_str())
            .or_else(|| self.extension_commands.get(canonical.as_str()))
    }

    /// List all available command IDs (config first, then stack, then extension commands; sorted for stable order).
    pub fn list_command_ids(&self) -> Vec<CommandId> {
        let mut ids: Vec<&str> = self.all_command_keys().collect();
        ids.sort_unstable();
        ids.dedup();
        ids.iter().map(|s| CommandId::from(*s)).collect()
    }

    /// Expand to a flat list of exec-only command IDs (no composites), so run_plan need not recurse.
    ///
    /// Returns [`ExpandError`] distinguishing the three distinct failure modes
    /// — unknown id, cycle, depth exceeded — so callers can render accurate
    /// diagnostics instead of blanket "unknown command". (ERR-10 / READ-5.)
    ///
    /// # Recursion Depth
    ///
    /// The recursion is bounded by the cycle detection mechanism - each composite can only
    /// be visited once per expansion. For deeply nested composites, the call stack depth is
    /// limited by the number of unique composites, not the total depth. In practice, this
    /// means a graph with N composites has at most N stack frames during expansion.
    ///
    /// An additional guard limits expansion to 100 levels to prevent pathological cases.
    pub fn expand_to_leaves(&self, id: &str) -> Result<Vec<CommandId>, ExpandError> {
        /// CQ-012: Maximum recursion depth for composite expansion.
        ///
        /// This limit prevents stack overflow from pathological configs with deeply
        /// nested composites (e.g., a -> b -> c -> ... -> z with 100+ levels). Normal
        /// configs typically have 2-5 levels (e.g., verify -> [build, test] -> cargo).
        /// The cycle detection already catches circular references, so this is a
        /// defense against accidental deep nesting.
        const MAX_DEPTH: usize = 100;
        let mut visited = std::collections::HashSet::new();
        self.expand_inner(id, &mut visited, 0, MAX_DEPTH)
    }

    fn expand_inner(
        &self,
        id: &str,
        visited: &mut std::collections::HashSet<String>,
        depth: usize,
        max_depth: usize,
    ) -> Result<Vec<CommandId>, ExpandError> {
        if depth > max_depth {
            tracing::warn!(
                id = %id,
                depth = depth,
                max_depth = max_depth,
                "composite expansion depth limit exceeded"
            );
            return Err(ExpandError::DepthExceeded {
                id: id.to_string(),
                max_depth,
            });
        }
        let canonical = self.canonical_id(id);
        let spec = self
            .resolve(canonical)
            .ok_or_else(|| ExpandError::Unknown(id.to_string()))?;
        match spec {
            CommandSpec::Exec(_) => Ok(vec![CommandId::from(canonical)]),
            CommandSpec::Composite(c) => {
                // PATTERN-1 / TASK-0505: track only the active recursion
                // stack so a diamond DAG (A -> [B, C]; B, C -> [D]) does not
                // raise a false-positive cycle on the second visit to D.
                // True cycles (self-reference, A -> B -> A) still re-enter
                // a node already on the stack and trigger the check.
                if !visited.insert(canonical.to_string()) {
                    return Err(ExpandError::Cycle(canonical.to_string()));
                }
                let mut out = Vec::new();
                for sub in &c.commands {
                    out.extend(self.expand_inner(sub, visited, depth + 1, max_depth)?);
                }
                visited.remove(canonical);
                Ok(out)
            }
        }
    }

    /// Resolve a leaf ID to an owned [`ExecCommandSpec`], producing a typed
    /// [`ResolveExecError`] that sequential (`execute_step`) and raw
    /// (`run_plan_raw`) paths both surface identically. (ERR-10 / TASK-0130.)
    pub(super) fn resolve_exec_leaf(&self, id: &str) -> Result<ExecCommandSpec, ResolveExecError> {
        match self.resolve(id) {
            Some(CommandSpec::Exec(e)) => Ok(e.clone()),
            Some(CommandSpec::Composite(_)) => {
                Err(ResolveExecError::CompositeInLeafPlan(id.to_string()))
            }
            None => Err(ResolveExecError::Unknown(id.to_string())),
        }
    }

    /// Resolve command IDs to exec specs, returning Err with the offending ID on failure.
    pub(super) fn resolve_exec_specs(
        &self,
        command_ids: &[CommandId],
    ) -> Result<Vec<(CommandId, ExecCommandSpec)>, CommandId> {
        let mut steps = Vec::with_capacity(command_ids.len());
        for id in command_ids {
            match self.resolve(id) {
                // Clone is required: specs must be owned to move into spawned tasks.
                // Acceptable for typical parallel groups (<10 commands).
                Some(CommandSpec::Exec(e)) => steps.push((id.clone(), e.clone())),
                _ => return Err(id.clone()),
            }
        }
        Ok(steps)
    }
}

/// Build an `alias → canonical_name` map by flattening one or more command
/// stores in iteration order. Later stores override earlier ones (matching
/// the existing stack → extension precedence). Collisions across stores are
/// logged at `tracing::warn!` with both canonical owners, consistent with
/// `CommandRegistry` and `DataRegistry` duplicate-detection policy.
pub(super) fn build_alias_map<'a, I>(stores: I) -> std::collections::HashMap<String, String>
where
    I: IntoIterator<Item = &'a IndexMap<CommandId, CommandSpec>>,
{
    let mut map = std::collections::HashMap::new();
    for store in stores {
        for (name, spec) in store {
            for alias in spec.aliases() {
                if let Some(existing) = map.get(alias.as_str()) {
                    tracing::warn!(
                        alias = %alias,
                        existing = %existing,
                        new = %name,
                        "alias collision: later store overrides earlier"
                    );
                }
                map.insert(alias.clone(), name.to_string());
            }
        }
    }
    map
}
