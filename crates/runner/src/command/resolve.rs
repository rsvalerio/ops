//! Command resolution: lookups across config / stack / extension stores,
//! alias resolution, and composite expansion.
//!
//! Split out of `command/mod.rs` (ARCH-1 / TASK-0303) so the orchestrator
//! file is purely about *running* plans, not naming them.

use super::{CommandRunner, ExpandError, ResolveExecError, UnknownCommand};
use indexmap::IndexMap;
use ops_core::config::{CommandId, CommandSpec, ExecCommandSpec};

/// PATTERN-1 / TASK-1283: walk state for `expand_inner`. Bundling visited /
/// depth / aggregated flags into one struct also keeps the recursive
/// signature within clippy's `too_many_arguments` budget.
struct ExpandCtx<'a> {
    visited: std::collections::HashSet<&'a str>,
    depth: usize,
    max_depth: usize,
    any_parallel: bool,
    fail_fast_disabled: bool,
    /// TASK-1657: `(name, value)` of the first composite in this plan to
    /// declare `parallel`, used to reject a tree that disagrees with itself.
    parallel_decl: Option<(&'a str, bool)>,
    /// TASK-1657: same, for `fail_fast`.
    fail_fast_decl: Option<(&'a str, bool)>,
}

/// TASK-1657: enforce that every composite in one plan agrees on a scheduling
/// flag, recording the first declaration and rejecting any later disagreement.
///
/// Comparing against the *first* composite visited is sufficient to prove
/// whole-plan agreement: expansion is a depth-first walk from the root, so the
/// first declaration is the root's, and if every later node matches the root
/// then all nodes match each other. It also makes the error name the root the
/// user actually invoked rather than an arbitrary interior pair.
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

// TEST-15 / TASK-1664: counts walks over the command stores.
//
// PERF-3 / TASK-0766 folded `canonical_id` + `resolve` into the single
// `canonical_with_spec` pass, halving store traversals per visited node. That
// contract used to be pinned by timing 1k expansions against a two-second
// wall-clock budget — which is load-dependent in a debug build (measured at
// 9.8s under CPU contention) and, worse, too coarse to actually catch the 2x
// regression it was guarding: a doubling would not reliably breach the
// budget. Counting the traversals pins it exactly and deterministically.
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

    /// Return the canonical command name for a given ID or alias, borrowed
    /// from the runner's stores (lifetime tied to `&self`). Returns `None`
    /// if the id is not known.
    ///
    /// Borrowed return lets `expand_inner` track the active recursion stack
    /// in a `HashSet<&str>` without allocating a new String per visit
    /// (OWN-8 / TASK-0714).
    ///
    /// PERF-3 / TASK-0766: `expand_inner` no longer calls this (it uses
    /// `canonical_with_spec` which folds the canonical lookup with the
    /// spec fetch into one pass), but the signature is preserved as part
    /// of the public-ish helper surface that tests and future callers may
    /// depend on for canonical-name normalization without requiring the
    /// spec.
    #[allow(dead_code)]
    pub(super) fn canonical_id<'a>(&'a self, id: &str) -> Option<&'a str> {
        #[cfg(test)]
        record_store_walk();
        if let Some((k, _)) = self.config.commands.get_key_value(id) {
            return Some(k.as_str());
        }
        if let Some((k, _)) = self.stack_commands.get_key_value(id) {
            return Some(k.as_str());
        }
        if let Some((k, _)) = self.extension_commands.get_key_value(id) {
            return Some(k.as_str());
        }
        if let Some((k, _)) = self.builtin_commands.get_key_value(id) {
            return Some(k.as_str());
        }
        if let Some(name) = self.config.resolve_alias(id) {
            return Some(name);
        }
        if let Some(name) = self.non_config_alias_map.get(id) {
            return Some(name.as_str());
        }
        None
    }

    /// Resolve a command id (or alias) to its `(canonical_name, spec)` pair
    /// in a single pass over the same stores [`canonical_id`] and [`resolve`]
    /// each walk independently.
    ///
    /// PERF-3 / TASK-0766: composite expansion previously called both
    /// `canonical_id(id)` and then `resolve(canonical)`, which traversed the
    /// config → stack → extension → alias chain twice per node. For a
    /// recursion-heavy composite graph the duplication scales linearly with
    /// graph size; this helper folds the work into one walk while keeping
    /// the public `canonical_id` / `resolve` shapes untouched for callers
    /// (and tests) that depend on them individually.
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
            // ERR-1 / TASK-1089: orphan config alias (alias map survived a
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
            // ERR-1 / TASK-1089: orphan config alias — config alias map
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
    /// PERF-3 / TASK-1180: collect into a `BTreeSet<&str>` so sort+dedup
    /// happens during insertion, then map straight into `CommandId`. The
    /// previous shape allocated two `Vec`s (`Vec<&str>` then `Vec<CommandId>`)
    /// and a separate `sort_unstable`/`dedup` pass; tab-completion latency on
    /// `--list` and the help/discovery paths benefits from the single-pass form.
    pub fn list_command_ids(&self) -> Vec<CommandId> {
        let ids: std::collections::BTreeSet<&str> = self.all_command_keys().collect();
        ids.into_iter().map(CommandId::from).collect()
    }

    /// Expand to a flat list of exec-only command IDs (no composites), so `run_plan` need not recurse.
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
    ///
    /// # Errors
    ///
    /// [`ExpandError`] if `id` is unknown, the composite tree cycles, or
    /// expansion exceeds the depth limit.
    pub fn expand_to_leaves(&self, id: &str) -> Result<Vec<CommandId>, ExpandError> {
        let (leaves, _has_parallel, _fail_fast_disabled) = self.expand_to_leaves_with_flags(id)?;
        Ok(leaves)
    }

    /// PATTERN-1 / TASK-1283: walk the composite tree exactly once and
    /// return both the leaf ids and the aggregated `(any_parallel,
    /// fail_fast_disabled)` flags. `merge_plan` (and the raw single-command
    /// path) previously walked the same subtree twice — once via
    /// `expand_to_leaves` to collect leaves, then again via the CLI-side
    /// `composite_tree_flags` to recompute the flags. Two independent
    /// traversals can drift in cycle/order semantics; folding them here
    /// keeps the leaves and the flags in sync by construction.
    ///
    /// # Errors
    ///
    /// [`ExpandError`] if `id` is unknown, the composite tree cycles, expansion
    /// exceeds the depth limit, or the tree declares conflicting `parallel` /
    /// `fail_fast` values.
    pub fn expand_to_leaves_with_flags(
        &self,
        id: &str,
    ) -> Result<(Vec<CommandId>, bool, bool), ExpandError> {
        /// CQ-012: Maximum recursion depth for composite expansion.
        ///
        /// This limit prevents stack overflow from pathological configs with deeply
        /// nested composites (e.g., a -> b -> c -> ... -> z with 100+ levels). Normal
        /// configs typically have 2-5 levels (e.g., verify -> [build, test] -> cargo).
        /// The cycle detection already catches circular references, so this is a
        /// defense against accidental deep nesting.
        const MAX_DEPTH: usize = 100;
        let mut ctx = ExpandCtx {
            visited: std::collections::HashSet::new(),
            depth: 0,
            max_depth: MAX_DEPTH,
            any_parallel: false,
            fail_fast_disabled: false,
            parallel_decl: None,
            fail_fast_decl: None,
        };
        let leaves = self.expand_inner(id, &mut ctx)?;
        Ok((leaves, ctx.any_parallel, ctx.fail_fast_disabled))
    }

    fn expand_inner<'a>(
        &'a self,
        id: &str,
        ctx: &mut ExpandCtx<'a>,
    ) -> Result<Vec<CommandId>, ExpandError> {
        if ctx.depth > ctx.max_depth {
            tracing::warn!(
                id = ?id,
                depth = ctx.depth,
                max_depth = ctx.max_depth,
                "composite expansion depth limit exceeded"
            );
            return Err(ExpandError::DepthExceeded {
                id: id.to_string(),
                max_depth: ctx.max_depth,
            });
        }
        // PERF-3 / TASK-0766: fold canonical_id+resolve into one traversal
        // over the config / stack / extension / alias chain.
        let (canonical, spec) = self
            .canonical_with_spec(id)
            .ok_or_else(|| ExpandError::Unknown(UnknownCommand::new(id)))?;
        match spec {
            CommandSpec::Exec(_) => Ok(vec![CommandId::from(canonical)]),
            CommandSpec::Composite(c) => {
                // PATTERN-1 / TASK-0505: track only the active recursion
                // stack so a diamond DAG (A -> [B, C]; B, C -> [D]) does not
                // raise a false-positive cycle on the second visit to D.
                // True cycles (self-reference, A -> B -> A) still re-enter
                // a node already on the stack and trigger the check.
                //
                // OWN-8 (TASK-0714): visited stores `&'a str` borrowed from
                // the runner's command stores, so canonical names are not
                // cloned per recursion.
                if !ctx.visited.insert(canonical) {
                    return Err(ExpandError::Cycle(canonical.to_string()));
                }
                // TASK-1657: the plan is flat and scheduled as one unit, so
                // every composite in it must agree on the scheduling flags.
                // Checked before recursing so the error names the shallowest
                // offender rather than a deeper one that happens to differ.
                check_schedule_flag(&mut ctx.parallel_decl, "parallel", canonical, c.parallel)?;
                check_schedule_flag(&mut ctx.fail_fast_decl, "fail_fast", canonical, c.fail_fast)?;
                // PATTERN-1 / TASK-1283: aggregate parallel/fail_fast flags
                // along the same single pass that collects leaves. With the
                // agreement check above these are now uniform across the
                // plan, but the aggregation is kept so callers keep a single
                // source of truth for the effective scheduling.
                if c.parallel {
                    ctx.any_parallel = true;
                }
                if !c.fail_fast {
                    ctx.fail_fast_disabled = true;
                }
                let mut out = Vec::new();
                // `expand_inner` returns early above unless `ctx.depth <=
                // ctx.max_depth` (`MAX_DEPTH`), so the increment stays far
                // below `usize::MAX`, and the matching decrement only runs
                // after it, on a depth `>= 1`. Both saturating forms are
                // therefore exactly equal to `+= 1` / `-= 1` here.
                ctx.depth = ctx.depth.saturating_add(1);
                for sub in &c.commands {
                    out.extend(self.expand_inner(sub, ctx)?);
                }
                ctx.depth = ctx.depth.saturating_sub(1);
                ctx.visited.remove(canonical);
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
