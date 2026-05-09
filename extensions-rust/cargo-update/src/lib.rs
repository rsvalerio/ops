//! Cargo update extension: runs `cargo update --dry-run` and parses available dependency updates.
//!
//! This is a data-source-only extension (no commands). It provides parsed update
//! information that the about page consumes via the `--update` flag.

#[cfg(test)]
mod tests;

use ops_core::output::format_error_tail;
use ops_core::subprocess::{run_cargo, RunError};
use ops_extension::{
    Context, DataField, DataProvider, DataProviderError, DataProviderSchema, ExtensionType,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Output;
use std::time::Duration;

pub const NAME: &str = "cargo-update";
pub const DESCRIPTION: &str = "Cargo update dry-run: available dependency updates";
pub const SHORTNAME: &str = "update";
pub const DATA_PROVIDER_NAME: &str = "cargo_update";

/// The action type for a dependency update entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum UpdateAction {
    Update,
    Add,
    Remove,
}

/// A single dependency update entry parsed from `cargo update --dry-run` output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UpdateEntry {
    pub action: UpdateAction,
    pub name: String,
    /// Version being updated from (None for Add actions).
    pub from: Option<String>,
    /// Version being updated to (None for Remove actions).
    pub to: Option<String>,
}

/// Result of parsing `cargo update --dry-run` output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[must_use = "CargoUpdateResult carries the parsed update entries and counts — silently dropping it makes the cargo update --dry-run invocation observe nothing"]
#[non_exhaustive]
pub struct CargoUpdateResult {
    pub entries: Vec<UpdateEntry>,
    pub update_count: usize,
    pub add_count: usize,
    pub remove_count: usize,
}

/// Default timeout for `cargo update --dry-run`; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`.
pub const CARGO_UPDATE_TIMEOUT: Duration = Duration::from_secs(120);

/// Run `cargo update --dry-run` in the given working directory.
///
/// # Errors
///
/// Returns [`RunError::Io`] if the subprocess fails to spawn and
/// [`RunError::Timeout`] if it runs longer than [`CARGO_UPDATE_TIMEOUT`] (or
/// the `OPS_SUBPROCESS_TIMEOUT_SECS` override).
pub fn run_cargo_update_dry_run(working_dir: &Path) -> Result<Output, RunError> {
    run_cargo(
        &["update", "--dry-run"],
        working_dir,
        CARGO_UPDATE_TIMEOUT,
        "cargo update --dry-run",
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
/// - `Adding new-crate v0.1.0`
/// - `Removing old-crate v0.2.0`
///
/// Skips noise lines: `Updating crates.io index`, `Locking ...`, `warning:`, `note:`.
pub fn parse_update_output(stderr: &[u8]) -> CargoUpdateResult {
    let text = String::from_utf8_lossy(stderr);
    let mut entries = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        // PERF-3 / TASK-0970: skip the strip_ansi allocation when no escape
        // is present (the common case — terminals without color, redirected
        // CI output). The Cow path keeps the typed-result branches identical
        // for downstream parsing.
        let clean_cow = strip_ansi(trimmed);
        let clean = clean_cow.trim();

        // Skip noise lines
        if clean.is_empty()
            || clean.starts_with("Locking")
            || clean.starts_with("warning:")
            || clean.starts_with("note:")
        {
            continue;
        }

        // PATTERN-1 / TASK-1054: skip the "Updating <registry> index" noise
        // line only on its exact documented forms. The previous
        // `starts_with("Updating") && contains("index")` predicate matched
        // anywhere in the line and silently dropped legitimate updates for
        // crates whose names contain `index` (e.g. `Updating indexer v1.0.0
        // -> v1.0.1`). Guard on absence of the `->` arrow — the index-progress
        // line never carries one — to robustly distinguish noise from updates,
        // independent of registry naming.
        if clean.starts_with("Updating") && is_index_progress_line(clean) {
            continue;
        }

        if let Some(entry) = parse_action_line(clean) {
            entries.push(entry);
        } else if starts_with_known_verb(clean) {
            // TASK-0472: a line that begins with a known verb but did not
            // parse is highly likely to indicate cargo-update format drift.
            // Promote to warn so the count regression is observable at the
            // default log level — debug would silently disappear.
            tracing::warn!(
                line = ?clean,
                "skipping cargo-update line that begins with a known verb but did not parse — possible format drift"
            );
        }
    }

    let update_count = entries
        .iter()
        .filter(|e| e.action == UpdateAction::Update)
        .count();
    let add_count = entries
        .iter()
        .filter(|e| e.action == UpdateAction::Add)
        .count();
    let remove_count = entries
        .iter()
        .filter(|e| e.action == UpdateAction::Remove)
        .count();

    CargoUpdateResult {
        entries,
        update_count,
        add_count,
        remove_count,
    }
}

/// Strip ANSI escape sequences from a string.
///
/// Handles any CSI sequence: `ESC [ <params> <final byte>` where the final
/// byte is in `0x40..=0x7E` (covers SGR `m`, erase-line `K`, cursor-move `H`,
/// etc.). This is broader than the previous implementation which only consumed
/// up to `m` and left stray bytes from other CSI shapes.
///
/// ERR-1 / TASK-0882: iterate over `chars()` rather than raw bytes so a
/// non-ASCII UTF-8 sequence (localized cargo/rustc messages, crate
/// metadata with non-ASCII characters, tracing diagnostic lines) round-
/// trips identically. The previous `bytes[i] as char` cast interpreted
/// each continuation byte as a Latin-1 code point and silently corrupted
/// every multi-byte character.
fn strip_ansi(s: &str) -> std::borrow::Cow<'_, str> {
    // PERF-3 / TASK-0970: hot path on the data-source pipeline used by CI.
    // Fast-path the typical case (no `\x1b` in the line) by returning a
    // borrow — only allocate when we actually have to rewrite the string.
    if !s.contains('\x1b') {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    // PATTERN-1 / TASK-1028: bound the CSI scan so a truncated input
    // (`...\x1b[3` with no final byte before EOF) does not drain `chars`
    // to end-of-string and silently swallow trailing visible text.
    // Real CSI sequences are short (~10 bytes); 64 is generous.
    const CSI_SCAN_CAP: usize = 64;
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            // Consume the `[` then parameter/intermediate bytes (0x20..=0x3F)
            // and the final byte (0x40..=0x7E). All CSI bytes are ASCII so
            // matching against u32 code points is safe.
            chars.next();
            let mut buffered = String::new();
            let mut terminated = false;
            for _ in 0..CSI_SCAN_CAP {
                match chars.next() {
                    Some(next) => {
                        let cp = next as u32;
                        buffered.push(next);
                        if (0x40..=0x7E).contains(&cp) {
                            terminated = true;
                            break;
                        }
                    }
                    None => break,
                }
            }
            if !terminated {
                // Truncated or runaway CSI: emit a debug breadcrumb and
                // preserve the consumed-but-unterminated bytes (including the
                // `\x1b[` lead-in) so trailing visible text is not silently
                // dropped to EOF. Noisy by design — better than missing data.
                tracing::debug!(
                    buffered = ?buffered,
                    "strip_ansi: truncated or runaway CSI sequence; preserving buffered bytes"
                );
                result.push('\x1b');
                result.push('[');
                result.push_str(&buffered);
            }
        } else {
            result.push(c);
        }
    }
    std::borrow::Cow::Owned(result)
}

/// Whether the version after `name` represents the source (from) or target (to) version.
#[derive(Clone, Copy)]
enum VersionRole {
    From,
    To,
}

/// Table-driven dispatch for `Updating` / `Adding` / `Removing` lines.
///
/// Each entry maps a leading verb to its `UpdateAction` and the role of the
/// single version that follows the name (for `Updating`, both versions are
/// captured separately via the `->` arrow form).
const ACTION_PREFIXES: &[(&str, UpdateAction, VersionRole)] = &[
    ("Updating", UpdateAction::Update, VersionRole::From),
    ("Adding", UpdateAction::Add, VersionRole::To),
    ("Removing", UpdateAction::Remove, VersionRole::From),
];

/// PATTERN-1 / TASK-1054: distinguish the index-progress noise line
/// (`Updating crates.io index`, optionally with an alternate-registry
/// `(sparse+https://...)` suffix) from a real update line such as
/// `Updating indexer v1.0.0 -> v1.0.1`. The previous gate
/// `contains("index")` matched any crate name containing the substring
/// `index` and silently dropped legitimate updates.
///
/// Caller must already know `line` starts with `Updating`. Returns true
/// iff the line has the documented index-progress shape: the second
/// whitespace-separated token is exactly `index`, with at most a
/// parenthesised alternate-registry suffix after it. A real update line
/// always has the version (`v1.2.3`) as the third token, so it cannot
/// match this shape.
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
    // releases / locales — ERR-1 / TASK-1252). A real update line always
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
    match tokens.next() {
        None => true,
        Some(rest) => rest.starts_with('('),
    }
}

/// True when `line` starts with one of our recognised verb prefixes — used
/// solely to keep the tracing diagnostic narrow: lines that don't begin with
/// any known verb are noise (warnings, blank, etc.) and don't deserve a
/// "skipping cargo-update line" log.
fn starts_with_known_verb(line: &str) -> bool {
    // PATTERN-1 / TASK-1030: require a whitespace (or end-of-string) boundary
    // after the verb so prefix-without-boundary matches like `Updatingxyz ...`
    // do not classify as a known verb and emit false-positive drift warnings.
    let matches_verb = ACTION_PREFIXES.iter().any(|(prefix, _, _)| {
        line.starts_with(prefix)
            && line[prefix.len()..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace)
    });
    if !matches_verb {
        return false;
    }
    // ERR-1 / TASK-1252: only treat the line as a real action line (and
    // therefore worth a format-drift warn when parse_action_line fails) if
    // it carries a `v\d` version token. Progress lines such as the 2-token
    // `Updating crates.io` form share the `Updating` verb but have no
    // version, so without this guard parse_action_line's failure would
    // bubble into a bogus drift warn on every `ops about --refresh`.
    line.split_whitespace().any(is_version_token)
}

/// `true` iff `tok` matches the `v<digit>...` shape cargo emits for the
/// from/to versions on a real update line.
fn is_version_token(tok: &str) -> bool {
    let mut chars = tok.chars();
    chars.next() == Some('v') && chars.next().is_some_and(|c| c.is_ascii_digit())
}

/// Parse one of:
/// - `Updating serde v1.0.0 -> v1.0.1`
/// - `Adding new-crate v0.1.0`
/// - `Removing old-crate v0.2.0`
fn parse_action_line(line: &str) -> Option<UpdateEntry> {
    for &(prefix, action, role) in ACTION_PREFIXES {
        let Some(rest) = line.strip_prefix(prefix) else {
            continue;
        };
        // PATTERN-1 / TASK-1030: only fire when a whitespace boundary follows
        // the verb; otherwise `Updatingxyz serde v1 -> v2` would strip to
        // `xyz serde v1 -> v2` and consume the line.
        if !rest.chars().next().is_none_or(char::is_whitespace) {
            continue;
        }
        let rest = rest.trim();

        // TASK-0476: iterator-based destructuring avoids the per-line
        // `Vec<&str>` allocation that `splitn(...).collect()` introduces on
        // a hot path (must_use provider runs in CI metadata pipelines).
        if matches!(action, UpdateAction::Update) {
            let mut it = rest.split_whitespace();
            let name = it.next()?;
            let from = it.next()?;
            let arrow = it.next()?;
            let to = it.next()?;
            if arrow != "->" {
                return None;
            }
            // TASK-0613: a future cargo could append annotations such as
            // `Updating serde v1 -> v2 (yanked)`. The previous `splitn(4, ' ')`
            // silently glued the trailing tokens onto `to`, corrupting the
            // version. Warn loudly so format drift is visible instead of
            // producing wrong-but-plausible output.
            if it.next().is_some() {
                tracing::warn!(line = ?line, "cargo-update `Updating` line has unexpected trailing tokens; annotation discarded");
            }
            return Some(UpdateEntry {
                action,
                name: name.to_string(),
                from: Some(strip_v_prefix(from).to_string()),
                to: Some(strip_v_prefix(to).to_string()),
            });
        }

        // TASK-0949: mirror the `Updating` arm — reject `<name> <version>
        // <extra…>` so a future cargo annotation like `Adding new-crate v0.1.0
        // (locked)` does not silently get glued onto the parsed version.
        let mut it = rest.split_whitespace();
        let name = it.next()?;
        let version_raw = it.next()?;
        if it.next().is_some() {
            tracing::warn!(
                line = ?line,
                "cargo-update `Adding`/`Removing` line has unexpected trailing tokens; annotation discarded"
            );
        }
        let version = Some(strip_v_prefix(version_raw).to_string());
        let (from, to) = match role {
            VersionRole::From => (version, None),
            VersionRole::To => (None, version),
        };
        return Some(UpdateEntry {
            action,
            name: name.to_string(),
            from,
            to,
        });
    }
    None
}

/// API-9 / TASK-0922: construct via the registered extension factory only.
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
        registry.register(DATA_PROVIDER_NAME, Box::new(CargoUpdateProvider));
    },
    factory: CARGO_UPDATE_FACTORY = |_, _| {
        Some((NAME, Box::new(CargoUpdateExtension)))
    },
}

/// Data provider that runs `cargo update --dry-run` and returns parsed results.
pub struct CargoUpdateProvider;

impl DataProvider for CargoUpdateProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let output = run_cargo_update_dry_run(&ctx.working_directory).map_err(|e| {
            DataProviderError::from(anyhow::anyhow!("cargo update --dry-run failed: {}", e))
        })?;

        // TASK-0502: a successful spawn with a non-zero exit (e.g. lockfile
        // contention, network error, malformed Cargo.toml) leaves stderr
        // *not* shaped like the dry-run report. Parsing it would silently
        // produce an empty `CargoUpdateResult` — i.e. "no updates available"
        // for a failed invocation. Surface the error like sibling providers
        // (test-coverage, metadata, deps) instead.
        if !output.status.success() {
            let stderr_tail = format_error_tail(&output.stderr, 10);
            return Err(DataProviderError::from(anyhow::anyhow!(
                "cargo update --dry-run exited with status {}: {}",
                output.status,
                stderr_tail
            )));
        }

        let result = parse_update_output(&output.stderr);
        serde_json::to_value(&result).map_err(DataProviderError::from)
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
                DataField::new("add_count", "usize", "Number of new dependencies to add"),
                DataField::new("remove_count", "usize", "Number of dependencies to remove"),
            ],
        )
    }
}
