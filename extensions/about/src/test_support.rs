//! Shared test helpers for the about extensions.
//!
//! Path / directive tracing fields must flow through `Debug` so embedded
//! newlines / ANSI escapes cannot forge log records. Each provider keeps
//! its own `*_path_debug_escapes_control_characters` test so the contract
//! is visible at every call site, and they share the assertion logic —
//! deleting one site cannot silently weaken coverage.
//!
//! The tracing-capture harness (`capture_tracing`, `capture_warn`,
//! `count_warnings`, `pin_global_dispatcher`, `TracingBuf`, `WarnCounter`)
//! lives in `ops_core::test_utils`: it is shared with `crates/core`,
//! `crates/cli` and `extensions/git`, which — like every about-family
//! crate — depend on `ops-core` directly. Import it from there.

/// Pin the property guaranteed by `Debug` formatting on `Path::display()`
/// (or any value carrying user-controlled text):
///
/// 1. raw newlines must not survive in the rendered field,
/// 2. ANSI escape (ESC, U+001B) must not survive,
/// 3. the rendered field must contain the escaped form `\n`.
///
/// Each `about` extension's per-provider test calls this with a value
/// shaped like its own tracing site, so removing one provider's site
/// does not weaken sweep coverage elsewhere.
///
/// # Panics
///
/// If `value`'s `Debug` rendering leaks a raw newline or ANSI escape — that
/// is the assertion this helper exists to make.
pub fn assert_debug_escapes_control_chars<T: std::fmt::Debug>(value: T) {
    let rendered = format!("{value:?}");
    assert_rendered_escapes_control_chars(&rendered);
    assert!(
        rendered.contains("\\n"),
        "expected escaped newline in Debug rendering: {rendered}"
    );
}

/// The half of [`assert_debug_escapes_control_chars`] that applies to an
/// already-rendered string: no raw newline and no raw ANSI `ESC` survived.
///
/// Callers that capture a real `tracing` record (rather
/// than rendering a value themselves) assert the same property on the captured
/// text — trim the record's own trailing newline first. Splitting it out keeps
/// one definition instead of a second copy at every capture site.
///
/// # Panics
///
/// If `rendered` carries a raw newline or a raw ANSI `ESC` — that is the
/// assertion this helper exists to make.
pub fn assert_rendered_escapes_control_chars(rendered: &str) {
    assert!(
        !rendered.contains('\n'),
        "raw newline leaked into rendered output: {rendered}"
    );
    assert!(
        !rendered.contains('\u{1b}'),
        "raw ANSI ESC leaked into rendered output: {rendered}"
    );
}

/// Write `content` to `path`, creating any missing parent directories.
///
/// Import it as `use ops_about::test_support::write_file as write;` to
/// keep existing call sites unchanged.
///
/// # Panics
///
/// If the parent directories cannot be created or the file cannot be
/// written — this is fixture setup, where a failure is a broken test, not a
/// condition to handle.
pub fn write_file(path: &std::path::Path, content: &str) {
    // The workspace bans `unwrap`/`expect`/`panic!` outside `#[cfg(test)]`,
    // and this module is compiled as library code, so failures are reported
    // through `assert!` — which carries the same message and the same
    // "broken fixture" semantics.
    if let Some(parent) = path.parent() {
        let created = std::fs::create_dir_all(parent);
        assert!(
            created.is_ok(),
            "create fixture dir {}: {:?}",
            parent.display(),
            created.err()
        );
    }
    let written = std::fs::write(path, content);
    assert!(
        written.is_ok(),
        "write fixture file {}: {:?}",
        path.display(),
        written.err()
    );
}
