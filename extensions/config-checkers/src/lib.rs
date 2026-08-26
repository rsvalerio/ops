//! `check-json` and `check-yaml` — generic-stack file validators modelled on
//! the same-named hooks from `pre-commit/pre-commit-hooks` (and mirrored by
//! `j178/prek`).
//!
//! Each checker walks the candidate file set (reusing the text-fixers'
//! discovery walk + git ls-files fast path), filters by extension, parses
//! each file, and reports a [`CheckerReport`] so the CLI can exit non-zero
//! when at least one file failed to parse. Files are never modified.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

pub mod json;
pub mod yaml;

use std::ffi::OsStr;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::Utf8Error;

use anyhow::Context;
use ops_extension::ExtensionType;

pub const NAME: &str = "config-checkers";
pub const DESCRIPTION: &str = "JSON and YAML parse-validators";
pub const SHORTNAME: &str = "config-checkers";

/// Default per-file size cap (16 MiB).
///
/// Files exceeding this are skipped and recorded in
/// [`CheckerReport::files_skipped`] rather than read into memory and parsed
/// — a defence against accidental or malicious oversized inputs triggering
/// an allocator/parser `DoS` on CI runners and pre-commit hosts.
pub const DEFAULT_MAX_BYTES: u64 = 16 * 1024 * 1024;

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

/// Typed parse error for [`json::check_json`] and [`yaml::check_yaml`].
#[derive(Debug)]
pub enum CheckError {
    /// File bytes were not valid UTF-8 (only meaningful for parsers that
    /// require a `&str`).
    InvalidUtf8(Utf8Error),
    /// Underlying parser rejected the input. The concrete parser error types
    /// (`serde_json::Error`, `json5::Error`, `saphyr::ScanError`) diverge, so
    /// the message is stored as a rendered string.
    Parse(String),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8(e) => write!(f, "invalid UTF-8: {e}"),
            Self::Parse(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for CheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidUtf8(e) => Some(e),
            Self::Parse(_) => None,
        }
    }
}

/// Options shared by both checkers.
#[derive(Debug, Clone)]
pub struct CheckerOptions {
    pub root: PathBuf,
    pub tracked_only: bool,
    /// JSON only: accept JSON5 (a strict superset of JSONC — comments and
    /// trailing commas, plus unquoted keys, single-quoted strings, hex
    /// numbers, etc.).
    pub allow_json5: bool,
    /// Per-file size cap. Files larger than this are skipped without being
    /// read or parsed.
    pub max_bytes: u64,
}

impl CheckerOptions {
    #[must_use]
    pub fn new(root: PathBuf, tracked_only: bool) -> Self {
        Self {
            root,
            tracked_only,
            allow_json5: false,
            max_bytes: DEFAULT_MAX_BYTES,
        }
    }

    #[must_use]
    pub fn with_allow_json5(mut self, allow: bool) -> Self {
        self.allow_json5 = allow;
        self
    }

    #[must_use]
    pub fn with_max_bytes(mut self, max_bytes: u64) -> Self {
        self.max_bytes = max_bytes;
        self
    }
}

/// A file that failed to parse during a checker run.
#[derive(Debug, Clone)]
pub struct FailedFile {
    pub path: PathBuf,
    pub message: String,
}

/// Outcome of a checker run.
#[derive(Debug, Default)]
pub struct CheckerReport {
    pub files_scanned: usize,
    pub files_failed: Vec<FailedFile>,
    /// Files skipped because they exceed [`CheckerOptions::max_bytes`].
    /// Counted separately from `files_scanned` so callers can distinguish
    /// "validated and OK" from "not validated at all".
    pub files_skipped: usize,
}

impl CheckerReport {
    #[must_use]
    pub fn failed(&self) -> bool {
        !self.files_failed.is_empty()
    }
}

/// Validate every `*.json` file under `opts.root`.
///
/// # Errors
/// Propagates discovery failures and writer I/O errors with the checker
/// label and root attached for diagnosis.
#[must_use = "the CheckerReport drives the process exit code; ignoring it defeats the validator"]
pub fn run_check_json(
    opts: &CheckerOptions,
    writer: &mut dyn Write,
) -> anyhow::Result<CheckerReport> {
    let allow_json5 = opts.allow_json5;
    run_checker(
        opts,
        writer,
        "check-json",
        |ext| matches_ext(ext, &["json"]),
        move |bytes| json::check_json(bytes, allow_json5),
    )
}

/// Validate every `*.yaml` / `*.yml` file under `opts.root`.
///
/// # Errors
/// Propagates discovery failures and writer I/O errors with the checker
/// label and root attached for diagnosis.
#[must_use = "the CheckerReport drives the process exit code; ignoring it defeats the validator"]
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
        .is_some_and(|e| allowed.iter().any(|a| a.eq_ignore_ascii_case(e)))
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
    C: Fn(&[u8]) -> Result<(), CheckError>,
{
    let files =
        ops_text_fixers::discovery::discover(&opts.root, opts.tracked_only).with_context(|| {
            format!(
                "{label}: file discovery failed for root {} (tracked_only={})",
                opts.root.display(),
                opts.tracked_only
            )
        })?;
    let mut report = CheckerReport::default();

    for path in files {
        if !ext_ok(path.extension()) {
            continue;
        }
        let display = relative_to(&path, &opts.root);

        // Size gate before reading — avoids loading a multi-GB file into
        // memory just to discover the parser would OOM on it.
        match std::fs::metadata(&path) {
            Ok(md) if md.len() > opts.max_bytes => {
                report.files_skipped += 1;
                writeln!(
                    writer,
                    "{label}: {}: skipped (size {} exceeds cap {})",
                    display.display(),
                    md.len(),
                    opts.max_bytes
                )
                .with_context(|| format!("{label}: writing skip notice failed"))?;
                continue;
            }
            Ok(_) => {}
            Err(e) => {
                report.files_scanned += 1;
                let msg = format!("metadata: {e}");
                writeln!(writer, "{label}: {}: {msg}", display.display())
                    .with_context(|| format!("{label}: writing failure line failed"))?;
                report.files_failed.push(FailedFile {
                    path: display,
                    message: msg,
                });
                continue;
            }
        }

        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                report.files_scanned += 1;
                let msg = format!("read: {e}");
                writeln!(writer, "{label}: {}: {msg}", display.display())
                    .with_context(|| format!("{label}: writing failure line failed"))?;
                report.files_failed.push(FailedFile {
                    path: display,
                    message: msg,
                });
                continue;
            }
        };
        report.files_scanned += 1;
        if let Err(err) = check(&bytes) {
            let msg = err.to_string();
            writeln!(writer, "{label}: {}: {msg}", display.display())
                .with_context(|| format!("{label}: writing failure line failed"))?;
            report.files_failed.push(FailedFile {
                path: display,
                message: msg,
            });
        }
    }

    Ok(report)
}

fn relative_to(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

/// One-line summary for the CLI.
///
/// # Errors
/// Propagates writer I/O errors so a broken pipe in CI is not silently
/// hidden from the caller.
pub fn write_summary(
    report: &CheckerReport,
    label: &str,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{label}: scanned {} file(s), {} failed, {} skipped",
        report.files_scanned,
        report.files_failed.len(),
        report.files_skipped,
    )
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
        assert_eq!(report.files_failed[0].path, PathBuf::from("bad.json"));
    }

    #[test]
    fn check_json_json5_flag_accepts_comments_and_unquoted_keys() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("c.json"), br#"{ /* x */ "a": 1, }"#);

        let strict = CheckerOptions::new(root.to_path_buf(), false);
        let mut buf = Vec::new();
        assert!(run_check_json(&strict, &mut buf).unwrap().failed());

        let lenient = CheckerOptions::new(root.to_path_buf(), false).with_allow_json5(true);
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
        assert_eq!(report.files_failed[0].path, PathBuf::from("bad.yaml"));
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

    #[test]
    fn oversized_files_are_skipped_not_parsed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // A file that would fail strict JSON parse, but exceeds a tiny cap
        // — must be reported as skipped, never reach the parser.
        write(&root.join("huge.json"), b"not valid json at all");

        let opts = CheckerOptions::new(root.to_path_buf(), false).with_max_bytes(4);
        let mut buf = Vec::new();
        let report = run_check_json(&opts, &mut buf).unwrap();

        assert_eq!(report.files_scanned, 0);
        assert_eq!(report.files_skipped, 1);
        assert!(report.files_failed.is_empty());
        assert!(!report.failed());
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("skipped"), "out was {out:?}");
    }

    #[test]
    #[cfg(unix)]
    fn unreadable_file_is_reported_as_failed() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let p = root.join("locked.json");
        write(&p, br#"{"a": 1}"#);
        // Root bypasses permission bits; skip the assertion in that case.
        // SAFETY: `libc::geteuid` is always safe to call.
        let is_root = unsafe { libc::geteuid() } == 0;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o000)).unwrap();

        let opts = CheckerOptions::new(root.to_path_buf(), false);
        let mut buf = Vec::new();
        let report = run_check_json(&opts, &mut buf).unwrap();

        // Restore so tempdir cleanup can remove it.
        let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644));

        if is_root {
            return;
        }
        assert_eq!(report.files_failed.len(), 1);
        assert_eq!(report.files_failed[0].path, PathBuf::from("locked.json"));
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("locked.json"), "out was {out:?}");
    }

    /// A `Write` that fails on the first byte — used to assert writer
    /// errors surface from `run_checker` instead of being dropped.
    struct FailingWriter;
    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("induced write failure"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn writer_errors_propagate() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("bad.json"), br#"{"a": }"#);

        let opts = CheckerOptions::new(root.to_path_buf(), false);
        let mut w = FailingWriter;
        let err = run_check_json(&opts, &mut w).unwrap_err();
        assert!(
            err.to_string().contains("check-json"),
            "context missing: {err}"
        );
    }

    // Note: `discovery::discover` currently swallows filesystem errors and
    // returns an empty list, so the `with_context` wrap on its `?` site is
    // a defensive future-proof — there is no externally-reachable failure
    // mode to assert against today.
}
