//! Tests for `interpret_machete_output`: which `(exit code, stdout, stderr)`
//! triples may be parsed as authoritative, and which must fail closed.
//!
//! The fixtures are cargo-machete 0.9.2 output captured verbatim.

use super::*;

const CLEAN_STDOUT: &str =
    "cargo-machete didn't find any unused dependencies in this directory. Good job!\n";

const FOUND_STDOUT: &str = "\
cargo-machete found the following unused dependencies in this directory:
ops-deps -- ./extensions-rust/deps/Cargo.toml:
\tlinkme
ops-run-before-push -- ./extensions/run-before-push/Cargo.toml:
\tanyhow
\tlinkme

If you believe cargo-machete has detected an unused dependency incorrectly,
you can add the dependency to the list of dependencies to ignore in the
`[package.metadata.cargo-machete]` section of the appropriate Cargo.toml.
For example:

[package.metadata.cargo-machete]
ignored = [\"prost\"]

You can also try running it with the `--with-metadata` flag for better accuracy,
though this may modify your Cargo.lock files.

";

const PROGRESS_STDERR: &str = "Analyzing dependencies of crates in this directory...\nDone!\n";

fn interpret(code: Option<i32>, stdout: &str, stderr: &str) -> anyhow::Result<Vec<UnusedDepEntry>> {
    interpret_machete_output(code, stdout.as_bytes(), stderr.as_bytes())
}

#[test]
fn clean_exit_zero_yields_no_entries() {
    let entries = interpret(Some(0), CLEAN_STDOUT, PROGRESS_STDERR).expect("clean run parses");
    assert!(entries.is_empty());
}

/// Exit 1 is cargo-machete's "found something" answer, not a tool failure.
#[test]
fn exit_one_lists_each_dependency_with_package_and_manifest() {
    let entries = interpret(Some(1), FOUND_STDOUT, PROGRESS_STDERR).expect("exit 1 parses");
    let got: Vec<(&str, &str, &str)> = entries
        .iter()
        .map(|e| {
            (
                e.package.as_str(),
                e.manifest_path.as_str(),
                e.dependency.as_str(),
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            ("ops-deps", "./extensions-rust/deps/Cargo.toml", "linkme"),
            (
                "ops-run-before-push",
                "./extensions/run-before-push/Cargo.toml",
                "anyhow"
            ),
            (
                "ops-run-before-push",
                "./extensions/run-before-push/Cargo.toml",
                "linkme"
            ),
        ]
    );
}

/// The `ignored but actually used` note goes to stderr and is not a finding.
#[test]
fn ignored_but_used_note_on_stderr_is_not_an_error() {
    let stderr = "Analyzing dependencies of crates in this directory...\n\
                  \t\u{26a0}\u{fe0f}  serde was marked as ignored, but is actually used!\nDone!\n";
    let entries = interpret(Some(1), FOUND_STDOUT, stderr).expect("note is informational");
    assert_eq!(entries.len(), 3);
}

/// cargo-machete skips a manifest it cannot parse and still exits 0.
#[test]
fn manifest_error_on_stderr_fails_closed_even_at_exit_zero() {
    let stderr = "Analyzing dependencies of crates in this directory...\n\
                  error when handling ./Cargo.toml: TOML parse error at line 5, column 14\n\
                  Done!\n";
    let err = interpret(Some(0), CLEAN_STDOUT, stderr).expect_err("broken manifest must surface");
    let msg = err.to_string();
    assert!(msg.contains("could not analyse a manifest"), "{msg}");
    assert!(
        msg.contains("./Cargo.toml"),
        "must name the manifest: {msg}"
    );
}

#[test]
fn exit_two_fails_with_stderr_tail() {
    let stderr = "Done!\nError: Errors when walking over directories:\n/x: No such file\n";
    let err = interpret(Some(2), "", stderr).expect_err("exit 2 is an error");
    let msg = err.to_string();
    assert!(msg.contains("status 2"), "{msg}");
    assert!(msg.contains("walking over directories"), "{msg}");
}

#[test]
fn signal_kill_fails_closed() {
    let err = interpret(None, "", "").expect_err("signal must surface");
    assert!(err.to_string().contains("signal"), "{err}");
}

#[test]
fn unexpected_exit_code_fails_closed() {
    let err = interpret(Some(101), CLEAN_STDOUT, "panicked").expect_err("101 must surface");
    let msg = err.to_string();
    assert!(msg.contains("unexpected status code 101"), "{msg}");
    assert!(msg.contains("panicked"), "{msg}");
}

#[test]
fn exit_one_with_clean_summary_fails_closed() {
    let err = interpret(Some(1), CLEAN_STDOUT, "").expect_err("contradiction must surface");
    assert!(err.to_string().contains("status 1"), "{err}");
}

#[test]
fn exit_zero_with_listing_fails_closed() {
    let err = interpret(Some(0), FOUND_STDOUT, "").expect_err("contradiction must surface");
    assert!(err.to_string().contains("status 0"), "{err}");
}

/// Empty stdout at exit 0 (a wrapper that swallowed it, a changed summary
/// wording) is not a clean run.
#[test]
fn exit_zero_with_empty_stdout_fails_closed() {
    let err = interpret(Some(0), "", PROGRESS_STDERR).expect_err("empty stdout must surface");
    assert!(err.to_string().contains("neither"), "{err}");
}

#[test]
fn reworded_summary_fails_closed() {
    let stdout = "cargo-machete: no unused dependencies found. Good job!\n";
    let err = interpret(Some(0), stdout, "").expect_err("unknown summary must surface");
    assert!(err.to_string().contains("does not recognise"), "{err}");
}

/// A dependency line that lost its tab indent would otherwise read as a
/// malformed header and drop the dependency.
#[test]
fn listing_line_without_tab_fails_closed() {
    let stdout = "\
cargo-machete found the following unused dependencies in this directory:
app -- ./Cargo.toml:
    serde
";
    let err = interpret(Some(1), stdout, "").expect_err("space-indented dep must surface");
    assert!(err.to_string().contains("does not recognise"), "{err}");
}

#[test]
fn package_header_without_dependencies_fails_closed() {
    let stdout = "\
cargo-machete found the following unused dependencies in this directory:
app -- ./Cargo.toml:
lib -- ./lib/Cargo.toml:
\tserde
";
    let err = interpret(Some(1), stdout, "").expect_err("empty package must surface");
    assert!(err.to_string().contains("`app`"), "{err}");
}

#[test]
fn dependency_before_any_package_fails_closed() {
    let stdout = "\
cargo-machete found the following unused dependencies in this directory:
\tserde
";
    let err = interpret(Some(1), stdout, "").expect_err("orphan dep must surface");
    assert!(err.to_string().contains("does not recognise"), "{err}");
}

#[test]
fn listing_header_with_no_entries_fails_closed() {
    let stdout = "cargo-machete found the following unused dependencies in this directory:\n\n";
    let err = interpret(Some(1), stdout, "").expect_err("empty listing must surface");
    assert!(err.to_string().contains("listed none"), "{err}");
}

/// A listing cut off before its trailer (no blank line) still parses: the
/// trailer is advice, not data.
#[test]
fn listing_without_trailer_parses() {
    let stdout = "\
cargo-machete found the following unused dependencies in this directory:
app -- ./Cargo.toml:
\tserde";
    let entries = interpret(Some(1), stdout, "").expect("no trailer is fine");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].dependency, "serde");
}

#[test]
fn package_header_splits_on_first_separator() {
    assert_eq!(
        parse_package_header("app -- ./a -- b/Cargo.toml:"),
        Some(("app", "./a -- b/Cargo.toml"))
    );
    assert_eq!(parse_package_header("app -- ./Cargo.toml"), None);
    assert_eq!(parse_package_header(" -- ./Cargo.toml:"), None);
}
