//! Cargo update extension: runs `cargo update --dry-run` and parses available dependency updates.
//!
//! This is a data-source-only extension (no commands). It provides parsed update
//! information that the about page consumes via the `--update` flag.

// The test module uses `unwrap` freely; production code does not. No other
// lint relaxation is needed — the crate performs no numeric casts.
#![cfg_attr(test, allow(clippy::unwrap_used))]

#[cfg(test)]
mod tests;

use ops_core::output::format_error_tail;
use ops_core::subprocess::{run_cargo, RunError};
use ops_extension::{
    Context, DataField, DataProvider, DataProviderError, DataProviderSchema, ExtensionType,
};
use ops_theme::strip_ansi_preserving_raw;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Output;
use std::time::Duration;

/// Extension identifier used to register this crate in the engine's
/// extension registry.
pub const NAME: &str = "cargo-update";
/// One-line description shown by `ops about` for this extension.
pub const DESCRIPTION: &str = "Cargo update dry-run: available dependency updates";
/// CLI-facing short name (`update`) used in commands and user-facing output.
pub const SHORTNAME: &str = "update";
/// Registry key of the `cargo_update` data provider this crate registers —
/// the key the about page's `--update` view looks the parsed entries up by.
pub const DATA_PROVIDER_NAME: &str = "cargo_update";

/// The action type for a dependency update entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum UpdateAction {
    /// A dependency moves from one version to another.
    Update,
    /// A dependency moves *down* to a lower version.
    ///
    /// Cargo's lockfile-change printer emits `Downgrading` alongside
    /// `Updating` / `Adding` / `Removing` whenever the lockfile holds a version
    /// above what `Cargo.toml` now requires — a tightened requirement, a lifted
    /// `[patch]`, a yanked release.
    Downgrade,
    /// A dependency appears in the lockfile with no prior version.
    Add,
    /// A dependency leaves the lockfile with no replacement version.
    Remove,
}

/// One parsed dependency change: a cargo lockfile verb plus exactly the
/// versions that verb carries.
///
/// `Update`/`Downgrade` carry both versions, `Add` only `to`, `Remove` only
/// `from`. Making version presence part of the variant means an entry can only
/// exist in one of the four valid shapes — the type itself rules out
/// combinations the parser can never produce.
///
/// Serialized as an internally-tagged JSON object with lowercase verbs
/// (`{"action":"update","name":…,"from":…,"to":…}`); a variant's absent version
/// is omitted rather than emitted as `null`. Deserialization — the about page
/// reads this JSON back from a cache — is validating: a payload whose action
/// and version presence disagree either fails (a version the action requires is
/// missing or `null`) or normalizes (a version foreign to the action is
/// ignored), never yielding a silently invalid entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
#[non_exhaustive]
pub enum UpdateEntry {
    /// `Updating <name> <from> -> <to>`.
    Update {
        /// Crate name.
        name: String,
        /// Version being updated from.
        from: String,
        /// Version being updated to.
        to: String,
    },
    /// `Downgrading <name> <from> -> <to>`.
    Downgrade {
        /// Crate name.
        name: String,
        /// Version being downgraded from.
        from: String,
        /// Version being downgraded to.
        to: String,
    },
    /// `Adding <name> <to>` — no prior version exists.
    Add {
        /// Crate name.
        name: String,
        /// Version being added.
        to: String,
    },
    /// `Removing <name> <from>` — no replacement version exists.
    Remove {
        /// Crate name.
        name: String,
        /// Version being removed.
        from: String,
    },
}

impl UpdateEntry {
    /// The lockfile verb this entry was parsed from.
    #[must_use]
    pub const fn action(&self) -> UpdateAction {
        match self {
            Self::Update { .. } => UpdateAction::Update,
            Self::Downgrade { .. } => UpdateAction::Downgrade,
            Self::Add { .. } => UpdateAction::Add,
            Self::Remove { .. } => UpdateAction::Remove,
        }
    }

    /// The crate name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name_and_versions().0
    }

    /// Version moved from — `None` exactly on [`UpdateEntry::Add`], the one
    /// action that has no prior version.
    #[must_use]
    pub fn from(&self) -> Option<&str> {
        let (_, from, _) = self.name_and_versions();
        from
    }

    /// Version moved to — `None` exactly on [`UpdateEntry::Remove`], the one
    /// action that removes without replacing.
    #[must_use]
    pub fn to(&self) -> Option<&str> {
        let (_, _, to) = self.name_and_versions();
        to
    }

    /// Single destructure point for the accessors: `(name, from, to)` with
    /// the absent sides `None`, mirroring the variant structure.
    fn name_and_versions(&self) -> (&str, Option<&str>, Option<&str>) {
        match self {
            Self::Update { name, from, to } | Self::Downgrade { name, from, to } => {
                (name, Some(from), Some(to))
            }
            Self::Add { name, to } => (name, None, Some(to)),
            Self::Remove { name, from } => (name, Some(from), None),
        }
    }
}

/// Result of parsing `cargo update --dry-run` output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[must_use = "CargoUpdateResult carries the parsed update entries and counts — silently dropping it makes the cargo update --dry-run invocation observe nothing"]
#[non_exhaustive]
pub struct CargoUpdateResult {
    /// One entry per parsed action line, in cargo's output order.
    pub entries: Vec<UpdateEntry>,
    /// Number of `Updating` lines in [`CargoUpdateResult::entries`].
    pub update_count: usize,
    /// Number of `Downgrading` lines in [`CargoUpdateResult::entries`].
    ///
    /// `#[serde(default)]` so a cached payload that omits the field still
    /// deserializes — the about page consumes this JSON from a cache that can
    /// outlive the schema it was written against.
    #[serde(default)]
    pub downgrade_count: usize,
    /// Number of `Adding` lines in [`CargoUpdateResult::entries`].
    pub add_count: usize,
    /// Number of `Removing` lines in [`CargoUpdateResult::entries`].
    pub remove_count: usize,
}

/// Default timeout for `cargo update --dry-run`; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`.
pub const CARGO_UPDATE_TIMEOUT: Duration = Duration::from_mins(2);

/// Argv handed to `cargo` by [`run_cargo_update_dry_run`].
const CARGO_UPDATE_ARGS: &[&str] = &["update", "--dry-run"];

/// Operator-facing label for the subprocess invocation.
const CARGO_UPDATE_LABEL: &str = "cargo update --dry-run";

/// The exact subprocess invocation [`run_cargo_update_dry_run`] performs,
/// expressed as data.
///
/// Describing the call separately from making it lets a test pin the argv,
/// working directory, timeout and label without spawning cargo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CargoUpdateInvocation<'a> {
    args: &'static [&'static str],
    working_dir: &'a Path,
    timeout: Duration,
    label: &'static str,
}

/// The invocation [`run_cargo_update_dry_run`] runs for `working_dir`.
const fn cargo_update_invocation(working_dir: &Path) -> CargoUpdateInvocation<'_> {
    CargoUpdateInvocation {
        args: CARGO_UPDATE_ARGS,
        working_dir,
        timeout: CARGO_UPDATE_TIMEOUT,
        label: CARGO_UPDATE_LABEL,
    }
}

/// Run `cargo update --dry-run` in the given working directory.
///
/// # Errors
///
/// Returns [`RunError::Io`] if the subprocess fails to spawn and
/// [`RunError::Timeout`] if it runs longer than [`CARGO_UPDATE_TIMEOUT`] (or
/// the `OPS_SUBPROCESS_TIMEOUT_SECS` override).
pub fn run_cargo_update_dry_run(working_dir: &Path) -> Result<Output, RunError> {
    let invocation = cargo_update_invocation(working_dir);
    run_cargo(
        invocation.args,
        invocation.working_dir,
        invocation.timeout,
        invocation.label,
    )
}

/// Strip leading `v` prefix from a version string.
fn strip_v_prefix(version: &str) -> &str {
    version.strip_prefix('v').unwrap_or(version)
}

/// Parse the stderr output of `cargo update --dry-run` into structured data.
///
/// Handles lines like:
/// - `Updating serde v1.0.0 -> v1.0.1`
/// - `Downgrading serde v1.0.220 -> v1.0.219`
/// - `Adding new-crate v0.1.0`
/// - `Removing old-crate v0.2.0`
///
/// Skips noise lines: `Updating crates.io index`, `Locking ...`, `Unchanged ...`,
/// `warning:`, `note:`.
pub fn parse_update_output(stderr: &[u8]) -> CargoUpdateResult {
    let text = String::from_utf8_lossy(stderr);
    let mut entries = Vec::new();
    // Accumulate per-action counts during the parse loop so `entries` is
    // walked once rather than once per action afterwards.
    let mut update_count = 0usize;
    let mut downgrade_count = 0usize;
    let mut add_count = 0usize;
    let mut remove_count = 0usize;

    for line in text.lines() {
        let trimmed = line.trim();

        // The Cow return skips the strip allocation when the line carries no
        // escape — the common case for terminals without color and for
        // redirected CI output — while keeping the downstream parse identical
        // either way.
        let clean_cow = strip_ansi_preserving_raw(trimmed);
        let clean = clean_cow.trim();

        // Skip noise lines. `Unchanged` is the verbose-only arm of cargo's
        // lockfile-change printer and carries no change, so it is filtered
        // deliberately here rather than falling through as unrecognised.
        if clean.is_empty()
            || clean.starts_with("Locking")
            || clean.starts_with("Unchanged")
            || clean.starts_with("warning:")
            || clean.starts_with("note:")
        {
            continue;
        }

        // Skip the "Updating <registry> index" noise line, matching only its
        // exact documented forms. Matching `index` anywhere in the line would
        // drop legitimate updates for crates whose names contain it (e.g.
        // `Updating indexer v1.0.0 -> v1.0.1`), so the shape test keys on token
        // position instead, independent of registry naming.
        if clean.starts_with("Updating") && is_index_progress_line(clean) {
            continue;
        }

        match parse_action_line(clean) {
            ActionLineOutcome::Parsed(entry) => {
                // At most one increment per line of the in-memory `stderr`
                // string, whose length is bounded by `isize::MAX`, so
                // `saturating_add` equals `+= 1` exactly.
                match entry.action() {
                    UpdateAction::Update => update_count = update_count.saturating_add(1),
                    UpdateAction::Downgrade => downgrade_count = downgrade_count.saturating_add(1),
                    UpdateAction::Add => add_count = add_count.saturating_add(1),
                    UpdateAction::Remove => remove_count = remove_count.saturating_add(1),
                }
                entries.push(entry);
            }
            // The verb matched but a field failed validation. Such a line is
            // never published as an entry, and never dropped silently either.
            ActionLineOutcome::Rejected(reason) => {
                tracing::warn!(
                    line = ?clean,
                    reason,
                    "skipping cargo-update line whose parsed fields failed validation"
                );
            }
            // A line that begins with a known verb but did not parse most
            // likely indicates cargo-update format drift. It is logged at warn
            // so the resulting count regression is observable at the default
            // log level; at debug it would disappear.
            ActionLineOutcome::NoMatch => {
                if starts_with_known_verb(clean) {
                    tracing::warn!(
                        line = ?clean,
                        "skipping cargo-update line that begins with a known verb but did not parse — possible format drift"
                    );
                }
            }
        }
    }

    CargoUpdateResult {
        entries,
        update_count,
        downgrade_count,
        add_count,
        remove_count,
    }
}

// ANSI stripping lives in `ops-theme`. `ops_theme::strip_ansi_preserving_raw`
// runs the workspace's single ANSI grammar — the same iterator `strip_ansi` /
// `visible_width` / `truncate_to_width` consume — under the policy this parser
// needs: complete sequences are removed, while truncated or runaway escapes and
// stray introducers survive verbatim so the field validators below reject the
// line instead of silently swallowing visible text.

/// Shape of the version portion that follows the crate name on an action line.
#[derive(Clone, Copy, Debug)]
enum VersionShape {
    /// `<from> -> <to>` — both versions present, separated by the arrow.
    Arrow,
    /// A single version recorded as the `from` version.
    From,
    /// A single version recorded as the `to` version.
    To,
}

/// Table-driven dispatch for cargo's lockfile-change verbs.
///
/// Each entry maps a leading verb to its [`UpdateAction`] and the shape of the
/// version portion that follows the crate name.
///
/// The table must list every verb cargo's `print_lockfile_updates` printer
/// emits — `Unchanged` is the one exception, filtered as noise in
/// [`parse_update_output`] because it is verbose-only and carries no change.
const ACTION_PREFIXES: &[(&str, UpdateAction, VersionShape)] = &[
    ("Updating", UpdateAction::Update, VersionShape::Arrow),
    ("Downgrading", UpdateAction::Downgrade, VersionShape::Arrow),
    ("Adding", UpdateAction::Add, VersionShape::To),
    ("Removing", UpdateAction::Remove, VersionShape::From),
];

/// `true` iff `line` is the index-progress noise line (`Updating crates.io
/// index`, optionally with an alternate-registry `(sparse+https://...)`
/// suffix) rather than a real update such as `Updating indexer v1.0.0 ->
/// v1.0.1`.
///
/// Caller must already know `line` starts with `Updating`. The test is
/// positional rather than a substring search, so a crate whose *name* contains
/// `index` is never mistaken for progress noise: the shape requires `index` as
/// the third whitespace-separated token, with at most a parenthesised
/// alternate-registry suffix after it, while a real update line always carries
/// the version (`v1.2.3`) in that position.
fn is_index_progress_line(line: &str) -> bool {
    let mut tokens = line.split_whitespace();
    // First token is "Updating" (caller guarantees).
    if tokens.next() != Some("Updating") {
        return false;
    }
    // Second token: registry name (e.g. `crates.io`, `github.com`,
    // `my-registry`). Any non-empty token is acceptable.
    if tokens.next().is_none() {
        return false;
    }
    // Third token: either `index` (canonical 3-token form) or absent
    // (2-token form `Updating crates.io` observed on some cargo
    // releases / locales). A real update line always
    // has the from-version (`v1.0.0`) here, so a 2-token line cannot be
    // confused with a real update.
    let Some(third) = tokens.next() else {
        return true;
    };
    if third != "index" {
        return false;
    }
    // Anything after `index` must be the alternate-registry suffix in
    // parens, e.g. `(sparse+https://index.crates.io/)`. Crucially, a real
    // update would have ` -> vX.Y.Z` here.
    tokens.next().is_none_or(|rest| rest.starts_with('('))
}

/// Matches `line` against [`ACTION_PREFIXES`], returning the matched action,
/// its version shape, and the trimmed remainder of the line after the verb.
///
/// This is the single definition of "does `line` open with a known verb,
/// followed by a whitespace boundary?", shared by [`parse_action_line`] and
/// [`starts_with_known_verb`] so both agree by construction. The whitespace
/// boundary is what stops a prefix-without-boundary match like
/// `Updatingxyz serde v1 -> v2` from classifying as a known verb (a
/// false-positive drift warning) and from being consumed by the caller's
/// `strip_prefix`.
fn match_verb(line: &str) -> Option<(UpdateAction, VersionShape, &str)> {
    ACTION_PREFIXES.iter().find_map(|&(prefix, action, shape)| {
        let rest = line.strip_prefix(prefix)?;
        rest.chars()
            .next()
            .is_none_or(char::is_whitespace)
            .then(|| (action, shape, rest.trim()))
    })
}

/// True when `line` starts with one of our recognised verb prefixes — used
/// solely to keep the tracing diagnostic narrow: lines that don't begin with
/// any known verb are noise (warnings, blank, etc.) and don't deserve a
/// "skipping cargo-update line" log.
fn starts_with_known_verb(line: &str) -> bool {
    if match_verb(line).is_none() {
        return false;
    }
    // Only a line carrying a `v\d` version token counts as a real action line,
    // and therefore as worth a format-drift warn when parse_action_line fails.
    // Progress lines such as the 2-token `Updating crates.io` form, or
    // `Updating git repository \`...\``, share the `Updating` verb but have no
    // version; without this guard their parse failure would produce a bogus
    // drift warn on every `ops about --refresh`.
    line.split_whitespace().any(is_version_token)
}

/// `true` iff `tok` matches the `v<digit>...` shape cargo emits for the
/// from/to versions on a real update line.
fn is_version_token(tok: &str) -> bool {
    let mut chars = tok.chars();
    chars.next() == Some('v') && chars.next().is_some_and(|c| c.is_ascii_digit())
}

/// `true` iff `tok` is shaped like a version cargo would print — an optional
/// `v` prefix followed by an ASCII digit — and carries no control characters.
///
/// Looser than [`is_version_token`] on purpose: the bare-numeric form
/// (`Updating serde 1.0.0 -> 1.0.1`) has always been accepted by the parser,
/// while [`is_version_token`] gates the drift warn and must not fire on
/// progress lines.
fn is_version_shaped(tok: &str) -> bool {
    let version = strip_v_prefix(tok);
    version.starts_with(|c: char| c.is_ascii_digit()) && is_control_free(version)
}

/// `true` iff `tok` carries no control character.
///
/// [`ops_theme::strip_ansi_preserving_raw`] deliberately preserves truncated
/// escape sequences and bare `ESC` bytes so visible text is never swallowed, so
/// a field reaching this point can still contain `ESC`, `NUL`, `BEL`, ... Crate
/// names and versions never legitimately do, and these values are serialised
/// into the provider JSON the about page renders to an operator's terminal.
fn is_control_free(tok: &str) -> bool {
    !tok.chars().any(char::is_control)
}

/// Outcome of interpreting a single non-noise line.
enum ActionLineOutcome {
    /// A well-formed action line.
    Parsed(UpdateEntry),
    /// A known verb whose fields failed validation. Carries the reason for the
    /// caller's single warn site; never produces an entry.
    Rejected(&'static str),
    /// Not an action line at all (unknown verb, or a shape the parser does not
    /// recognise). The caller decides whether this is format drift.
    NoMatch,
}

/// Parse one of:
/// - `Updating serde v1.0.0 -> v1.0.1`
/// - `Downgrading serde v1.0.220 -> v1.0.219`
/// - `Adding new-crate v0.1.0`
/// - `Removing old-crate v0.2.0`
fn parse_action_line(line: &str) -> ActionLineOutcome {
    let Some((action, shape, rest)) = match_verb(line) else {
        return ActionLineOutcome::NoMatch;
    };

    // Iterator-based destructuring keeps this line loop allocation-free; it
    // runs over every stderr line in CI metadata pipelines.
    let mut it = rest.split_whitespace();
    let Some(name) = it.next() else {
        return ActionLineOutcome::NoMatch;
    };

    if matches!(shape, VersionShape::Arrow) {
        let (Some(from), Some(arrow), Some(to)) = (it.next(), it.next(), it.next()) else {
            return ActionLineOutcome::NoMatch;
        };
        if arrow != "->" {
            return ActionLineOutcome::NoMatch;
        }
        // A future cargo could append annotations such as
        // `Updating serde v1 -> v2 (yanked)`. Splitting on whitespace keeps the
        // extra tokens out of `to` rather than gluing them onto the version;
        // warn so the format drift is visible instead of producing
        // wrong-but-plausible output.
        if it.next().is_some() {
            tracing::warn!(line = ?line, "cargo-update `Updating`/`Downgrading` line has unexpected trailing tokens; annotation discarded");
        }
        if !is_control_free(name) {
            return ActionLineOutcome::Rejected("crate name carries control characters");
        }
        if !is_version_shaped(from) || !is_version_shaped(to) {
            return ActionLineOutcome::Rejected("version token is not shaped like a version");
        }
        // The variant, not a doc comment, states which versions this action
        // carries.
        let entry = match action {
            UpdateAction::Update => UpdateEntry::Update {
                name: name.to_string(),
                from: strip_v_prefix(from).to_string(),
                to: strip_v_prefix(to).to_string(),
            },
            UpdateAction::Downgrade => UpdateEntry::Downgrade {
                name: name.to_string(),
                from: strip_v_prefix(from).to_string(),
                to: strip_v_prefix(to).to_string(),
            },
            // ACTION_PREFIXES pairs the Arrow shape only with Update and
            // Downgrade; Add/Remove carry single-version shapes below. There
            // is no non-panicking body for this arm — per docs/clippy.md the
            // invariant lives in ACTION_PREFIXES, not here.
            #[allow(clippy::unreachable)]
            other => unreachable!("Arrow shape paired with {other:?} in ACTION_PREFIXES"),
        };
        return ActionLineOutcome::Parsed(entry);
    }

    // Mirrors the `Updating` arm: `<name> <version> <extra…>` warns rather than
    // gluing a future cargo annotation like `Adding new-crate v0.1.0 (locked)`
    // onto the parsed version.
    let Some(version_raw) = it.next() else {
        return ActionLineOutcome::NoMatch;
    };
    if it.next().is_some() {
        tracing::warn!(
            line = ?line,
            "cargo-update `Adding`/`Removing` line has unexpected trailing tokens; annotation discarded"
        );
    }
    if !is_control_free(name) {
        return ActionLineOutcome::Rejected("crate name carries control characters");
    }
    // The version position is validated, not merely occupied: without this
    // check `Adding new-crate (locked) v0.1.0` would publish `(locked)` as the
    // version, and `Adding foo v` would publish `Some("")`, which reads as a
    // known version to every consumer that checks `is_some()`.
    if !is_version_shaped(version_raw) {
        return ActionLineOutcome::Rejected("version token is not shaped like a version");
    }
    // The variant, not a doc comment, states which versions each action
    // carries.
    let entry = match shape {
        VersionShape::From => UpdateEntry::Remove {
            name: name.to_string(),
            from: strip_v_prefix(version_raw).to_string(),
        },
        VersionShape::To => UpdateEntry::Add {
            name: name.to_string(),
            to: strip_v_prefix(version_raw).to_string(),
        },
        // The arrow shape returned above, so this arm cannot run; there is
        // no non-panicking body for it — per docs/clippy.md the invariant
        // lives in the early return above, not here.
        #[allow(clippy::unreachable)]
        other @ VersionShape::Arrow => {
            unreachable!("single-version branch reached with {other:?} shape")
        }
    };
    ActionLineOutcome::Parsed(entry)
}

/// Datasource extension exposing parsed `cargo update --dry-run` results
/// under the [`DATA_PROVIDER_NAME`] key.
///
/// Construct it through the registered extension factory, not directly.
#[non_exhaustive]
pub struct CargoUpdateExtension;

ops_extension::impl_extension! {
    CargoUpdateExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Rust),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        let _ = registry.register(DATA_PROVIDER_NAME, Box::new(CargoUpdateProvider));
    },
    factory: CARGO_UPDATE_FACTORY = |_, _| {
        Some((NAME, Box::new(CargoUpdateExtension)))
    },
}

/// Converts a subprocess [`RunError`] into a [`DataProviderError`], keeping the
/// error's source chain intact.
///
/// `anyhow::Error::new(e).context(...)` preserves the chain rather than
/// flattening it to Display, so downstream consumers (structured logs, error
/// inspectors) can walk `.source()` / `anyhow::Chain` to distinguish spawn
/// failures from timeouts. It is a named function so a test can assert on the
/// value production actually produces rather than on a rebuilt copy of this
/// expression.
fn map_run_error(err: RunError) -> DataProviderError {
    DataProviderError::from(anyhow::Error::new(err).context("cargo update --dry-run failed"))
}

/// Turns a finished `cargo update --dry-run` [`Output`] into the provider's
/// JSON value.
///
/// This is the output-interpretation half of [`CargoUpdateProvider::provide`],
/// kept separate so every branch below is reachable from a test with a
/// hand-built [`Output`] and no subprocess. Mirrors
/// `deps::interpret_upgrade_output`.
///
/// # Errors
///
/// Returns [`DataProviderError::ComputationFailed`] when `cargo` exited
/// non-zero, and [`DataProviderError::Serialization`] if the parsed result
/// cannot be encoded as JSON.
fn interpret_output(output: &Output) -> Result<serde_json::Value, DataProviderError> {
    // A successful spawn with a non-zero exit (lockfile contention, network
    // error, malformed Cargo.toml) leaves stderr *not* shaped like the dry-run
    // report. Parsing it would produce an empty `CargoUpdateResult` — i.e. "no
    // updates available" for a failed invocation — so the error is surfaced
    // instead, matching sibling providers (test-coverage, metadata, deps).
    if !output.status.success() {
        let stderr_tail = format_error_tail(&output.stderr, 10);
        // `format_error_tail` normalises CR/CRLF/bare-CR but does NOT scrub
        // other C0 control bytes (ESC `\x1b`, BEL, NUL, ...). Cargo's stderr is
        // influenced by crate names, version strings and registry metadata —
        // surface an attacker can shape via a poisoned crate. The tail goes
        // through the Debug formatter (`{:?}`) so embedded ANSI escapes, NULs
        // or newlines cannot forge log records or repaint the operator's
        // terminal, matching the sibling sites in deps
        // (`interpret_upgrade_output` / `interpret_deny_result`).
        return Err(DataProviderError::from(anyhow::anyhow!(
            "cargo update --dry-run exited with status {}: {:?}",
            output.status,
            stderr_tail
        )));
    }

    // Cargo prints the dry-run lockfile report on stderr, not stdout.
    let result = parse_update_output(&output.stderr);
    serde_json::to_value(&result).map_err(DataProviderError::from)
}

/// Data provider that runs `cargo update --dry-run` and returns parsed results.
pub struct CargoUpdateProvider;

impl DataProvider for CargoUpdateProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let output = run_cargo_update_dry_run(ctx.working_directory()).map_err(map_run_error)?;
        interpret_output(&output)
    }

    fn schema(&self) -> DataProviderSchema {
        DataProviderSchema::new(
            "Available dependency updates from cargo update --dry-run",
            vec![
                DataField::new(
                    "entries",
                    "Vec<UpdateEntry>",
                    "List of dependency update/add/remove entries",
                ),
                DataField::new("update_count", "usize", "Number of updates available"),
                DataField::new("downgrade_count", "usize", "Number of downgrades available"),
                DataField::new("add_count", "usize", "Number of new dependencies to add"),
                DataField::new("remove_count", "usize", "Number of dependencies to remove"),
            ],
        )
    }
}
