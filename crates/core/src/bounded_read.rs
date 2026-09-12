//! The bounded read pipeline shared by the file-walking extensions
//! (`ops-text-fixers`, `ops-config-checkers`).
//!
//! DUP-2 / TASK-2162: this layer used to exist twice, near-verbatim, and the
//! copies had already diverged in the security-relevant direction — the
//! fixers' `open` used [`std::fs::symlink_metadata`] while the checkers' used
//! [`std::fs::metadata`], so a symlink was judged by itself in one crate and
//! by its target in the other. Both now share this one implementation, which
//! judges a symlink **as itself**: a repository-controlled link must never
//! put its target — which can live outside the run's root, or be a FIFO or
//! device that blocks `open(2)` — in front of a tool that reads and rewrites
//! files from a git hook.
//!
//! The vocabulary of *what happened instead of a read* lives here too
//! ([`SkipReason`], [`FailureKind`], [`FailedFile`]), along with the two
//! bookkeeping moves every consumer performs the same way
//! ([`record_failure`], [`report_walk_errors`]), so a hardening fix applied
//! once reaches every file-walking extension at once.
//!
//! # Why the cap is a property of the read
//!
//! `metadata()` describes a path at one instant and the file it described can
//! be replaced or extended before the read; `Read::take` bounds the read
//! itself, so the ceiling holds regardless. The handle's own metadata is also
//! what the fixers' atomic rewrite restores onto the new inode, which is why
//! [`read_candidate`] hands it back alongside the bytes.

use std::fs::{File, Metadata};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;

/// Default per-file size cap: 16 MiB.
///
/// Files larger than this are skipped and reported rather than read. The
/// consumers hold the whole file in memory (and the fixers allocate a second
/// buffer of the same size when a file actually changes — since TASK-2168
/// `fix_trailing` scans first and allocates only on a needed trim, so a
/// clean file, the steady state, stays at roughly 1x), so worst-case peak
/// resident memory is roughly twice the largest candidate — and the
/// candidate set is repository-controlled. A multi-gigabyte NUL-free file
/// (a CSV export, an ndjson dump, a `.sql` seed, a minified bundle) is
/// ordinary in a repo and would otherwise OOM-kill a `git commit`.
///
/// DUP-2 / TASK-2162: defined once here so `ops-text-fixers` and
/// `ops-config-checkers` cannot drift on what "too big to hold" means.
/// Nothing a whitespace fixer or config validator should be looking at comes
/// close to it.
pub const DEFAULT_MAX_BYTES: u64 = 16 * 1024 * 1024;

/// Why a discovered file was deliberately not examined.
///
/// A skip is a decision, not a malfunction: the file was reachable and the
/// consumer chose to leave it alone. Contrast [`FailureKind`], which means
/// the consumer wanted to look and could not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// Over the configured byte cap.
    TooLarge { len: u64, cap: u64 },
    /// A directory, device, FIFO, socket or symlink: never something to read
    /// or rewrite, and reading one can block forever or never reach EOF.
    NotRegularFile,
    /// Listed by discovery but absent when the consumer reached it — a staged
    /// deletion under `--tracked`, a sparse checkout, or a plain race.
    Vanished,
    /// Not text: contains a NUL byte or is not valid UTF-8. Produced by
    /// consumers that sniff content after the read (the text fixers);
    /// read-only consumers that select candidates by extension never
    /// construct it.
    NotText,
}

impl std::fmt::Display for SkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge { len, cap } => write!(f, "size {len} exceeds cap {cap}"),
            Self::NotRegularFile => f.write_str("not a regular file"),
            Self::Vanished => f.write_str("not present in the worktree"),
            Self::NotText => f.write_str("not text"),
        }
    }
}

/// Why a file could not be completed.
///
/// Every consumer uses `Metadata` and `Read`; `Write` is the fixers' added
/// "computed the fix but could not write it back", and `Parse` is the
/// validators' "the parser rejected the content". Each kind survives in the
/// type rather than in a message prefix because the CLI maps the report onto
/// exit codes whose documented meaning differs per kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// The handle's `metadata` failed (the open itself reports as
    /// [`FailureKind::Read`], or as a skip when the no-follow walk refused
    /// the entry).
    Metadata(ErrorKind),
    /// The file could not be opened or read.
    Read(ErrorKind),
    /// The rewrite was computed but could not be written back.
    Write(ErrorKind),
    /// The content was read but rejected (a parse failure or an exceeded
    /// checker bound).
    Parse,
}

/// A file a run could not complete.
#[derive(Debug, Clone)]
pub struct FailedFile {
    /// Path of the failed file, relative to the run's root directory where
    /// possible (for the validators that root is `CheckerOptions::root`;
    /// joining it onto the root resolves the file).
    pub path: PathBuf,
    /// Which stage failed — metadata lookup, read, write-back, or parse.
    pub kind: FailureKind,
    /// Human-readable failure detail, including the underlying I/O or
    /// parser error message.
    pub message: String,
}

/// What [`read_candidate`] found at a path instead of content.
#[derive(Debug)]
pub enum Rejected {
    /// Deliberately not examined.
    Skipped(SkipReason),
    /// I/O failure, with the line to render for it.
    Failed(FailureKind, String),
}

/// Bytes read in full within the cap, with the metadata of the handle they
/// came from — the same metadata a fixer's atomic rewrite restores onto the
/// new inode.
type Content = (Vec<u8>, Metadata);

/// Read one candidate file under a hard byte ceiling.
///
/// The symlink/type guards and the read bound are the two halves of the
/// security posture shared by every file-walking extension; see the module
/// docs for both.
///
/// # Errors
///
/// The `Err` payloads are not I/O errors but the caller's decisions, returned
/// by value so they cannot be silently dropped: [`Rejected::Skipped`] when
/// the path is deliberately out of scope (wrong type, over the cap, gone),
/// [`Rejected::Failed`] when the stat, open or read actually failed.
pub fn read_candidate(path: &Path, max_bytes: u64) -> Result<Content, Rejected> {
    let (file, metadata) = open_regular_file(path, max_bytes)?;
    read_bounded(file, metadata, max_bytes)
}

/// Open `path` if — and only if — it is a regular file within the cap.
///
/// The open delegates to [`crate::text::open_refusing_symlinks`]: one
/// descriptor-based walk that refuses a symlink at *any* component and
/// verifies the descriptor's own type with `fstat(2)` before any read, under
/// `O_NONBLOCK`. That closes the stat/open window the previous
/// `symlink_metadata` pre-check left open (the entry could be swapped between
/// the check and the `File::open`), and keeps a FIFO or device from wedging
/// the run inside `open(2)` — without a second no-follow implementation to
/// drift from the one the about/config layers already share.
fn open_regular_file(path: &Path, max_bytes: u64) -> Result<(File, Metadata), Rejected> {
    let file = match crate::text::open_refusing_symlinks(path) {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Err(Rejected::Skipped(SkipReason::Vanished));
        }
        // A refused symlink or non-regular entry (FIFO, socket, device, and
        // a directory where the platform open itself refuses it) surfaces as
        // `InvalidInput` with a stable message; on platforms where opening a
        // directory fails at `open(2)` it surfaces as `IsADirectory`. Both
        // are the deliberate out-of-scope decision, not a malfunction.
        Err(e) if e.kind() == ErrorKind::InvalidInput || e.kind() == ErrorKind::IsADirectory => {
            return Err(Rejected::Skipped(SkipReason::NotRegularFile));
        }
        Err(e) => return Err(read_failure(&e)),
    };
    // From the handle, so it describes the file that will actually be read
    // rather than whatever the path resolves to on a second lookup.
    let md = match file.metadata() {
        Ok(md) => md,
        Err(e) => return Err(metadata_failure(&e)),
    };
    if !md.is_file() {
        return Err(Rejected::Skipped(SkipReason::NotRegularFile));
    }
    if md.len() > max_bytes {
        return Err(Rejected::Skipped(SkipReason::TooLarge {
            len: md.len(),
            cap: max_bytes,
        }));
    }
    Ok((file, md))
}

/// Read `file` with the cap enforced by the reader itself.
fn read_bounded(file: File, metadata: Metadata, max_bytes: u64) -> Result<Content, Rejected> {
    // `max_bytes + 1` rather than `max_bytes`: reading one byte past the cap
    // is what makes an over-cap file *detectable* instead of silently
    // truncated and then rewritten/parsed as if it were the whole file —
    // which for a fixer that writes its input back would destroy everything
    // past the cap.
    let ceiling = max_bytes.saturating_add(1);
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len().min(max_bytes)).unwrap_or(0));
    if let Err(e) = file.take(ceiling).read_to_end(&mut bytes) {
        return Err(read_failure(&e));
    }
    // The stat above was a snapshot; the file may have grown since. This is
    // the check that holds, because it measures what was read.
    let read = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if read > max_bytes {
        return Err(Rejected::Skipped(SkipReason::TooLarge {
            len: read,
            cap: max_bytes,
        }));
    }
    Ok((bytes, metadata))
}

fn metadata_failure(e: &std::io::Error) -> Rejected {
    Rejected::Failed(FailureKind::Metadata(e.kind()), format!("metadata: {e}"))
}

fn read_failure(e: &std::io::Error) -> Rejected {
    Rejected::Failed(FailureKind::Read(e.kind()), format!("read: {e}"))
}

/// `path` relative to the run's `root` where possible, unchanged otherwise.
#[must_use]
pub fn relative_to(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

/// The report-side half of the shared pipeline: where per-file failures and
/// discovery walk errors land in a consumer's report.
///
/// Implemented by `FixerReport` (ops-text-fixers) and `CheckerReport`
/// (ops-config-checkers); the shared bookkeeping below is written against
/// this trait so the failure sources cannot drift apart in wording or
/// accounting between the two extensions.
pub trait FileRunReport {
    /// Record one file the run could not complete.
    fn push_failure(&mut self, failure: FailedFile);

    /// Adopt the discovery walk's traversal errors as the report's own.
    fn adopt_walk_errors(&mut self, errors: Vec<String>);
}

/// Render report-line text with newlines and terminal control characters
/// escaped, so a hostile filename (or an I/O error message embedding one)
/// cannot forge extra report lines or repaint the operator terminal. The
/// escaped forms stay readable (`\n`, not a replacement character).
fn safe_line_text(text: &str) -> String {
    if !text.chars().any(crate::text::is_unsafe_display_char) {
        return text.to_string();
    }
    text.chars()
        .map(|c| {
            if crate::text::is_unsafe_display_char(c) {
                // `{:?}` of a char yields `'\n'`; drop the quotes so the
                // escape blends into the surrounding text.
                format!("{c:?}").trim_matches('\'').to_string()
            } else {
                c.to_string()
            }
        })
        .collect()
}

/// Emit one failure line and record it, so the failure sources cannot drift
/// apart in either wording or bookkeeping.
///
/// The path is on the line and in the record. That is the whole point: the
/// predecessor propagated a bare `io::Error`, so a repository-wide run could
/// fail with `Permission denied (os error 13)` and nothing anywhere naming
/// which of thousands of files it meant.
///
/// Both the path and the message originate in the walked tree (an extension
/// root can be repo-supplied), so the rendered line goes through
/// [`safe_line_text`]; the `FailedFile` record keeps the raw path for
/// programmatic consumers (JSON, exit-code mapping), which do not interpret
/// control bytes.
///
/// # Errors
///
/// If `writer` fails while rendering the line.
pub fn record_failure<R: FileRunReport>(
    report: &mut R,
    writer: &mut dyn Write,
    label: &str,
    failure: FailedFile,
) -> anyhow::Result<()> {
    writeln!(
        writer,
        "{label}: {}: {}",
        safe_line_text(&failure.path.display().to_string()),
        safe_line_text(&failure.message)
    )
    .with_context(|| format!("{label}: writing failure line failed"))?;
    report.push_failure(failure);
    Ok(())
}

/// Print every discovery walk error and adopt it into the report.
///
/// An entry the walk could not traverse hides an unknown number of files, and
/// a gate that reports "clean" over them is fail-open. Printing the notice is
/// not enough — the errors have to reach the report so its `failed` verdict
/// can drive a non-zero exit.
///
/// # Errors
///
/// If `writer` fails while rendering a line.
pub fn report_walk_errors<R: FileRunReport>(
    report: &mut R,
    writer: &mut dyn Write,
    label: &str,
    errors: Vec<String>,
) -> anyhow::Result<()> {
    for error in &errors {
        writeln!(writer, "{label}: walk error: {error}")
            .with_context(|| format!("{label}: writing the walk-error notice failed"))?;
    }
    report.adopt_walk_errors(errors);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TASK-2162 AC #5, pinning the hardening config-checkers picked up:
    /// a symlink is judged by itself, never by its target. With
    /// `fs::metadata` this read would have *succeeded* and returned the
    /// target's bytes; the no-follow walk refuses the link itself.
    ///
    /// The fixture is created under the canonicalized tempdir root: the
    /// open now refuses symlinked components on the way to the file too
    /// (macOS tempdirs live behind the `/var` → `/private/var` symlink),
    /// and the test must exercise the *final-component* link, not that
    /// prefix refusal.
    #[cfg(unix)]
    #[test]
    fn a_symlink_is_never_judged_by_its_target() {
        let dir = tempfile::tempdir().unwrap();
        let root = crate::test_utils::canonical_root(&dir);
        let target = root.join("target.json");
        std::fs::write(&target, br#"{"a": 1}"#).unwrap();
        let link = root.join("link.json");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        match read_candidate(&link, DEFAULT_MAX_BYTES) {
            Err(Rejected::Skipped(SkipReason::NotRegularFile)) => {}
            other => panic!("a symlink must be a NotRegularFile skip, got {other:?}"),
        }
    }

    /// A symlink in a *directory component* of the path is refused by the
    /// shared no-follow walk, not just one at the final component — the
    /// `stat`-pre-check this module used before would have followed it.
    #[cfg(unix)]
    #[test]
    fn a_symlinked_directory_component_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let root = crate::test_utils::canonical_root(&dir);
        let real_dir = root.join("real");
        std::fs::create_dir(&real_dir).unwrap();
        std::fs::write(real_dir.join("a.txt"), b"hello\n").unwrap();
        std::os::unix::fs::symlink(&real_dir, root.join("alias")).unwrap();

        match read_candidate(&root.join("alias").join("a.txt"), DEFAULT_MAX_BYTES) {
            Err(Rejected::Skipped(SkipReason::NotRegularFile)) => {}
            other => panic!("a symlinked component must be refused, got {other:?}"),
        }
    }

    #[test]
    fn reads_a_regular_file_within_the_cap() {
        let dir = tempfile::tempdir().unwrap();
        let root = crate::test_utils::canonical_root(&dir);
        let p = root.join("a.txt");
        std::fs::write(&p, b"hello\n").unwrap();

        let (bytes, metadata) = match read_candidate(&p, DEFAULT_MAX_BYTES) {
            Ok(c) => c,
            other => panic!("expected a read, got {other:?}"),
        };
        assert_eq!(bytes, b"hello\n");
        assert!(metadata.is_file());
    }

    #[test]
    fn an_over_cap_file_is_skipped_by_the_read_itself() {
        let dir = tempfile::tempdir().unwrap();
        let root = crate::test_utils::canonical_root(&dir);
        let p = root.join("big.txt");
        std::fs::write(&p, b"trailing space   \n").unwrap();

        match read_candidate(&p, 4) {
            Err(Rejected::Skipped(SkipReason::TooLarge { len, cap })) => {
                assert_eq!(len, 18);
                assert_eq!(cap, 4);
            }
            other => panic!("expected a TooLarge skip, got {other:?}"),
        }
    }

    #[test]
    fn a_vanished_file_is_a_skip_not_a_failure() {
        let dir = tempfile::tempdir().unwrap();
        let root = crate::test_utils::canonical_root(&dir);
        match read_candidate(&root.join("gone.txt"), DEFAULT_MAX_BYTES) {
            Err(Rejected::Skipped(SkipReason::Vanished)) => {}
            other => panic!("expected a Vanished skip, got {other:?}"),
        }
    }

    /// A failure line must stay one line: a filename carrying a newline (or
    /// an ANSI escape) cannot forge additional report lines or repaint the
    /// terminal. The escapes stay readable, and clean text passes through
    /// byte-for-byte.
    #[test]
    fn record_failure_escapes_control_bytes_in_the_rendered_line() {
        struct Sink {
            failures: Vec<FailedFile>,
        }
        impl FileRunReport for Sink {
            fn push_failure(&mut self, failure: FailedFile) {
                self.failures.push(failure);
            }
            fn adopt_walk_errors(&mut self, _errors: Vec<String>) {}
        }

        let mut sink = Sink {
            failures: Vec::new(),
        };
        let mut line = Vec::new();
        let failure = FailedFile {
            path: std::path::PathBuf::from("evil\nINJECT.txt"),
            kind: FailureKind::Read(ErrorKind::PermissionDenied),
            message: "read: \u{1b}[31mdenied".to_string(),
        };
        record_failure(&mut sink, &mut line, "fix", failure).unwrap();
        let rendered = String::from_utf8(line).unwrap();
        assert_eq!(
            rendered,
            "fix: evil\\nINJECT.txt: read: \\u{1b}[31mdenied\n"
        );
        // The record keeps the raw path for programmatic consumers.
        assert_eq!(
            sink.failures[0].path,
            std::path::Path::new("evil\nINJECT.txt")
        );
        // Clean text is untouched.
        assert_eq!(safe_line_text("plain/path.txt"), "plain/path.txt");
    }

    /// The skip wording is user-facing (it lands on the per-file line), so
    /// pin it: a wording change must be deliberate and visible here.
    #[test]
    fn skip_reason_wording_is_pinned() {
        assert_eq!(
            SkipReason::TooLarge { len: 20, cap: 4 }.to_string(),
            "size 20 exceeds cap 4"
        );
        assert_eq!(SkipReason::NotRegularFile.to_string(), "not a regular file");
        assert_eq!(
            SkipReason::Vanished.to_string(),
            "not present in the worktree"
        );
        assert_eq!(SkipReason::NotText.to_string(), "not text");
    }
}
