//! `trailing-whitespace` and `end-of-file-fixer` — generic-stack text fixers
//! modelled on the same-named hooks from `pre-commit/pre-commit-hooks`.
//!
//! Each fixer walks the candidate file set, skips binaries, rewrites files
//! in place when changes are needed, and reports a [`FixerReport`] so the
//! CLI can exit non-zero when at least one file was modified (matches the
//! pre-commit contract that fails a commit on change).

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

pub mod binary;
pub mod discovery;
pub mod eof;
pub mod trailing;

use std::io::Write;
use std::path::PathBuf;

use ops_extension::ExtensionType;

pub const NAME: &str = "text-fixers";
pub const DESCRIPTION: &str = "Trailing whitespace and end-of-file fixers";
pub const SHORTNAME: &str = "text-fixers";

pub struct TextFixersExtension;

ops_extension::impl_extension! {
    TextFixersExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::COMMAND,
    command_names: &["trailing-whitespace", "end-of-file-fixer"],
    data_provider_name: None,
    register_commands: |_self, registry| {
        registry.insert(
            "trailing-whitespace".into(),
            ops_core::config::CommandSpec::Exec(
                ops_core::config::ExecCommandSpec::new("ops", ["trailing-whitespace"]),
            ),
        );
        registry.insert(
            "end-of-file-fixer".into(),
            ops_core::config::CommandSpec::Exec(
                ops_core::config::ExecCommandSpec::new("ops", ["end-of-file-fixer"]),
            ),
        );
    },
    register_data_providers: |_self, _registry| {},
    factory: TEXT_FIXERS_FACTORY = |_, _| {
        Some((NAME, Box::new(TextFixersExtension)))
    },
}

/// Options for both fixers.
#[derive(Debug, Clone)]
pub struct FixerOptions {
    pub root: PathBuf,
    pub tracked_only: bool,
}

impl FixerOptions {
    #[must_use]
    pub const fn new(root: PathBuf, tracked_only: bool) -> Self {
        Self { root, tracked_only }
    }
}

/// Outcome of a fixer run.
#[derive(Debug, Default)]
pub struct FixerReport {
    pub files_scanned: usize,
    pub files_changed: Vec<PathBuf>,
}

impl FixerReport {
    #[must_use]
    pub const fn changed(&self) -> bool {
        !self.files_changed.is_empty()
    }
}

/// Strip trailing whitespace from every text file under `opts.root`.
///
/// # Errors
///
/// If the candidate file set cannot be discovered (including a failing
/// `git ls-files` when `tracked_only` is set), or if a file that needs
/// fixing cannot be written back.
pub fn run_trailing_whitespace(
    opts: &FixerOptions,
    writer: &mut dyn Write,
) -> anyhow::Result<FixerReport> {
    run_fixer(opts, writer, "trailing-whitespace", trailing::fix_trailing)
}

/// Ensure every text file under `opts.root` ends with exactly one newline.
///
/// # Errors
///
/// If the candidate file set cannot be discovered (including a failing
/// `git ls-files` when `tracked_only` is set), or if a file that needs
/// fixing cannot be written back.
pub fn run_end_of_file_fixer(
    opts: &FixerOptions,
    writer: &mut dyn Write,
) -> anyhow::Result<FixerReport> {
    run_fixer(opts, writer, "end-of-file-fixer", eof::fix_eof)
}

fn run_fixer(
    opts: &FixerOptions,
    writer: &mut dyn Write,
    label: &str,
    fix: fn(&[u8]) -> Option<Vec<u8>>,
) -> anyhow::Result<FixerReport> {
    let files = discovery::discover(&opts.root, opts.tracked_only)?;
    let mut report = FixerReport::default();

    for path in files {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        report.files_scanned += 1;

        if binary::is_probably_binary(&bytes) {
            continue;
        }
        let Some(fixed) = fix(&bytes) else {
            continue;
        };
        if fixed == bytes {
            continue;
        }
        std::fs::write(&path, &fixed)?;
        let display = path.strip_prefix(&opts.root).unwrap_or(&path).to_path_buf();
        writeln!(writer, "{label}: fixed {}", display.display()).ok();
        report.files_changed.push(display);
    }

    Ok(report)
}

/// Convenience used by the CLI: write a one-line summary to `writer`.
pub fn write_summary(report: &FixerReport, label: &str, writer: &mut dyn Write) {
    let _ = writeln!(
        writer,
        "{label}: scanned {} file(s), {} changed",
        report.files_scanned,
        report.files_changed.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write(p: &Path, content: &[u8]) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn trailing_whitespace_rewrites_dirty_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("a.txt"), b"hello   \nworld\n");
        write(&root.join("clean.txt"), b"clean\n");
        write(&root.join("bin.dat"), b"hello\0world   ");

        let opts = FixerOptions::new(root.to_path_buf(), false);
        let mut buf = Vec::new();
        let report = run_trailing_whitespace(&opts, &mut buf).unwrap();

        assert_eq!(report.files_changed.len(), 1);
        assert_eq!(report.files_changed[0], PathBuf::from("a.txt"));
        assert_eq!(
            std::fs::read(root.join("a.txt")).unwrap(),
            b"hello\nworld\n"
        );
        assert_eq!(std::fs::read(root.join("clean.txt")).unwrap(), b"clean\n");
        assert_eq!(
            std::fs::read(root.join("bin.dat")).unwrap(),
            b"hello\0world   ",
            "binary file must be left alone"
        );
    }

    #[test]
    fn eof_fixer_adds_missing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("a.txt"), b"hello");
        write(&root.join("b.txt"), b"hello\n");
        write(&root.join("c.txt"), b"hello\n\n\n");

        let opts = FixerOptions::new(root.to_path_buf(), false);
        let mut buf = Vec::new();
        let report = run_end_of_file_fixer(&opts, &mut buf).unwrap();

        assert_eq!(report.files_changed.len(), 2);
        assert_eq!(std::fs::read(root.join("a.txt")).unwrap(), b"hello\n");
        assert_eq!(std::fs::read(root.join("b.txt")).unwrap(), b"hello\n");
        assert_eq!(std::fs::read(root.join("c.txt")).unwrap(), b"hello\n");
    }

    #[test]
    fn second_run_is_clean() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(&root.join("a.txt"), b"hi   \nbye");

        let opts = FixerOptions::new(root.to_path_buf(), false);
        let mut buf = Vec::new();
        run_trailing_whitespace(&opts, &mut buf).unwrap();
        run_end_of_file_fixer(&opts, &mut buf).unwrap();

        let mut buf = Vec::new();
        let r1 = run_trailing_whitespace(&opts, &mut buf).unwrap();
        let r2 = run_end_of_file_fixer(&opts, &mut buf).unwrap();
        assert!(!r1.changed());
        assert!(!r2.changed());
    }

    #[test]
    fn extension_constants_kebab_case() {
        for n in ["trailing-whitespace", "end-of-file-fixer", NAME, SHORTNAME] {
            assert!(
                n.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "must be kebab-case: {n}"
            );
        }
    }
}
