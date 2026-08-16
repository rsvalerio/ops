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
    .map_err(|e| anyhow::anyhow!("failed to run cargo upgrade: {e}"))?;

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
                total_content_lines += 1;
                if saw_separator {
                    body_lines += 1;
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
        entries_emitted: entries.len(),
    };
    (entries, diag)
}

fn slice_column<'a>(line: &'a str, cols: &[(usize, usize)], idx: usize) -> Option<&'a str> {
    let &(start, end) = cols.get(idx)?;
    let (start, end) = clamp_to_char_boundaries(line, start, end)?;
    let trimmed = line[start..end].trim();
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
    let trimmed = line[start..end].trim();
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
    let len = line.len();
    cols.iter()
        .zip(cols.iter().skip(1).map(|c| c.0).chain(std::iter::once(len)))
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
mod tests {
    use super::*;

    #[test]
    fn classify_header() {
        assert!(matches!(
            classify_upgrade_line("name   old req compatible latest  new req"),
            UpgradeLine::Header
        ));
        assert!(matches!(
            classify_upgrade_line("Name   Old Req Compatible Latest  New Req"),
            UpgradeLine::Header
        ));
    }

    #[test]
    fn classify_separator() {
        assert!(matches!(
            classify_upgrade_line("====   ======= ========== ======  ======="),
            UpgradeLine::Separator
        ));
    }

    #[test]
    fn classify_body() {
        assert!(matches!(
            classify_upgrade_line("serde  1.0.100 1.0.228    1.0.228 1.0.228"),
            UpgradeLine::Body
        ));
        assert!(matches!(
            classify_upgrade_line("Updating crates.io index"),
            UpgradeLine::Body
        ));
    }
}
