//! Command resolution: lookups across config / stack / extension stores,
//! alias resolution, and composite expansion into plan trees.
//!
//! Kept apart from `command/mod.rs` so the orchestrator file is purely
//! about *running* plans, not naming them.

use super::{CommandRunner, ExpandError, ResolveExecError, UnknownCommand};
use indexmap::IndexMap;
use ops_core::config::{CommandId, CommandSpec, ExecCommandSpec};

/// One command's execution plan after composite expansion (TASK-2275).
///
/// Expansion used to flatten the whole composite tree into one leaf list
/// scheduled by its root, so a sequential root silently downgraded a nested
/// parallel group ("run these groups in order, but let the steps inside one
/// group run together" was impossible). The tree keeps each group's own
/// scheduling:
///
/// - [`CommandPlan::Stage`] is one schedulable flat plan — everything under a
///   `parallel = true` group (or a lone exec leaf). The stage's `parallel`
///   and `fail_fast` come from that group's own subtree.
/// - [`CommandPlan::Sequence`] is a `parallel = false` group: each entry runs
///   as its own plan, one after another, under that entry's own schedule —
///   exactly what typing the entries on the command line does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandPlan {
    /// A flat, schedulable plan: `leaf_ids` run sequentially, or in
    /// exclusive-split stages when `parallel` (see `parallel.rs`).
    /// `fail_fast` is the value every composite in the stage's subtree
    /// agreed on (`true` for an exec-only stage).
    Stage {
        leaf_ids: Vec<CommandId>,
        parallel: bool,
        fail_fast: bool,
    },
    /// A sequential group's entries, in declaration order. `fail_fast` is the
    /// group's *own* declaration: when false, every entry runs regardless of
    /// failures; when true, a failing entry whose
    /// [`effective_fail_fast`](CommandPlan::effective_fail_fast) is true
    /// stops the entries after it.
    Sequence {
        children: Vec<Self>,
        fail_fast: bool,
    },
}

impl CommandPlan {
    /// The group's own `fail_fast` declaration.
    #[must_use]
    pub const fn fail_fast(&self) -> bool {
        match self {
            Self::Stage { fail_fast, .. } | Self::Sequence { fail_fast, .. } => *fail_fast,
        }
    }

    /// `fail_fast` as a parent sees it: the node's own declaration AND every
    /// descendant's — a single `fail_fast = false` anywhere in the tree makes
    /// the whole tree non-fail-fast. This is the same whole-tree aggregation
    /// the pre-TASK-2275 flat walk computed, so a name's plan reports what
    /// the config's uniform value always said.
    #[must_use]
    pub fn effective_fail_fast(&self) -> bool {
        match self {
            Self::Stage { .. } => self.fail_fast(),
            Self::Sequence { children, .. } => {
                self.fail_fast() && children.iter().all(Self::effective_fail_fast)
            }
        }
    }

    /// Every exec leaf in execution order (left-to-right over the tree).
    #[must_use]
    pub fn leaf_ids(&self) -> Vec<CommandId> {
        match self {
            Self::Stage { leaf_ids, .. } => leaf_ids.clone(),
            Self::Sequence { children, .. } => children.iter().flat_map(Self::leaf_ids).collect(),
        }
    }

    /// Whether any stage in the tree declares `parallel = true`.
    #[must_use]
    pub fn any_parallel(&self) -> bool {
        match self {
            Self::Stage { parallel, .. } => *parallel,
            Self::Sequence { children, .. } => children.iter().any(Self::any_parallel),
        }
    }

    /// Whether the tree needs a multi-thread runtime: a parallel stage with
    /// more than one leaf actually fans out (a 1-leaf parallel stage takes
    /// `run_plan`'s sequential shortcut, so worker threads would be pure
    /// start-up cost — mirrors the threshold the pre-tree executor used).
    #[must_use]
    pub fn needs_worker_threads(&self) -> bool {
        match self {
            Self::Stage {
                leaf_ids, parallel, ..
            } => *parallel && leaf_ids.len() > 1,
            Self::Sequence { children, .. } => children.iter().any(Self::needs_worker_threads),
        }
    }
}

/// Walk state for `expand_node` / `expand_flat_node`. Shared across the
/// whole expansion so cycle detection and the depth budget span sequential
/// and flat recursion alike.
struct PlanCtx<'a> {
    visited: std::collections::HashSet<&'a str>,
    depth: usize,
    max_depth: usize,
}

/// Flag-aggregation state for one flat walk (the subtree of a single
/// `parallel = true` group). Every composite in that subtree must agree on
/// both flags; a disagreement is rejected naming the parallel root.
struct FlatFlags<'a> {
    /// `(name, value)` of the flat plan's root composite's `parallel`.
    parallel_decl: Option<(&'a str, bool)>,
    /// Same, for `fail_fast`.
    fail_fast_decl: Option<(&'a str, bool)>,
    fail_fast_disabled: bool,
}

/// Enforce that every composite in one flat stage agrees on a scheduling
/// flag, recording the first declaration and rejecting any later
/// disagreement.
///
/// Comparing against the *first* composite visited is sufficient to prove
/// whole-stage agreement: expansion is a depth-first walk from the parallel
/// root, so the first declaration is the root's, and if every later node
/// matches the root then all nodes match each other. It also makes the error
/// name the root the user actually invoked rather than an arbitrary interior
/// pair.
fn check_schedule_flag<'a>(
    decl: &mut Option<(&'a str, bool)>,
    flag: &'static str,
    name: &'a str,
    value: bool,
) -> Result<(), ExpandError> {
    match *decl {
        None => {
            *decl = Some((name, value));
            Ok(())
        }
        Some((root, root_value)) if root_value != value => {
            tracing::warn!(
                flag = %flag,
                root = ?root,
                root_value,
                conflicting = ?name,
                conflicting_value = value,
                "rejecting composite plan with conflicting scheduling flags"
            );
            Err(ExpandError::ConflictingSchedule {
                flag,
                root: root.to_string(),
                root_value,
                conflicting: name.to_string(),
                conflicting_value: value,
            })
        }
        Some(_) => Ok(()),
    }
}

/// Shared depth-budget check for both recursion shapes, so the sequential
/// and flat walks keep one guard (and one warning) instead of two drifting
/// copies. Returns the [`ExpandError::DepthExceeded`] to propagate, if any.
fn depth_guard(id: &str, ctx: &PlanCtx<'_>) -> Option<ExpandError> {
    if ctx.depth > ctx.max_depth {
        tracing::warn!(
            id = ?id,
            depth = ctx.depth,
            max_depth = ctx.max_depth,
            "composite expansion depth limit exceeded"
        );
        return Some(ExpandError::DepthExceeded {
            id: id.to_string(),
            max_depth: ctx.max_depth,
        });
    }
    None
}

// Counts walks over the command stores.
//
// `canonical_with_spec` resolves a command and its canonical name in a
// single pass, so each visited node costs one store traversal rather than
// two. That contract is pinned by counting the traversals rather than
// timing 1k expansions against a wall-clock budget: timing is
// load-dependent in a debug build and too coarse to catch a 2x
// regression, while counting pins it exactly and deterministically.
// **Thread-local**, not a global counter. Test binaries run tests in parallel
// threads and many of them resolve commands, so a process-wide counter is
// incremented by unrelated tests between a reader's two observations. Each
// test observes only the walks made on its own thread, which makes an exact
// equality assertion sound.
#[cfg(test)]
thread_local! {
    static STORE_WALKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn record_store_walk() {
    // Test-only traversal counter: bounded by the walks one test thread
    // performs, so `saturating_add` is exactly equal to `+ 1` here.
    STORE_WALKS.with(|c| c.set(c.get().saturating_add(1)));
}

#[cfg(test)]
pub(super) fn store_walk_count() -> usize {
    STORE_WALKS.with(std::cell::Cell::get)
}

impl CommandRunner {
    /// Iterator over all command keys across config → stack → extension.
    pub(super) fn all_command_keys(&self) -> impl Iterator<Item = &str> {
        self.config
            .commands
            .keys()
            .map(std::string::String::as_str)
            .chain(
                self.stack_commands
                    .keys()
                    .map(ops_core::config::CommandId::as_str),
            )
            .chain(
                self.extension_commands
                    .keys()
                    .map(ops_core::config::CommandId::as_str),
            )
            .chain(
                self.builtin_commands
                    .keys()
                    .map(ops_core::config::CommandId::as_str),
            )
    }

    /// Look up a command by ID across all stores (config → stack → extension → builtin).
    /// Builtins land last so user config / stack defaults / extensions can shadow them.
    fn find_in_stores(&self, id: &str) -> Option<&CommandSpec> {
        #[cfg(test)]
        record_store_walk();
        self.config
            .commands
            .get(id)
            .or_else(|| self.stack_commands.get(id))
            .or_else(|| self.extension_commands.get(id))
            .or_else(|| self.builtin_commands.get(id))
    }

    /// Resolve a command by ID or alias (config first, then stack defaults, then extension, then aliases).
    #[must_use]
    pub fn resolve(&self, id: &str) -> Option<&CommandSpec> {
        self.find_in_stores(id).or_else(|| self.resolve_alias(id))
    }

    /// Resolve a command id (or alias) to its `(canonical_name, spec)` pair
    /// in a single pass over the command stores and alias maps, where
    /// [`Self::resolve`] and [`Self::resolve_alias`] each walk independently.
    ///
    /// Composite expansion calls this once per node rather than doing a
    /// canonical-name-only lookup followed by `resolve(canonical)`, which
    /// would traverse the config → stack → extension → alias chain twice
    /// per node. For a recursion-heavy composite graph the duplication
    /// scales linearly with graph size; this helper folds the work into
    /// one walk.
    pub(super) fn canonical_with_spec<'a>(
        &'a self,
        id: &str,
    ) -> Option<(&'a str, &'a CommandSpec)> {
        #[cfg(test)]
        record_store_walk();
        if let Some((k, v)) = self.config.commands.get_key_value(id) {
            return Some((k.as_str(), v));
        }
        if let Some((k, v)) = self.stack_commands.get_key_value(id) {
            return Some((k.as_str(), v));
        }
        if let Some((k, v)) = self.extension_commands.get_key_value(id) {
            return Some((k.as_str(), v));
        }
        if let Some((k, v)) = self.builtin_commands.get_key_value(id) {
            return Some((k.as_str(), v));
        }
        if let Some(name) = self.config.resolve_alias(id) {
            if let Some((k, v)) = self.config.commands.get_key_value(name) {
                return Some((k.as_str(), v));
            }
            // Orphan config alias (alias map survived a
            // config edit that removed the underlying entry). Fall through
            // to stack / extension lookups below — both by the canonical
            // name the orphan alias points to and by the original id, so a
            // stack default sharing either name still resolves.
            if let Some((k, v)) = self.stack_commands.get_key_value(name) {
                return Some((k.as_str(), v));
            }
            if let Some((k, v)) = self.extension_commands.get_key_value(name) {
                return Some((k.as_str(), v));
            }
            if let Some((k, v)) = self.builtin_commands.get_key_value(name) {
                return Some((k.as_str(), v));
            }
        }
        if let Some(name) = self.non_config_alias_map.get(id) {
            let n = name.as_str();
            if let Some((k, v)) = self.stack_commands.get_key_value(n) {
                return Some((k.as_str(), v));
            }
            if let Some((k, v)) = self.extension_commands.get_key_value(n) {
                return Some((k.as_str(), v));
            }
            if let Some((k, v)) = self.builtin_commands.get_key_value(n) {
                return Some((k.as_str(), v));
            }
        }
        None
    }

    /// Look up a command by alias across all command sources.
    fn resolve_alias(&self, alias: &str) -> Option<&CommandSpec> {
        // Config aliases use a dedicated method (separate alias map)
        if let Some(name) = self.config.resolve_alias(alias) {
            if let Some(spec) = self.config.commands.get(name) {
                return Some(spec);
            }
            // Orphan config alias — config alias map
            // points at a name that has no command in `config.commands`
            // (possible when a config edit removes the canonical entry but
            // leaves a stale alias entry, or when alias storage drifts from
            // command storage). Fall through to the stack/extension stores
            // by the canonical name *and* by the original alias so a stack
            // default of the same name still resolves instead of
            // short-circuiting to `None`.
            if let Some(spec) = self
                .stack_commands
                .get(name)
                .or_else(|| self.extension_commands.get(name))
                .or_else(|| self.builtin_commands.get(name))
            {
                return Some(spec);
            }
        }
        let canonical = self.non_config_alias_map.get(alias)?;
        self.stack_commands
            .get(canonical.as_str())
            .or_else(|| self.extension_commands.get(canonical.as_str()))
            .or_else(|| self.builtin_commands.get(canonical.as_str()))
    }

    /// List all available command IDs (config first, then stack, then extension commands; sorted for stable order).
    ///
    /// Collects into a `BTreeSet<&str>` so sort+dedup happens during
    /// insertion, then maps straight into `CommandId` — one pass, with no
    /// intermediate `Vec<&str>` / `Vec<CommandId>` pair and no separate
    /// `sort_unstable`/`dedup` pass. Tab-completion latency on `--list` and
    /// the help/discovery paths benefits from the single-pass form.
    pub fn list_command_ids(&self) -> Vec<CommandId> {
        let ids: std::collections::BTreeSet<&str> = self.all_command_keys().collect();
        ids.into_iter().map(CommandId::from).collect()
    }

    /// Expand to a flat list of exec-only command IDs (no composites), in
    /// execution order, so `run_plan` need not recurse.
    ///
    /// Returns [`ExpandError`] distinguishing the three distinct failure modes
    /// — unknown id, cycle, depth exceeded — so callers can render accurate
    /// diagnostics instead of blanket "unknown command".
    ///
    /// # Recursion Depth
    ///
    /// The recursion is bounded by the cycle detection mechanism - each composite can only
    /// be visited once per expansion. For deeply nested composites, the call stack depth is
    /// limited by the number of unique composites, not the total depth. In practice, this
    /// means a graph with N composites has at most N stack frames during expansion.
    ///
    /// An additional guard limits expansion to 100 levels to prevent pathological cases.
    ///
    /// # Errors
    ///
    /// [`ExpandError`] if `id` is unknown, the composite tree cycles, or
    /// expansion exceeds the depth limit.
    pub fn expand_to_leaves(&self, id: &str) -> Result<Vec<CommandId>, ExpandError> {
        Ok(self.expand_to_plan(id)?.leaf_ids())
    }

    /// Expand a named command into a [`CommandPlan`] tree (TASK-2275).
    ///
    /// One walk of the composite tree produces both the structure and every
    /// stage's scheduling flags, so the executed steps and their schedule
    /// are derived from the same traversal (no risk of independent walks
    /// drifting in cycle/ordering semantics).
    ///
    /// # Errors
    ///
    /// [`ExpandError`] if `id` is unknown, the composite tree cycles, expansion
    /// exceeds the depth limit, a parallel group contains a sequential one,
    /// or a parallel group's subtree declares conflicting `fail_fast` values.
    pub fn expand_to_plan(&self, id: &str) -> Result<CommandPlan, ExpandError> {
        /// Maximum recursion depth for composite expansion.
        ///
        /// This limit prevents stack overflow from pathological configs with deeply
        /// nested composites (e.g., a -> b -> c -> ... -> z with 100+ levels). Normal
        /// configs typically have 2-5 levels (e.g., verify -> [build, test] -> cargo).
        /// The cycle detection already catches circular references, so this is a
        /// defense against accidental deep nesting.
        const MAX_DEPTH: usize = 100;
        let mut ctx = PlanCtx {
            visited: std::collections::HashSet::new(),
            depth: 0,
            max_depth: MAX_DEPTH,
        };
        self.expand_node(id, &mut ctx)
    }

    /// Expand one node of the plan tree: an exec leaf becomes a sequential
    /// one-leaf stage, a `parallel = true` group becomes one flat stage over
    /// its whole subtree, and a `parallel = false` group becomes a sequence
    /// that expands each entry as its own plan.
    fn expand_node<'a>(
        &'a self,
        id: &str,
        ctx: &mut PlanCtx<'a>,
    ) -> Result<CommandPlan, ExpandError> {
        if let Some(err) = depth_guard(id, ctx) {
            return Err(err);
        }
        // One traversal over the config / stack / extension / alias chain
        // resolves both the canonical name and the spec.
        let (canonical, spec) = self
            .canonical_with_spec(id)
            .ok_or_else(|| ExpandError::Unknown(UnknownCommand::new(id)))?;
        match spec {
            CommandSpec::Exec(_) => Ok(CommandPlan::Stage {
                leaf_ids: vec![CommandId::from(canonical)],
                parallel: false,
                fail_fast: true,
            }),
            CommandSpec::Composite(c) => {
                // Track only the active recursion
                // stack so a diamond DAG (A -> [B, C]; B, C -> [D]) does not
                // raise a false-positive cycle on the second visit to D.
                // True cycles (self-reference, A -> B -> A) still re-enter
                // a node already on the stack and trigger the check.
                //
                // `visited` stores `&'a str` borrowed from
                // the runner's command stores, so canonical names are not
                // cloned per recursion.
                if !ctx.visited.insert(canonical) {
                    return Err(ExpandError::Cycle(canonical.to_string()));
                }
                // `expand_node` returns early above unless `ctx.depth <=
                // ctx.max_depth` (`MAX_DEPTH`), so the increment stays far
                // below `usize::MAX`, and the matching decrement only runs
                // after it, on a depth `>= 1`. Both saturating forms are
                // therefore exactly equal to `+= 1` / `-= 1` here.
                ctx.depth = ctx.depth.saturating_add(1);
                let plan = if c.parallel {
                    // The whole subtree is one flat plan scheduled by this
                    // group. Seed the agreement checks with the root's own
                    // values so any disagreement names this group as the
                    // plan the conflicting flag belongs to.
                    let mut flat = FlatFlags {
                        parallel_decl: Some((canonical, true)),
                        fail_fast_decl: Some((canonical, c.fail_fast)),
                        fail_fast_disabled: !c.fail_fast,
                    };
                    let leaf_ids = self.expand_flat(&c.commands, ctx, &mut flat)?;
                    CommandPlan::Stage {
                        leaf_ids,
                        parallel: true,
                        fail_fast: !flat.fail_fast_disabled,
                    }
                } else {
                    // TASK-2275: a sequential group runs each entry as its
                    // own plan, under that entry's own schedule — the same
                    // treatment command-line names get (TASK-2262).
                    let mut children = Vec::with_capacity(c.commands.len());
                    for sub in &c.commands {
                        children.push(self.expand_node(sub, ctx)?);
                    }
                    CommandPlan::Sequence {
                        children,
                        fail_fast: c.fail_fast,
                    }
                };
                ctx.depth = ctx.depth.saturating_sub(1);
                ctx.visited.remove(canonical);
                Ok(plan)
            }
            // The load path materializes clones before the runner exists;
            // this arm only fires for a Config built outside the loader.
            CommandSpec::Clone(_) => Err(ExpandError::UnmaterializedClone(canonical.to_string())),
        }
    }

    /// Flatten one `parallel = true` group's entries into a single leaf
    /// list, enforcing the whole-stage flag agreement.
    fn expand_flat<'a>(
        &'a self,
        subs: &[String],
        ctx: &mut PlanCtx<'a>,
        flat: &mut FlatFlags<'a>,
    ) -> Result<Vec<CommandId>, ExpandError> {
        let mut out = Vec::new();
        for sub in subs {
            out.extend(self.expand_flat_node(sub, ctx, flat)?);
        }
        Ok(out)
    }

    fn expand_flat_node<'a>(
        &'a self,
        id: &str,
        ctx: &mut PlanCtx<'a>,
        flat: &mut FlatFlags<'a>,
    ) -> Result<Vec<CommandId>, ExpandError> {
        if let Some(err) = depth_guard(id, ctx) {
            return Err(err);
        }
        let (canonical, spec) = self
            .canonical_with_spec(id)
            .ok_or_else(|| ExpandError::Unknown(UnknownCommand::new(id)))?;
        match spec {
            CommandSpec::Exec(_) => Ok(vec![CommandId::from(canonical)]),
            CommandSpec::Composite(c) => {
                if !ctx.visited.insert(canonical) {
                    return Err(ExpandError::Cycle(canonical.to_string()));
                }
                // The stage is flat and scheduled as one unit by its
                // parallel root. A nested sequential group is rejected:
                // running that group's steps concurrently would break the
                // ordering it declares. `fail_fast` must agree across the
                // stage. Checked before recursing so the error names the
                // shallowest offender.
                check_schedule_flag(&mut flat.parallel_decl, "parallel", canonical, c.parallel)?;
                check_schedule_flag(
                    &mut flat.fail_fast_decl,
                    "fail_fast",
                    canonical,
                    c.fail_fast,
                )?;
                if !c.fail_fast {
                    flat.fail_fast_disabled = true;
                }
                let mut out = Vec::new();
                ctx.depth = ctx.depth.saturating_add(1);
                for sub in &c.commands {
                    out.extend(self.expand_flat_node(sub, ctx, flat)?);
                }
                ctx.depth = ctx.depth.saturating_sub(1);
                ctx.visited.remove(canonical);
                Ok(out)
            }
            CommandSpec::Clone(_) => Err(ExpandError::UnmaterializedClone(canonical.to_string())),
        }
    }

    /// Resolve a leaf ID to an owned [`ExecCommandSpec`], producing a typed
    /// [`ResolveExecError`] that sequential (`execute_step`) and raw
    /// (`run_plan_raw`) paths both surface identically.
    pub(super) fn resolve_exec_leaf(&self, id: &str) -> Result<ExecCommandSpec, ResolveExecError> {
        match self.resolve(id) {
            Some(CommandSpec::Exec(e)) => Ok(e.clone()),
            Some(CommandSpec::Composite(_)) => {
                Err(ResolveExecError::CompositeInLeafPlan(id.to_string()))
            }
            Some(CommandSpec::Clone(_)) => Err(ResolveExecError::CloneInLeafPlan(id.to_string())),
            None => Err(ResolveExecError::Unknown(UnknownCommand::new(id))),
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
                        alias = ?alias,
                        existing = ?existing,
                        new = ?name.as_str(),
                        "alias collision: later store overrides earlier"
                    );
                }
                map.insert(alias.clone(), name.to_string());
            }
        }
    }
    map
}
