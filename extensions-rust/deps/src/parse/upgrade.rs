//! Parser for `cargo upgrade --dry-run` table output.

use crate::{UpgradeEntry, UpgradeResult};
use ops_core::subprocess::run_cargo;
use std::path::Path;
use std::time::Duration;

use super::truncate_for_log;

/// Default timeout for `cargo upgrade --dry-run`; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`.
const CARGO_UPGRADE_TIMEOUT: Duration = Duration::from_mins(3);

/// Run `cargo upgrade --dry-run` and parse the table output.
///
/// ERR-1 (TASK-0913): `cargo upgrade` exits non-zero on lockfile contention,
/// network failures, or a malformed `Cargo.toml`. The previous code parsed
/// stdout regardless of exit status and silently returned an empty
/// `Vec<UpgradeEntry>`, masking the upstream failure as "no upgrades
/// available". Surface non-zero exits as an error including the stderr
/// tail so the deps gate fails loudly. Mirrors the cargo-update fix made
/// in TASK-0502 and the cargo-deny exit-code handling below.
///
/// # Errors
///
/// If `cargo upgrade` cannot be spawned, exceeds its timeout, exits
/// non-zero, or emits a table whose header or row shape is unrecognised.
pub fn run_cargo_upgrade_dry_run(working_dir: &Path) -> anyhow::Result<Vec<UpgradeEntry>> {
    let output = run_cargo(
        &["upgrade", "--dry-run"],
        working_dir,
        CARGO_UPGRADE_TIMEOUT,
        "cargo upgrade --dry-run",
    )
    .map_err(|e| anyhow::anyhow!("failed to run cargo upgrade: {e}"))?;

    interpret_upgrade_output(output.status.code(), &output.stdout, &output.stderr)
}

/// Interpret an already-collected `cargo upgrade --dry-run` result.
///
/// Maps an `(exit_code, stdout, stderr)` triple to either a parsed
/// `Vec<UpgradeEntry>` or an error carrying the stderr tail. Split out from
/// [`run_cargo_upgrade_dry_run`] so callers (and tests) can pin the
/// exit-code and format-drift semantics without spawning the binary.
///
/// ARCH-9 / TASK-1846: this is the *guarded* upgrade entry point, published
/// on the same terms as [`super::interpret_deny_result`]. It is the only
/// path that applies [`check_missing_separator_drift`],
/// [`check_header_drift`] and [`check_row_shape_drift`], so it is what
/// callers who want the crate's fail-closed posture should reach for.
///
/// # Errors
///
/// If `cargo upgrade` was killed by a signal, exited non-zero, or produced
/// a table whose separator row, header line, or row shape indicates
/// cargo-edit format drift — in every case rather than scoring the run as
/// "no upgrades available".
pub fn interpret_upgrade_output(
    exit_code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
) -> anyhow::Result<Vec<UpgradeEntry>> {
    match exit_code {
        Some(0) => {
            let stdout = String::from_utf8_lossy(stdout);
            let (entries, diag) = parse_upgrade_table_inner(&stdout);
            check_missing_separator_drift(&diag)?;
            check_header_drift(&diag)?;
            check_row_shape_drift(&diag)?;
            Ok(entries)
        }
        None => anyhow::bail!(
            "cargo upgrade --dry-run terminated by signal (exit_code = None); \
             refusing to treat partial output as authoritative"
        ),
        Some(other) => {
            let stderr = String::from_utf8_lossy(stderr);
            anyhow::bail!(
                "cargo upgrade --dry-run exited with status {other}; \
                 refusing to parse output as authoritative. \
                 stderr (truncated): {:?}",
                truncate_for_log(stderr.trim())
            )
        }
    }
}

/// ERR-1 / TASK-1817: the third fail-open permutation. `check_header_drift`
/// (TASK-1074) and `check_row_shape_drift` (TASK-1202) are both gated on
/// `saw_separator`, so output that carries content lines but *no* `====`
/// separator row escaped both guards and scored as "no upgrades available".
/// The separator is pure decoration upstream, which makes dropping it the
/// most likely rendering change cargo-edit could make.
fn check_missing_separator_drift(diag: &UpgradeParseDiagnostics) -> anyhow::Result<()> {
    if !diag.saw_separator && diag.content_lines > 0 {
        anyhow::bail!(
            "cargo upgrade --dry-run produced {content_lines} content line(s) but no `====` \
             separator row, so no column geometry could be derived; refusing to score as \
             `no upgrades` — suspect cargo-edit table-rendering drift",
            content_lines = diag.content_lines
        );
    }
    Ok(())
}

fn check_header_drift(diag: &UpgradeParseDiagnostics) -> anyhow::Result<()> {
    if diag.saw_separator && !diag.saw_recognised_header {
        tracing::warn!(
            "TASK-1074: cargo-upgrade stdout had a `====` separator row but no recognised header line; \
             refusing to parse output as authoritative — suspect cargo-edit header-token drift"
        );
        anyhow::bail!(
            "cargo upgrade --dry-run produced a table whose header line was not recognised \
             (no `name` / `old req` / `new req` tokens); refusing to score as `no upgrades` — \
             suspect cargo-edit format drift"
        );
    }
    Ok(())
}

fn check_row_shape_drift(diag: &UpgradeParseDiagnostics) -> anyhow::Result<()> {
    if diag.saw_recognised_header
        && diag.saw_separator
        && diag.body_lines > 0
        && diag.entries_emitted == 0
    {
        tracing::warn!(
            body_lines = diag.body_lines,
            "TASK-1202: cargo-upgrade stdout had a recognised header, a `====` separator, \
             and body lines, but every row failed parse_upgrade_row (column-shape drift); \
             refusing to parse output as authoritative"
        );
        anyhow::bail!(
            "cargo upgrade --dry-run produced {body_lines} body row(s) but none filled the \
             5 fixed columns; refusing to score as `no upgrades` — suspect cargo-edit \
             row-shape drift",
            body_lines = diag.body_lines
        );
    }
    Ok(())
}

/// Parse the table output from `cargo upgrade --dry-run`.
///
/// SEC-15 / TASK-0383: column offsets are calibrated from the `====` separator
/// row rather than splitting on whitespace, so multi-word notes (e.g. "pinned
/// by parent") and any future column additions don't silently shift values
/// across `UpgradeEntry` fields.
///
/// **ARCH-9 / TASK-1846: not a drift-safe entry point.** This discards
/// [`UpgradeParseDiagnostics`], so it bypasses [`check_missing_separator_drift`]
/// (TASK-1817), [`check_header_drift`] (TASK-1074) and
/// [`check_row_shape_drift`] (TASK-1202) — an unrecognised table silently
/// yields an empty `Vec` that reads as "no upgrades available". Production
/// code goes through [`interpret_upgrade_output`]; this is `#[cfg(test)]`-only
/// and exists so the column-slicing tests can drive the geometry directly.
#[cfg(test)]
#[must_use]
pub fn parse_upgrade_table(stdout: &str) -> Vec<UpgradeEntry> {
    parse_upgrade_table_inner(stdout).0
}

/// Diagnostics surfaced by [`parse_upgrade_table_inner`] so callers higher up
/// the stack (e.g. [`interpret_upgrade_output`]) can decide whether to bail
/// on suspected cargo-edit format drift instead of silently scoring the run
/// as "no upgrades available".
struct UpgradeParseDiagnostics {
    /// `true` once a `====` row aligned the column offsets.
    saw_separator: bool,
    /// `true` once a header line matched the recognised token shape
    /// (`name` + `old req` / `new req`, case-insensitive).
    saw_recognised_header: bool,
    /// Number of non-empty lines observed after the `====` separator.
    body_lines: usize,
    /// Number of non-empty, non-header, non-separator lines observed
    /// anywhere in the output. ERR-1 / TASK-1817:
    /// [`check_missing_separator_drift`] uses it to tell "cargo printed
    /// nothing at all" (legitimately zero upgrades) apart from "cargo printed
    /// a table we could not align because the separator row is gone".
    content_lines: usize,
    /// Number of rows successfully parsed into an `UpgradeEntry`.
    /// ERR-1 / TASK-1202: combined with `body_lines`, this lets
    /// [`interpret_upgrade_output`] fail closed when every body row was
    /// dropped by `parse_upgrade_row` — wholesale row-shape drift would
    /// otherwise return an empty Vec and look like "no upgrades available".
    entries_emitted: usize,
}

enum UpgradeLine {
    Header,
    Separator,
    Body,
}

fn classify_upgrade_line(line: &str) -> UpgradeLine {
    let trimmed = line.trim_start();
    if trimmed.starts_with("====") {
        return UpgradeLine::Separator;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("name") && (lower.contains("old req") || lower.contains("new req")) {
        return UpgradeLine::Header;
    }
    UpgradeLine::Body
}

fn parse_upgrade_table_inner(stdout: &str) -> (Vec<UpgradeEntry>, UpgradeParseDiagnostics) {
    let mut entries = Vec::new();
    let mut columns: Option<Vec<(usize, usize)>> = None;
    let mut saw_separator = false;
    let mut saw_recognised_header = false;
    let mut body_lines: usize = 0;
    let mut total_content_lines: usize = 0;

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match classify_upgrade_line(line) {
            UpgradeLine::Header if saw_recognised_header => {}
            UpgradeLine::Header => {
                columns = None;
                saw_recognised_header = true;
            }
            UpgradeLine::Separator => {
                columns = Some(separator_columns(line));
                saw_separator = true;
            }
            UpgradeLine::Body => {
                // Both counters take at most one increment per line of the
                // in-memory `stdout` string, whose length is bounded by
                // `isize::MAX`, so `saturating_add` equals `+= 1` exactly.
                total_content_lines = total_content_lines.saturating_add(1);
                if saw_separator {
                    body_lines = body_lines.saturating_add(1);
                    if let Some(cols) = columns.as_deref() {
                        if let Some(entry) = parse_upgrade_row(line, cols) {
                            entries.push(entry);
                        }
                    }
                }
            }
        }
    }

    if total_content_lines > 0 && !saw_separator {
        tracing::warn!("TASK-1026: cargo-upgrade stdout had body lines but no `====` separator — suspect format drift");
    }
    let diag = UpgradeParseDiagnostics {
        saw_separator,
        saw_recognised_header,
        body_lines,
        content_lines: total_content_lines,
        entries_emitted: entries.len(),
    };
    (entries, diag)
}

fn slice_column<'a>(line: &'a str, cols: &[(usize, usize)], idx: usize) -> Option<&'a str> {
    let &(start, end) = cols.get(idx)?;
    let (start, end) = clamp_to_char_boundaries(line, start, end)?;
    // `clamp_to_char_boundaries` already guarantees the range; `get` keeps
    // that a skipped column rather than a panic if it ever stops holding.
    let trimmed = line.get(start..end)?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn slice_fixed_columns<'a>(line: &'a str, cols: &[(usize, usize)]) -> Option<[&'a str; 5]> {
    Some([
        slice_column(line, cols, 0)?,
        slice_column(line, cols, 1)?,
        slice_column(line, cols, 2)?,
        slice_column(line, cols, 3)?,
        slice_column(line, cols, 4)?,
    ])
}

fn slice_note(line: &str, cols: &[(usize, usize)]) -> Option<String> {
    let &(start, _) = cols.get(5)?;
    let (start, end) = clamp_to_char_boundaries(line, start, line.len())?;
    // As in `slice_column`: the range is already clamped, so `get` only
    // degrades to "no note" if that invariant is ever broken.
    let trimmed = line.get(start..end)?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_upgrade_row(line: &str, cols: &[(usize, usize)]) -> Option<UpgradeEntry> {
    if cols.len() < 5 {
        tracing::debug!(
            column_count = cols.len(),
            "TASK-0404: skipping row — fewer than 5 columns"
        );
        return None;
    }
    let [name, old_req, compatible, latest, new_req] = slice_fixed_columns(line, cols).or_else(|| {
        tracing::debug!(line = %line, "TASK-0404: skipping row that did not fill 5 fixed columns");
        None
    })?;
    Some(UpgradeEntry {
        name: name.to_string(),
        old_req: old_req.to_string(),
        compatible: compatible.to_string(),
        latest: latest.to_string(),
        new_req: new_req.to_string(),
        note: slice_note(line, cols),
    })
}

fn clamp_to_char_boundaries(line: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let len = line.len();
    if start >= len {
        return None;
    }
    let clamped_end = end.min(len);
    let mut s = start;
    // Loop guard keeps `s < clamped_end <= line.len()`, so `saturating_add`
    // equals `+= 1` exactly.
    while s < clamped_end && !line.is_char_boundary(s) {
        s = s.saturating_add(1);
    }
    let mut e = clamped_end;
    // Loop guard keeps `e > s >= 0`, so `saturating_sub` equals `-= 1`
    // exactly.
    while e > s && !line.is_char_boundary(e) {
        e = e.saturating_sub(1);
    }
    if s >= e {
        return None;
    }
    if s != start || e != clamped_end {
        tracing::warn!(
            requested_start = start,
            requested_end = clamped_end,
            adjusted_start = s,
            adjusted_end = e,
            line = %line,
            "TASK-0960: cargo-upgrade row slice clamped to UTF-8 char boundaries (multi-byte content crossed a column edge)"
        );
    }
    Some((s, e))
}

/// Sentinel `end` for the final column: "read to the end of the *data* row".
/// [`clamp_to_char_boundaries`] applies `end.min(line.len())` against the row
/// being sliced, so this resolves per row rather than against the separator.
const COLUMN_END_OF_ROW: usize = usize::MAX;

/// Derive `(start, end)` byte ranges from a `====` separator row.
///
/// **The invariant this does *not* rely on.** cargo-edit sizes each `=` run
/// to its *header token's* length, not to the widest value in the column —
/// the crate's own fixtures prove it (`latest` is a 6-wide `======` above
/// 7-char `1.0.228`; `note` is a 4-wide `====` above `incompatible`). Every
/// interior column absorbs that: its `end` chains forward to the next
/// column's `start`, which sits past the over-wide value.
///
/// CL-3 / TASK-1836: the **final** fixed column has nothing to chain to. It
/// used to be given `end = line.len()` — the length of the *separator* row —
/// which clamped it to the header token's width and silently truncated any
/// wider value (`new req` `1.10.100` decoded as `1.10.10`). That was worse
/// than a dropped row: the row still filled all five columns, so
/// `parse_upgrade_row` returned `Some`, no drift guard fired, nothing was
/// logged, and `ops deps` printed a version that does not exist. The last
/// column now reads to the end of the data row, the same trick
/// [`slice_note`] already used for the note column.
fn separator_columns(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut cols = Vec::new();
    let mut i = 0;
    // Every increment is guarded by `i < bytes.len()`, so `i` never exceeds
    // `bytes.len() <= isize::MAX` and `saturating_add` equals `+= 1` exactly.
    while i < bytes.len() {
        if matches!(bytes.get(i), Some(b'=')) {
            let start = i;
            while matches!(bytes.get(i), Some(b'=')) {
                i = i.saturating_add(1);
            }
            cols.push((start, i));
        } else {
            i = i.saturating_add(1);
        }
    }
    cols.iter()
        .zip(
            cols.iter()
                .skip(1)
                .map(|c| c.0)
                .chain(std::iter::once(COLUMN_END_OF_ROW)),
        )
        .map(|(&(start, _), end)| (start, end))
        .collect()
}

/// PERF-3 / TASK-1112: case-insensitive ASCII substring scan that does not
/// allocate.
fn contains_ascii_ci(haystack: &str, needle: &str) -> bool {
    let n = needle.as_bytes();
    if n.is_empty() {
        return true;
    }
    let h = haystack.as_bytes();
    if h.len() < n.len() {
        return false;
    }
    h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
}

/// Split upgrade entries into compatible and incompatible.
pub fn categorize_upgrades(entries: Vec<UpgradeEntry>) -> UpgradeResult {
    let mut compatible = Vec::new();
    let mut incompatible = Vec::new();

    for entry in entries {
        let is_incompatible = entry
            .note
            .as_deref()
            .is_some_and(|n| contains_ascii_ci(n, "incompatible"));
        if is_incompatible {
            incompatible.push(entry);
        } else {
            compatible.push(entry);
        }
    }

    UpgradeResult {
        compatible,
        incompatible,
    }
}

#[cfg(test)]
mod exit_code_tests;
#[cfg(test)]
mod table_tests;
