//! Backlog config: the `backlog.config.yml` subset the commands need.
//!
//! Everything else in that file (ports, git behaviour, board rendering) is
//! skipped silently — this crate implements neither the features nor their
//! failure modes. Discovery is cwd-based with no upward walk, matching the
//! ops-core config precedent.

use std::path::Path;

/// The config subset, with the defaults backlog.md ships.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacklogConfig {
    /// Status column order for `task list` grouping.
    pub statuses: Vec<String>,
    /// Status a fresh task gets when `-s` is absent.
    pub default_status: String,
    /// Backlog directory relative to the workspace root.
    pub backlog_directory: String,
    /// Task id prefix (`TASK-0001`).
    pub task_prefix: String,
    /// Zero-padding width of the numeric id part.
    pub zero_padded_ids: usize,
}

impl Default for BacklogConfig {
    fn default() -> Self {
        Self {
            statuses: vec![
                "Triage".to_string(),
                "To Do".to_string(),
                "In Progress".to_string(),
                "Done".to_string(),
            ],
            default_status: "Triage".to_string(),
            backlog_directory: ".backlog".to_string(),
            task_prefix: "TASK".to_string(),
            zero_padded_ids: 4,
        }
    }
}

impl BacklogConfig {
    /// Load `<dir>/backlog.config.yml`, or the defaults when the file is
    /// absent.
    ///
    /// # Errors
    ///
    /// The file exists but a needed key carries a shape this parser cannot
    /// read; the error names the file (ERR-13).
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        let path = dir.join("backlog.config.yml");
        let Some(src) = std::fs::read_to_string(&path)
            .ok()
            .filter(|s| !s.trim().is_empty())
        else {
            return Ok(Self::default());
        };
        Self::parse(&src).map_err(|e| anyhow::anyhow!("parsing {}: {e:#}", path.display()))
    }

    /// Parse config text. Line-oriented like the task frontmatter parser:
    /// flat `key: value` entries plus one flow-list key (`statuses`).
    ///
    /// # Errors
    ///
    /// A needed key with an unreadable shape.
    pub fn parse(src: &str) -> anyhow::Result<Self> {
        let mut cfg = Self::default();
        for (idx, line) in src.lines().enumerate() {
            let line_no = idx.saturating_add(1);
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some((key, rest)) = trimmed.split_once(':') else {
                continue;
            };
            let rest = rest.trim();
            if rest.is_empty() {
                continue;
            }
            match key {
                "statuses" => {
                    let items = parse_flow_list(rest).ok_or_else(|| {
                        anyhow::anyhow!("line {line_no}: `statuses` is not a list")
                    })?;
                    if !items.is_empty() {
                        cfg.statuses = items;
                    }
                }
                "default_status" => cfg.default_status = unquote(rest),
                "backlog_directory" => cfg.backlog_directory = unquote(rest),
                "task_prefix" => cfg.task_prefix = unquote(rest),
                "zero_padded_ids" => {
                    cfg.zero_padded_ids = rest.parse::<usize>().map_err(|_| {
                        anyhow::anyhow!("line {line_no}: `zero_padded_ids` is not a number: {rest}")
                    })?;
                }
                // date_format and everything git/browser/board related are
                // read by nothing here; skip.
                _ => {}
            }
        }
        Ok(cfg)
    }
}

/// Split a YAML flow list `["a", "b"]` into its items; `None` when `src` is
/// not a bracketed list.
fn parse_flow_list(src: &str) -> Option<Vec<String>> {
    let inner = src.strip_prefix('[')?.strip_suffix(']')?;
    if inner.trim().is_empty() {
        return Some(Vec::new());
    }
    let mut items = Vec::new();
    for candidate in inner.split(',') {
        let item = unquote(candidate.trim());
        if item.is_empty() {
            return None;
        }
        items.push(item);
    }
    Some(items)
}

/// Strip matching surrounding quotes from a scalar, if present.
fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const REAL_CONFIG: &str = "\
project_name: \"ops\"
default_status: \"Triage\"
statuses: [\"Triage\", \"To Do\", \"In Progress\", \"Done\"]
labels: []
definition_of_done: []
date_format: yyyy-mm-dd
max_column_width: 20
auto_open_browser: true
default_port: 6420
remote_operations: true
auto_commit: false
zero_padded_ids: 4
bypass_git_hooks: false
check_active_branches: true
active_branch_days: 30
task_prefix: \"TASK\"
backlog_directory: \".backlog\"
";

    /// The live repo's own config parses into the values the commands use,
    /// skipping every key this crate does not implement.
    #[test]
    fn parses_the_real_repo_config() {
        let cfg = BacklogConfig::parse(REAL_CONFIG).expect("must parse");
        assert_eq!(cfg.statuses, vec!["Triage", "To Do", "In Progress", "Done"]);
        assert_eq!(cfg.default_status, "Triage");
        assert_eq!(cfg.backlog_directory, ".backlog");
        assert_eq!(cfg.task_prefix, "TASK");
        assert_eq!(cfg.zero_padded_ids, 4);
    }

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = BacklogConfig::load(dir.path()).expect("load");
        assert_eq!(cfg, BacklogConfig::default());
    }

    #[test]
    fn bad_zero_padding_is_an_error_naming_the_value() {
        let err = BacklogConfig::parse("zero_padded_ids: wide\n").expect_err("must fail");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("zero_padded_ids"),
            "error must name the key, got: {rendered}"
        );
    }
}
