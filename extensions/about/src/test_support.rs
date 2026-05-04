//! Shared test helpers for the about extensions.
//!
//! DUP-3 / TASK-0985: the ERR-7 sweep (TASK-0818 / TASK-0930 / TASK-0809)
//! pinned that path / directive tracing fields flow through `Debug` so
//! embedded newlines / ANSI escapes cannot forge log records. Each
//! provider grew its own `*_path_debug_escapes_control_characters` test
//! that re-proved the same property of `std::fmt::Debug`. Per-site tests
//! still exist (so the sweep contract is visible at every call site), but
//! they now share the assertion logic — deletions of one site no longer
//! weaken coverage silently.

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
pub fn assert_debug_escapes_control_chars<T: std::fmt::Debug>(value: T) {
    let rendered = format!("{value:?}");
    assert!(
        !rendered.contains('\n'),
        "raw newline leaked into Debug rendering: {rendered}"
    );
    assert!(
        !rendered.contains('\u{1b}'),
        "raw ANSI ESC leaked into Debug rendering: {rendered}"
    );
    assert!(
        rendered.contains("\\n"),
        "expected escaped newline in Debug rendering: {rendered}"
    );
}
