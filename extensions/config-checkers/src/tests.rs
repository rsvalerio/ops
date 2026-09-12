//! Tests for the checking engine and the report it produces.
//!
//! The parser-level cases (depth caps, alias budgets, error variants) live
//! next to the parsers in `json.rs` and `yaml.rs`; what is covered here is
//! everything the engine decides — which files are read at all, how each
//! outcome is counted, and what the user-facing lines say.

use std::io::Write;
use std::path::{Path, PathBuf};

use super::{
    run_check_json, run_check_yaml, write_summary, CheckerOptions, CheckerReport, FailedFile,
    FailureKind, NAME, SHORTNAME,
};

/// Resolve a tempdir root through its symlinked prefix (macOS: `/var` →
/// `/private/var`), per the caller-canonicalizes-once rule
/// `ops_core::text::open_refusing_symlinks` documents: the shared bounded
/// read refuses symlinked directory components, so a raw tempdir path would
/// be refused for the prefix, not for anything the fixture set up.
fn canon(dir: &tempfile::TempDir) -> std::path::PathBuf {
    dir.path().canonicalize().unwrap()
}

fn write(p: &Path, content: &[u8]) {
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, content).unwrap();
}

/// Stage everything under `root` in a fresh git repo.
///
/// TEST-26 / TASK-2126: `false` now means exactly one thing — this
/// environment has no git binary, and the skip has already been surfaced on
/// stderr, so a caller's `return` is visible in the test output instead of a
/// vacuous pass. A git that *runs* and fails (spawn error, non-zero exit)
/// panics here: a present-but-refusing git is a broken fixture, not a
/// missing capability, and bailing out over it would delete the tracked-mode
/// coverage this suite exists to hold.
fn stage_all(root: &Path) -> bool {
    for args in [["init", "--quiet"], ["add", "-A"]] {
        if let Err(e) = ops_core::test_utils::git_fixture(root, &args) {
            match e {
                ops_core::test_utils::GitFixtureError::BinaryAbsent => {
                    ops_core::test_utils::skip_precondition(
                        "git fixture",
                        "git is not on PATH; tracked-mode assertions did not run",
                    );
                    return false;
                }
                ops_core::test_utils::GitFixtureError::CommandFailed { .. } => {
                    panic!("git fixture broke: {e}");
                }
            }
        }
    }
    true
}

#[test]
fn check_json_flags_only_broken_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = &canon(&dir);
    write(&root.join("ok.json"), br#"{"a": 1}"#);
    write(&root.join("bad.json"), br#"{"a": }"#);
    write(&root.join("note.txt"), br#"{"a": }"#); // wrong ext: ignored

    let opts = CheckerOptions::new(root.clone(), false);
    let mut buf = Vec::new();
    let report = run_check_json(&opts, &mut buf).unwrap();

    assert_eq!(report.files_scanned, 2);
    assert_eq!(report.files_failed.len(), 1);
    assert_eq!(report.files_failed[0].path, PathBuf::from("bad.json"));
}

#[test]
fn check_json_json5_flag_accepts_comments_and_unquoted_keys() {
    let dir = tempfile::tempdir().unwrap();
    let root = &canon(&dir);
    write(&root.join("c.json"), br#"{ /* x */ "a": 1, }"#);

    let strict = CheckerOptions::new(root.clone(), false);
    let mut buf = Vec::new();
    assert!(run_check_json(&strict, &mut buf).unwrap().failed());

    let lenient = CheckerOptions::new(root.clone(), false).with_allow_json5(true);
    let mut buf = Vec::new();
    assert!(!run_check_json(&lenient, &mut buf).unwrap().failed());
}

#[test]
fn check_yaml_flags_only_broken_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = &canon(&dir);
    write(&root.join("ok.yaml"), b"a: 1\nb: 2\n");
    write(&root.join("multi.yml"), b"a: 1\n---\nb: 2\n");
    write(&root.join("bad.yaml"), b"a: : :\n");

    let opts = CheckerOptions::new(root.clone(), false);
    let mut buf = Vec::new();
    let report = run_check_yaml(&opts, &mut buf).unwrap();

    assert_eq!(report.files_scanned, 3);
    assert_eq!(report.files_failed.len(), 1);
    assert_eq!(report.files_failed[0].path, PathBuf::from("bad.yaml"));
}

#[test]
fn extension_matching_is_case_insensitive() {
    let dir = tempfile::tempdir().unwrap();
    let root = &canon(&dir);
    write(&root.join("UPPER.JSON"), br#"{"ok": true}"#);

    let opts = CheckerOptions::new(root.clone(), false);
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
    let root = &canon(&dir);
    // A file that would fail strict JSON parse, but exceeds a tiny cap
    // — must be reported as skipped, never reach the parser.
    write(&root.join("huge.json"), b"not valid json at all");

    let opts = CheckerOptions::new(root.clone(), false).with_max_bytes(4);
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
fn parse_failures_are_recorded_with_the_parse_kind() {
    let dir = tempfile::tempdir().unwrap();
    let root = &canon(&dir);
    write(&root.join("bad.json"), br#"{"a": }"#);

    let opts = CheckerOptions::new(root.clone(), false);
    let mut buf = Vec::new();
    let report = run_check_json(&opts, &mut buf).unwrap();

    assert_eq!(report.files_failed.len(), 1);
    assert_eq!(report.files_failed[0].kind, FailureKind::Parse);
    assert_eq!(
        report.files_failed[0].message,
        "expected value at line 1 column 7"
    );
}

#[test]
#[cfg(unix)]
fn unreadable_file_is_reported_as_a_read_failure_not_a_parse_failure() {
    use std::os::unix::fs::PermissionsExt;

    // Root bypasses the permission bits entirely, so the assertion below
    // would invert rather than fail — the guard is mandatory, not cosmetic.
    // TEST-26 / TASK-2126: surfaced, so a root-container run that never
    // executes these assertions is distinguishable from one that did.
    if ops_core::test_utils::is_root_euid() {
        ops_core::test_utils::skip_precondition(
            "unreadable-file fixture",
            "running as root bypasses DAC; the read-failure assertions did not run",
        );
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let root = &canon(&dir);
    let p = root.join("locked.json");
    write(&p, br#"{"a": 1}"#);
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o000)).unwrap();

    let opts = CheckerOptions::new(root.clone(), false);
    let mut buf = Vec::new();
    let report = run_check_json(&opts, &mut buf).unwrap();

    // Restore so tempdir cleanup can remove it.
    let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644));

    assert_eq!(report.files_failed.len(), 1);
    assert_eq!(report.files_failed[0].path, PathBuf::from("locked.json"));
    assert_eq!(
        report.files_failed[0].kind,
        FailureKind::Read(std::io::ErrorKind::PermissionDenied)
    );
    // The file was never read, so it was never scanned.
    assert_eq!(report.files_scanned, 0);
    let out = String::from_utf8(buf).unwrap();
    assert!(out.contains("locked.json"), "out was {out:?}");
}

#[test]
fn tracked_only_validates_the_files_git_lists() {
    let dir = tempfile::tempdir().unwrap();
    let root = &canon(&dir);
    write(&root.join("ok.json"), br#"{"a": 1}"#);
    write(&root.join("bad.json"), br#"{"a": }"#);
    if !stage_all(root) {
        return; // no git: `discover` would fall back to the walk
    }

    let opts = CheckerOptions::new(root.clone(), true);
    let mut buf = Vec::new();
    let report = run_check_json(&opts, &mut buf).unwrap();

    assert_eq!(report.files_scanned, 2);
    assert_eq!(report.files_failed.len(), 1);
    assert_eq!(report.files_failed[0].path, PathBuf::from("bad.json"));
}

#[test]
fn tracked_but_deleted_file_is_skipped_rather_than_failing_the_hook() {
    let dir = tempfile::tempdir().unwrap();
    let root = &canon(&dir);
    write(&root.join("ok.json"), br#"{"a": 1}"#);
    write(&root.join("gone.json"), br#"{"a": 1}"#);
    if !stage_all(root) {
        return;
    }
    // `git ls-files` reports index entries, so an unstaged deletion (or a
    // sparse checkout) still lists the path. That is not a parse failure.
    std::fs::remove_file(root.join("gone.json")).unwrap();

    let opts = CheckerOptions::new(root.clone(), true);
    let mut buf = Vec::new();
    let report = run_check_json(&opts, &mut buf).unwrap();

    assert!(!report.failed(), "failures: {:?}", report.files_failed);
    assert_eq!(report.files_scanned, 1);
    assert_eq!(report.files_skipped, 1);
}

#[test]
#[cfg(unix)]
fn tracked_symlink_to_a_character_device_is_never_a_candidate() {
    let device = Path::new("/dev/zero");
    if !device.exists() {
        // TEST-26 / TASK-2126: surfaced, so a run without the device is
        // distinguishable from one that exercised the guard.
        ops_core::test_utils::skip_precondition(
            "/dev/zero fixture",
            "character device absent; the device-symlink assertions did not run",
        );
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let root = &canon(&dir);
    write(&root.join("ok.json"), br#"{"a": 1}"#);
    // A committed symlink to an endless device: `metadata()` reports length
    // 0, so a size gate lets it past, and an unbounded read never reaches
    // EOF. It must be rejected on file *type*, before any read.
    //
    // `ops_text_fixers::discovery` now applies that type test to both of its
    // modes, so the symlink is dropped from the candidate set before this
    // crate sees it and never reaches the checker's own `NotRegularFile`
    // skip. The hazard is handled one layer earlier; the property under test
    // is unchanged — the device is not read, and the run stays clean.
    std::os::unix::fs::symlink(device, root.join("evil.json")).unwrap();
    if !stage_all(root) {
        return;
    }

    let opts = CheckerOptions::new(root.clone(), true);
    let mut buf = Vec::new();
    let report = run_check_json(&opts, &mut buf).unwrap();

    assert!(!report.failed(), "failures: {:?}", report.files_failed);
    assert_eq!(report.files_scanned, 1, "only ok.json is read");
    assert_eq!(report.files_skipped, 0);
    let out = String::from_utf8(buf).unwrap();
    assert!(!out.contains("evil.json"), "out was {out:?}");
}

/// A walk error means the traversal silently omitted candidates, so the run
/// cannot honestly report "clean". Before this, the error was printed to the
/// writer and dropped: `failed()` stayed false and the CLI exited 0 over
/// directories it never read.
#[test]
fn walk_errors_make_the_report_fail() {
    let report = CheckerReport {
        walk_errors: vec!["IO error for operation on /x: permission denied".to_string()],
        ..CheckerReport::default()
    };
    assert!(
        report.failed(),
        "a traversal that lost candidates must not report success"
    );
    assert!(!CheckerReport::default().failed());
}

#[test]
fn write_summary_reports_each_counter_in_its_own_slot() {
    let report = CheckerReport {
        files_scanned: 7,
        files_failed: vec![
            FailedFile {
                path: PathBuf::from("a.json"),
                kind: FailureKind::Parse,
                message: "boom".to_string(),
            },
            FailedFile {
                path: PathBuf::from("b.json"),
                kind: FailureKind::Parse,
                message: "boom".to_string(),
            },
        ],
        files_skipped: 3,
        walk_errors: Vec::new(),
    };

    let mut buf = Vec::new();
    write_summary(&report, "check-json", &mut buf).unwrap();

    assert_eq!(
        String::from_utf8(buf).unwrap(),
        "check-json: scanned 7 file(s), 2 failed, 3 skipped, 0 walk error(s)\n"
    );
}

/// A run whose only problem is a traversal error must say so in the summary:
/// `failed()` is true (the walk hid candidates), so a line reporting only
/// "0 failed" would contradict the non-zero exit.
#[test]
fn the_summary_reports_walk_errors() {
    let report = CheckerReport {
        files_scanned: 1,
        files_failed: Vec::new(),
        files_skipped: 0,
        walk_errors: vec!["denied/: permission denied".to_string()],
    };
    assert!(report.failed(), "a walk error must fail the run");

    let mut buf = Vec::new();
    write_summary(&report, "check-json", &mut buf).unwrap();

    assert_eq!(
        String::from_utf8(buf).unwrap(),
        "check-json: scanned 1 file(s), 0 failed, 0 skipped, 1 walk error(s)\n"
    );
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
    let root = &canon(&dir);
    write(&root.join("bad.json"), br#"{"a": }"#);

    let opts = CheckerOptions::new(root.clone(), false);
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

// -- extension registration --

/// SEC-13 / TASK-2122 AC#1+#2+#4: the extension-registered specs spawn an
/// absolute program derived from `current_exe()` (not a bare, PATH-resolved
/// `"ops"` that any earlier `ops` shim could shadow), while the rendered
/// step line still reads `ops check-json` / `ops check-yaml`.
#[test]
fn registered_checkers_spawn_absolute_ops_and_display_as_ops() {
    use ops_core::config::CommandSpec;
    use ops_extension::Extension as _;

    let mut registry = ops_extension::CommandRegistry::new();
    super::ConfigCheckersExtension.register_commands(&mut registry);

    for id in ["check-json", "check-yaml"] {
        let Some(CommandSpec::Exec(exec)) = registry.get(id) else {
            panic!("{id} must be registered as an Exec spec");
        };
        // `current_exe()` succeeds under the test harness, so the program
        // must resolve absolute here; the literal "ops" is only the
        // fallback for when that lookup fails.
        assert!(
            std::path::Path::new(&exec.program).is_absolute(),
            "{id} must spawn an absolute current_exe()-derived program, got {:?}",
            exec.program
        );
        assert_eq!(exec.display_cmd(), format!("ops {id}"));
        assert_eq!(exec.args, vec![id.to_string()]);
    }
}
