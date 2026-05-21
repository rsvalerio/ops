//! `check-json` and `check-yaml` — generic-stack file validators modelled on
//! the same-named hooks from `pre-commit/pre-commit-hooks` (and mirrored by
//! `j178/prek`).
//!
//! Each checker walks the candidate file set (reusing the text-fixers'
//! discovery walk + git ls-files fast path), filters by extension, parses
//! each file, and reports a [`CheckerReport`] so the CLI can exit non-zero
//! when at least one file failed to parse. Files are never modified.

pub mod json;
pub mod yaml;

use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};

use ops_extension::ExtensionType;

pub const NAME: &str = "config-checkers";
pub const DESCRIPTION: &str = "JSON and YAML parse-validators";
pub const SHORTNAME: &str = "config-checkers";

pub struct ConfigCheckersExtension;

ops_extension::impl_extension! {
    ConfigCheckersExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::COMMAND,
    command_names: &["check-json", "check-yaml"],
    data_provider_name: None,
    register_commands: |_self, registry| {
        registry.insert(
            "check-json".into(),
            ops_core::config::CommandSpec::Exec(
                ops_core::config::ExecCommandSpec::new("ops", ["check-json"]),
            ),
        );
        registry.insert(
            "check-yaml".into(),
            ops_core::config::CommandSpec::Exec(
                ops_core::config::ExecCommandSpec::new("ops", ["check-yaml"]),
            ),
        );
    },
    register_data_providers: |_self, _registry| {},
    factory: CONFIG_CHECKERS_FACTORY = |_, _| {
        Some((NAME, Box::new(ConfigCheckersExtension)))
    },
}

/// Options shared by both checkers.
#[derive(Debug, Clone)]
pub struct CheckerOptions {
    pub root: PathBuf,
    pub tracked_only: bool,
    /// JSON only: accept comments and trailing commas (JSONC).
    pub allow_jsonc: bool,
}

impl CheckerOptions {
    pub fn new(root: PathBuf, tracked_only: bool) -> Self {
        Self {
            root,
            tracked_only,
            allow_jsonc: false,
        }
    }

    #[must_use]
    pub fn with_allow_jsonc(mut self, allow: bool) -> Self {
        self.allow_jsonc = allow;
        self
    }
}

/// Outcome of a checker run.
#[derive(Debug, Default)]
pub struct CheckerReport {
    pub files_scanned: usize,
    pub files_failed: Vec<(PathBuf, String)>,
}

impl CheckerReport {
    pub fn failed(&self) -> bool {
        !self.files_failed.is_empty()
    }
}

/// Validate every `*.json` file under `opts.root`.
pub fn run_check_json(
    opts: &CheckerOptions,
    writer: &mut dyn Write,
) -> anyhow::Result<CheckerReport> {
    let allow_jsonc = opts.allow_jsonc;
    run_checker(
        opts,
        writer,
        "check-json",
        |ext| matches_ext(ext, &["json"]),
        move |bytes| json::check_json(bytes, allow_jsonc),
    )
}

/// Validate every `*.yaml` / `*.yml` file under `opts.root`.
pub fn run_check_yaml(
    opts: &CheckerOptions,
    writer: &mut dyn Write,
) -> anyhow::Result<CheckerReport> {
    run_checker(
        opts,
        writer,
        "check-yaml",
        |ext| matches_ext(ext, &["yaml", "yml"]),
        yaml::check_yaml,
    )
}

fn matches_ext(ext: Option<&OsStr>, allowed: &[&str]) -> bool {
    ext.and_then(OsStr::to_str)
        .map(|e| {
            let lower = e.to_ascii_lowercase();
            allowed.iter().any(|a| *a == lower)
        })
        .unwrap_or(false)
}

fn run_checker<E, C>(
    opts: &CheckerOptions,
    writer: &mut dyn Write,
    label: &str,
    ext_ok: E,
    check: C,
) -> anyhow::Result<CheckerReport>
where
    E: Fn(Option<&OsStr>) -> bool,
    C: Fn(&[u8]) -> Result<(), String>,
{
    let files = ops_text_fixers::discovery::discover(&opts.root, opts.tracked_only)?;
    let mut report = CheckerReport::default();

    for path in files {
        if !ext_ok(path.extension()) {
            continue;
        }
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        report.files_scanned += 1;
        if let Err(msg) = check(&bytes) {
            let display = relative_to(&path, &opts.root);
            writeln!(writer, "{label}: {}: {msg}", display.display()).ok();
            report.files_failed.push((display, msg));
        }
    }

    Ok(report)
}

fn relative_to(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

/// One-line summary for the CLI.
pub fn write_summary(report: &CheckerReport, label: &str, writer: &mut dyn Write) {
    let _ = writeln!(
        writer,
        "{label}: scanned {} file(s), {} failed",
        report.files_scanned,
        report.files_failed.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, content: &[u8]) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn check_json_flags_only_broken_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("ok.json"), br#"{"a": 1}"#);
        write(&root.join("bad.json"), br#"{"a": }"#);
        write(&root.join("note.txt"), br#"{"a": }"#); // wrong ext: ignored

        let opts = CheckerOptions::new(root.to_path_buf(), false);
        let mut buf = Vec::new();
        let report = run_check_json(&opts, &mut buf).unwrap();

        assert_eq!(report.files_scanned, 2);
        assert_eq!(report.files_failed.len(), 1);
        assert_eq!(report.files_failed[0].0, PathBuf::from("bad.json"));
    }

    #[test]
    fn check_json_jsonc_flag_accepts_comments() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("c.json"), br#"{ /* x */ "a": 1, }"#);

        let strict = CheckerOptions::new(root.to_path_buf(), false);
        let mut buf = Vec::new();
        assert!(run_check_json(&strict, &mut buf).unwrap().failed());

        let lenient = CheckerOptions::new(root.to_path_buf(), false).with_allow_jsonc(true);
        let mut buf = Vec::new();
        assert!(!run_check_json(&lenient, &mut buf).unwrap().failed());
    }

    #[test]
    fn check_yaml_flags_only_broken_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("ok.yaml"), b"a: 1\nb: 2\n");
        write(&root.join("multi.yml"), b"a: 1\n---\nb: 2\n");
        write(&root.join("bad.yaml"), b"a: : :\n");

        let opts = CheckerOptions::new(root.to_path_buf(), false);
        let mut buf = Vec::new();
        let report = run_check_yaml(&opts, &mut buf).unwrap();

        assert_eq!(report.files_scanned, 3);
        assert_eq!(report.files_failed.len(), 1);
        assert_eq!(report.files_failed[0].0, PathBuf::from("bad.yaml"));
    }

    #[test]
    fn extension_matching_is_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("UPPER.JSON"), br#"{"ok": true}"#);

        let opts = CheckerOptions::new(root.to_path_buf(), false);
        let mut buf = Vec::new();
        let report = run_check_json(&opts, &mut buf).unwrap();
        assert_eq!(report.files_scanned, 1);
    }

    #[test]
    fn extension_constants_kebab_case() {
        for n in ["check-json", "check-yaml", NAME, SHORTNAME] {
            assert!(
                n.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "must be kebab-case: {n}"
            );
        }
    }
}
