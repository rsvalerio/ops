//! Parser for `cargo upgrade --dry-run` table output.

use crate::{UpgradeEntry, UpgradeResult};
use ops_core::subprocess::run_cargo;
use std::path::Path;
use std::time::Duration;

use super::truncate_for_log;

/// Default timeout for `cargo upgrade --dry-run`; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`.
const CARGO_UPGRADE_TIMEOUT: Duration = Duration::from_secs(180);

/// Run `cargo upgrade --dry-run` and parse the table output.
///
/// ERR-1 (TASK-0913): `cargo upgrade` exits non-zero on lockfile contention,
/// network failures, or a malformed `Cargo.toml`. The previous code parsed
/// stdout regardless of exit status and silently returned an empty
/// `Vec<UpgradeEntry>`, masking the upstream failure as "no upgrades
/// available". Surface non-zero exits as an error including the stderr
/// tail so the deps gate fails loudly. Mirrors the cargo-update fix made
/// in TASK-0502 and the cargo-deny exit-code handling below.
pub fn run_cargo_upgrade_dry_run(working_dir: &Path) -> anyhow::Result<Vec<UpgradeEntry>> {
    let output = run_cargo(
        &["upgrade", "--dry-run"],
        working_dir,
        CARGO_UPGRADE_TIMEOUT,
        "cargo upgrade --dry-run",
    )
    .map_err(|e| anyhow::anyhow!("failed to run cargo upgrade: {}", e))?;

    interpret_upgrade_output(output.status.code(), &output.stdout, &output.stderr)
}

/// Map a `cargo upgrade --dry-run` `(exit_code, stdout, stderr)` triple to
/// either a parsed `Vec<UpgradeEntry>` or an error including the stderr
/// tail. Split out so unit tests can pin the exit-code semantics without
/// spawning the binary.
pub fn interpret_upgrade_output(
    exit_code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
) -> anyhow::Result<Vec<UpgradeEntry>> {
    match exit_code {
        Some(0) => {
            let stdout = String::from_utf8_lossy(stdout);
            let (entries, diag) = parse_upgrade_table_inner(&stdout);
            // PATTERN-1 / TASK-1074: if cargo-edit's table prints a `====`
            // separator row but the header line above it does not match any
            // recognised token shape (renamed / localised columns), refuse to
            // score the run as "no upgrades available". The separator alone
            // would still calibrate column offsets and yield rows, but the
            // header drift is the canary for an upstream format change we
            // have not vetted — fail closed so operators see the drift
            // signal instead of silently downgrading the supply-chain gate.
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
            // ERR-1 / TASK-1202: a recognised header + separator with body
            // lines but zero parsed entries means every row was dropped by
            // `parse_upgrade_row` (column shape mismatch). Fail closed so a
            // wholesale row-shape drift surfaces as an error rather than a
            // silent empty Vec masquerading as "no upgrades available".
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
            Ok(entries)
        }
        None => anyhow::bail!(
            "cargo upgrade --dry-run terminated by signal (exit_code = None); \
             refusing to treat partial output as authoritative"
        ),
        Some(other) => {
            let stderr = String::from_utf8_lossy(stderr);
            // ERR-7 / SEC-21 / TASK-1160: format the stderr tail via Debug ({:?})
            // so embedded ANSI escapes / newlines / NULs from registry-served
            // diagnostics cannot forge log entries or repaint the operator
            // terminal. Mirrors the workspace policy enforced in probe.rs and
            // query.rs.
            anyhow::bail!(
                "cargo upgrade --dry-run exited with status {other}; \
                 refusing to parse output as authoritative. \
                 stderr (truncated): {:?}",
                truncate_for_log(stderr.trim())
            )
        }
    }
}

/// Parse the table output from `cargo upgrade --dry-run`.
///
/// SEC-15 / TASK-0383: column offsets are calibrated from the `====` separator
/// row rather than splitting on whitespace, so multi-word notes (e.g. "pinned
/// by parent") and any future column additions don't silently shift values
/// across `UpgradeEntry` fields.
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
    /// Number of non-empty, non-header, non-separator lines observed.
    body_lines: usize,
    /// Number of rows successfully parsed into an `UpgradeEntry`.
    /// ERR-1 / TASK-1202: combined with `body_lines`, this lets
    /// [`interpret_upgrade_output`] fail closed when every body row was
    /// dropped by `parse_upgrade_row` — wholesale row-shape drift would
    /// otherwise return an empty Vec and look like "no upgrades available".
    entries_emitted: usize,
}

fn parse_upgrade_table_inner(stdout: &str) -> (Vec<UpgradeEntry>, UpgradeParseDiagnostics) {
    let mut entries = Vec::new();
    let mut columns: Option<Vec<(usize, usize)>> = None;
    let mut saw_separator = false;
    let mut saw_recognised_header = false;
    let mut body_lines: usize = 0;

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // ERR-1 / TASK-1026: header detection must be case-insensitive and
        // tolerant of cargo-edit's column-name drift (e.g. `Name`, `Old Req`,
        // `New Req`). The cargo-edit table format is not a stable API, so a
        // capitalisation flip used to silently break detection — `columns`
        // stayed `None`, every data row was skipped, and the parser returned
        // an empty Vec, masquerading as "no upgrades available".
        let trimmed = line.trim_start();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("name") && (lower.contains("old req") || lower.contains("new req")) {
            // ERR-1 / TASK-1203: detect the header once. A future cargo-edit
            // sub-table header or a localised note row that happens to
            // contain the same tokens would otherwise re-arm `columns` to
            // None, dropping every subsequent body row until the next
            // `====` separator restores them. Keep the existing column
            // calibration in place after the first header detection.
            if saw_recognised_header {
                tracing::debug!(
                    line = %line,
                    "TASK-1203: ignoring repeat header-shaped line — column offsets retained"
                );
                continue;
            }
            columns = None;
            saw_recognised_header = true;
            continue;
        }

        // Separator row: `====   ======= ==========` defines exact byte columns.
        if trimmed.starts_with("====") {
            columns = Some(separator_columns(line));
            saw_separator = true;
            continue;
        }

        body_lines += 1;
        if let Some(cols) = columns.as_deref() {
            if let Some(entry) = parse_upgrade_row(line, cols) {
                entries.push(entry);
            }
        }
    }

    if body_lines > 0 && !saw_separator {
        tracing::warn!(
            "TASK-1026: cargo-upgrade stdout had body lines but no `====` separator row; \
             parse_upgrade_table is returning an empty result — suspect cargo-edit table-format drift"
        );
    }

    let entries_emitted = entries.len();
    (
        entries,
        UpgradeParseDiagnostics {
            saw_separator,
            saw_recognised_header,
            body_lines,
            entries_emitted,
        },
    )
}

fn parse_upgrade_row(line: &str, cols: &[(usize, usize)]) -> Option<UpgradeEntry> {
    if cols.len() < 5 {
        tracing::debug!(
            column_count = cols.len(),
            line = %line,
            "TASK-0404: skipping cargo-upgrade row — separator row had fewer than 5 columns"
        );
        return None;
    }

    let slice_col = |idx: usize| -> Option<&str> {
        let &(start, end) = cols.get(idx)?;
        let (start, end) = clamp_to_char_boundaries(line, start, end)?;
        let trimmed = line[start..end].trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    };

    let (Some(name), Some(old_req), Some(compatible), Some(latest), Some(new_req)) = (
        slice_col(0),
        slice_col(1),
        slice_col(2),
        slice_col(3),
        slice_col(4),
    ) else {
        tracing::debug!(
            line = %line,
            "TASK-0404: skipping cargo-upgrade row that did not fill the 5 fixed columns"
        );
        return None;
    };

    let note = cols.get(5).and_then(|(start, _)| {
        let (start, end) = clamp_to_char_boundaries(line, *start, line.len())?;
        let slice = line[start..end].trim();
        if slice.is_empty() {
            None
        } else {
            Some(slice.to_string())
        }
    });

    Some(UpgradeEntry {
        name: name.to_string(),
        old_req: old_req.to_string(),
        compatible: compatible.to_string(),
        latest: latest.to_string(),
        new_req: new_req.to_string(),
        note,
    })
}

fn clamp_to_char_boundaries(line: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let len = line.len();
    if start >= len {
        return None;
    }
    let clamped_end = end.min(len);
    let mut s = start;
    while s < clamped_end && !line.is_char_boundary(s) {
        s += 1;
    }
    let mut e = clamped_end;
    while e > s && !line.is_char_boundary(e) {
        e -= 1;
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

fn separator_columns(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut cols = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            let start = i;
            while i < bytes.len() && bytes[i] == b'=' {
                i += 1;
            }
            cols.push((start, i));
        } else {
            i += 1;
        }
    }
    for idx in 0..cols.len().saturating_sub(1) {
        cols[idx].1 = cols[idx + 1].0;
    }
    if let Some(last) = cols.last_mut() {
        last.1 = line.len();
    }
    cols
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
