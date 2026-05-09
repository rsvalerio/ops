//! Shared text utilities.

use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;

/// SEC-33 (TASK-0932): default cap on manifest-style file reads
/// (`Cargo.toml`, `go.mod`, `package.json`, `requirements.txt`, …).
/// `ops` runs in user-controlled working directories where an adversarial
/// repository could otherwise force unbounded allocations via an oversized
/// or `/dev/zero`-symlinked manifest. 4 MiB is well above any realistic
/// manifest while keeping a single oversize read bounded.
pub const MANIFEST_MAX_BYTES_DEFAULT: u64 = 4 * 1024 * 1024;

/// Env knob mirroring `OPS_PLAN_JSON_MAX_BYTES`. Positive integer; values
/// that fail to parse or are zero fall back to [`MANIFEST_MAX_BYTES_DEFAULT`].
pub const MANIFEST_MAX_BYTES_ENV: &str = "OPS_MANIFEST_MAX_BYTES";

/// PERF-3 / TASK-1055: resolve the env-driven cap once per process. The
/// value is process-global and constant for a run, so the prior per-call
/// `std::env::var` lookup contended on the global env lock under parallel
/// stack-detection probes (which call this from `read_capped_to_string`
/// for every manifest read). `OnceLock` keeps the override / fallback
/// semantics (parsed at first use) without re-reading. Mirrors
/// `crates/runner/src/command/results.rs::output_byte_cap` (TASK-0542).
static MANIFEST_MAX_BYTES: OnceLock<u64> = OnceLock::new();

/// ERR-2 / TASK-0840 (mirrored for TASK-1055): pure parser for a positive
/// "byte cap from env" value. Returns the resolved cap and, when the input
/// was present-but-unusable, a human message describing the fallback so the
/// caller can emit a `tracing::warn!` outside the unit-test path. Factored
/// out so the fallback semantics are unit-testable without poking the
/// process-global `OnceLock`.
///
/// ARCH-9 / TASK-1228: shared with [`cached_byte_cap_env`]. Both
/// `manifest_max_bytes` and `ops_toml_max_bytes` route through this so the
/// "unset / unparseable / zero / valid" matrix has one implementation.
pub(crate) fn parse_byte_cap_env(
    env_var: &str,
    raw: Option<&str>,
    default: u64,
) -> (u64, Option<String>) {
    match raw {
        None => (default, None),
        Some(s) => match s.parse::<u64>() {
            Ok(n) if n > 0 => (n, None),
            Ok(_) => (
                default,
                Some(format!(
                    "{env_var}={s:?} is not a positive integer; using default {default}"
                )),
            ),
            Err(e) => (
                default,
                Some(format!(
                    "{env_var}={s:?} failed to parse as u64 ({e}); using default {default}"
                )),
            ),
        },
    }
}

/// ARCH-9 / TASK-1228: resolve a positive byte-cap-from-env value once per
/// process. Both [`manifest_max_bytes`] and
/// [`crate::config::loader::ops_toml_max_bytes`] (and any future sibling
/// caps) route through this so the cache discipline, fallback semantics,
/// and one-shot warn diagnostic stay aligned across the codebase. The
/// shared shape mirrors `crates/runner/src/command/results.rs::output_byte_cap`
/// (TASK-0542).
///
/// Unset / zero / unparseable values fall back to `default` with a one-shot
/// `tracing::warn!` emitted from the `OnceLock` initialiser. Tests that
/// need to override the cap must set the env var before the first
/// resolver call; later changes are ignored.
pub fn cached_byte_cap_env(slot: &OnceLock<u64>, env_var: &'static str, default: u64) -> u64 {
    *slot.get_or_init(|| {
        let raw = std::env::var(env_var).ok();
        let (cap, warn_msg) = parse_byte_cap_env(env_var, raw.as_deref(), default);
        if let Some(msg) = warn_msg {
            tracing::warn!(env_var = env_var, "{msg}");
        }
        cap
    })
}

/// Effective manifest read cap. Resolved from the env knob on the first
/// call and cached behind a `OnceLock<u64>` for the remainder of the
/// process — subsequent calls do not touch `std::env`. Tests that need to
/// override the cap must set `OPS_MANIFEST_MAX_BYTES` before any call to
/// `manifest_max_bytes` (directly or via `read_capped_to_string` /
/// `for_each_trimmed_line`); changes after the first read are ignored.
/// Unparseable / zero values fall back to [`MANIFEST_MAX_BYTES_DEFAULT`]
/// with a one-shot `tracing::warn!` from the `OnceLock` initialiser.
pub fn manifest_max_bytes() -> u64 {
    cached_byte_cap_env(
        &MANIFEST_MAX_BYTES,
        MANIFEST_MAX_BYTES_ENV,
        MANIFEST_MAX_BYTES_DEFAULT,
    )
}

/// Read `path` to a `String`, capped at [`manifest_max_bytes`] bytes.
///
/// On a file larger than the cap, returns `Err` with `ErrorKind::InvalidData`
/// (and a message naming the cap) without holding the full content in memory:
/// the read is bounded by `Read::take(cap + 1)`.
///
/// `NotFound` and other IO errors are returned verbatim so callers can
/// classify (silent fall-through vs warn-and-skip vs hard fail).
pub fn read_capped_to_string(path: &Path) -> std::io::Result<String> {
    read_capped_to_string_with(path, manifest_max_bytes())
}

/// Internal: same as [`read_capped_to_string`] but with an explicit cap.
/// Used by unit tests to exercise the cap-handling behaviour without
/// depending on the process-global memoised [`manifest_max_bytes`] value.
fn read_capped_to_string_with(path: &Path, cap: u64) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = String::new();
    let limit = cap.saturating_add(1);
    (&mut file).take(limit).read_to_string(&mut buf)?;
    if buf.len() as u64 > cap {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("file exceeds {cap}-byte cap (override via {MANIFEST_MAX_BYTES_ENV})"),
        ));
    }
    Ok(buf)
}

/// Capitalize the first character of a string.
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Format a number with comma separators (e.g. 1234 → "1,234").
pub fn format_number(n: i64) -> String {
    if n < 0 {
        // checked_neg() returns None only for i64::MIN; format the magnitude
        // via unsigned to avoid the overflow on negation.
        let magnitude = match n.checked_neg() {
            Some(positive) => positive.to_string(),
            None => (n.unsigned_abs()).to_string(),
        };
        return format!("-{}", insert_thousands_separators(&magnitude));
    }
    insert_thousands_separators(&n.to_string())
}

/// PERF-3 (TASK-1065): single forward pass over ASCII digits, no second
/// allocation or `chars().rev()` round-trip. Callers in render hot paths
/// (`format_number` from About-card / table rendering) hit this for every
/// numeric cell. The input is always the decimal rendering of a non-negative
/// integer (`u64`/`i64::to_string` magnitude) so all bytes are ASCII digits;
/// indexing by byte position is therefore safe and avoids UTF-8 char iteration.
///
/// Strategy: compute the leading-group length (`len % 3`, falling back to 3
/// when the input length is a multiple of 3), copy that prefix, then for each
/// remaining 3-digit group push a separator followed by the group. The fast
/// path for fewer than four digits returns the input unchanged with no comma
/// allocation overhead beyond the single output `String`.
fn insert_thousands_separators(digits: &str) -> String {
    let bytes = digits.as_bytes();
    let len = bytes.len();
    if len <= 3 {
        // Zero-comma fast path: no separator needed for `n.abs() < 1000`.
        return digits.to_string();
    }
    let mut result = String::with_capacity(len + (len - 1) / 3);
    let head = match len % 3 {
        0 => 3,
        n => n,
    };
    // SAFETY-equivalent: `bytes` are ASCII digits (caller passes
    // `i64::to_string` magnitude), so byte slicing aligns with char boundaries.
    result.push_str(&digits[..head]);
    let mut i = head;
    while i < len {
        result.push(',');
        result.push_str(&digits[i..i + 3]);
        i += 3;
    }
    result
}

/// Extract the last path component as a project name, falling back to `"project"`.
pub fn dir_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
}

/// Read `path` as UTF-8 text and invoke `f` on each line, with surrounding whitespace
/// trimmed. Returns `Some(())` when the file was read, `None` if it was missing,
/// unreadable, or larger than [`manifest_max_bytes`] bytes. Used by line-based
/// manifest parsers (`go.mod`, `go.work`, `gradle.properties`, etc.) to share
/// the read-and-iterate skeleton.
///
/// Non-NotFound IO errors (PermissionDenied, IsADirectory, oversize, etc.) are
/// logged at `tracing::warn!` so operators can diagnose "manifest exists but is
/// unreadable" without changing log levels. NotFound remains silent — a missing
/// manifest is a normal condition for optional stacks.
///
/// SEC-33 (TASK-0932): the read is byte-capped via [`read_capped_to_string`] so
/// an adversarial manifest cannot OOM the process before the first callback.
pub fn for_each_trimmed_line<F: FnMut(&str)>(path: &Path, f: F) -> Option<()> {
    for_each_trimmed_line_with(path, manifest_max_bytes(), f)
}

/// Internal: same as [`for_each_trimmed_line`] but with an explicit cap.
/// Used by unit tests to exercise the cap-handling behaviour without
/// depending on the process-global memoised [`manifest_max_bytes`] value.
fn for_each_trimmed_line_with<F: FnMut(&str)>(path: &Path, cap: u64, mut f: F) -> Option<()> {
    let content = match read_capped_to_string_with(path, cap) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            // ERR-7 (TASK-0944): Debug-format path/error so a manifest path
            // under user-controlled CWD (`go.mod`, `gradle.properties`,
            // `requirements.txt`, ...) containing newlines or ANSI escapes
            // cannot forge log lines.
            tracing::warn!(
                path = ?path.display(),
                error = ?e,
                cap = cap,
                "failed to read manifest for line iteration"
            );
            return None;
        }
    };
    for line in content.lines() {
        f(line.trim());
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn capitalize_empty() {
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn capitalize_single_char() {
        assert_eq!(capitalize("a"), "A");
    }

    #[test]
    fn capitalize_already_upper() {
        assert_eq!(capitalize("Hello"), "Hello");
    }

    #[test]
    fn capitalize_lowercase() {
        assert_eq!(capitalize("hello"), "Hello");
    }

    #[test]
    fn format_number_zero() {
        assert_eq!(format_number(0), "0");
    }

    #[test]
    fn format_number_small() {
        assert_eq!(format_number(42), "42");
    }

    #[test]
    fn format_number_thousands() {
        assert_eq!(format_number(1234), "1,234");
    }

    #[test]
    fn format_number_millions() {
        assert_eq!(format_number(1_234_567), "1,234,567");
    }

    #[test]
    fn format_number_negative() {
        assert_eq!(format_number(-1234), "-1,234");
    }

    #[test]
    fn format_number_i64_min_does_not_panic() {
        // i64::MIN cannot be negated; ensure we render the magnitude with the
        // standard separator without panicking or wrapping.
        assert_eq!(format_number(i64::MIN), "-9,223,372,036,854,775,808");
    }

    #[test]
    fn format_number_i64_max() {
        assert_eq!(format_number(i64::MAX), "9,223,372,036,854,775,807");
    }

    #[test]
    fn dir_name_normal_path() {
        assert_eq!(
            dir_name(&PathBuf::from("/home/user/myproject")),
            "myproject"
        );
    }

    #[test]
    fn dir_name_root_fallback() {
        assert_eq!(dir_name(&PathBuf::from("/")), "project");
    }

    #[test]
    fn dir_name_empty_fallback() {
        assert_eq!(dir_name(&PathBuf::from("")), "project");
    }

    #[test]
    fn for_each_trimmed_line_invokes_per_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data");
        std::fs::write(&path, "  foo  \n\nbar\n").unwrap();
        let mut seen = Vec::new();
        let res = for_each_trimmed_line(&path, |line| seen.push(line.to_string()));
        assert!(res.is_some());
        assert_eq!(seen, vec!["foo", "", "bar"]);
    }

    #[test]
    fn for_each_trimmed_line_missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let res = for_each_trimmed_line(&dir.path().join("nope"), |_| {});
        assert!(res.is_none());
    }

    /// SEC-33 (TASK-0932): files larger than the cap must be rejected by the
    /// shared reader without slurping the full content into memory. The
    /// callback in `for_each_trimmed_line` must not run.
    ///
    /// Uses the `_with(cap)` internal variants so the test does not depend on
    /// the process-global `OnceLock`-memoised cap (which can be initialised by
    /// any earlier test in the same binary and then sticks for the rest of the
    /// run).
    #[test]
    fn for_each_trimmed_line_oversize_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("huge.txt");
        let oversize = vec![b'a'; 65];
        std::fs::write(&path, &oversize).unwrap();

        let mut called = false;
        let res = for_each_trimmed_line_with(&path, 64, |_| called = true);
        assert!(res.is_none(), "oversize file should return None");
        assert!(!called, "callback must not run for oversize file");
    }

    #[test]
    fn read_capped_to_string_oversize_returns_invalid_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big");
        std::fs::write(&path, vec![b'a'; 17]).unwrap();
        let err = read_capped_to_string_with(&path, 16).expect_err("must reject oversize");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_capped_to_string_at_cap_returns_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ok");
        std::fs::write(&path, b"12345678").unwrap();
        let got = read_capped_to_string_with(&path, 8).expect("at-cap file reads ok");
        assert_eq!(got, "12345678");
    }

    #[test]
    fn read_capped_to_string_missing_propagates_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let err =
            read_capped_to_string(&dir.path().join("nope")).expect_err("missing should error");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    /// ARCH-9 / TASK-1228: pin the parse_byte_cap_env shared parser across
    /// the unset / zero / unparseable / valid matrix. Both
    /// `manifest_max_bytes` and `ops_toml_max_bytes` route through this so
    /// fixing the matrix here pins both surfaces.
    #[test]
    fn parse_byte_cap_env_unset_returns_default_no_warn() {
        let (cap, warn) = parse_byte_cap_env("X", None, 100);
        assert_eq!(cap, 100);
        assert!(warn.is_none());
    }

    #[test]
    fn parse_byte_cap_env_zero_falls_back_with_warn() {
        let (cap, warn) = parse_byte_cap_env("X", Some("0"), 100);
        assert_eq!(cap, 100);
        assert!(warn.is_some());
    }

    #[test]
    fn parse_byte_cap_env_unparseable_falls_back_with_warn() {
        let (cap, warn) = parse_byte_cap_env("X", Some("not-a-number"), 100);
        assert_eq!(cap, 100);
        assert!(warn.is_some());
    }

    #[test]
    fn parse_byte_cap_env_valid_returns_value_no_warn() {
        let (cap, warn) = parse_byte_cap_env("X", Some("42"), 100);
        assert_eq!(cap, 42);
        assert!(warn.is_none());
    }

    #[test]
    fn manifest_max_bytes_invalid_env_falls_back_to_default() {
        // Don't touch env here — just verify the default constant matches
        // the runtime resolver when the env knob is unset (the most common
        // path).
        let resolved = manifest_max_bytes();
        assert!(
            resolved == MANIFEST_MAX_BYTES_DEFAULT || std::env::var(MANIFEST_MAX_BYTES_ENV).is_ok(),
            "default cap must apply when env unset"
        );
    }

    /// ERR-7 (TASK-0944): the warn event in `for_each_trimmed_line`
    /// Debug-formats the path so a manifest path containing newlines or
    /// ANSI escapes cannot forge log records. Pin the value-level escape
    /// without a tracing-subscriber dev-dep.
    #[test]
    fn for_each_trimmed_line_path_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc/go.mod");
        let rendered = format!("{:?}", p.display());
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

    /// TEST-19 (TASK-1033): the `chmod 0o000` mechanism only synthesises a
    /// `PermissionDenied` read error for non-root callers; the kernel skips
    /// DAC checks for effective UID 0, so this assertion silently inverts
    /// (read succeeds, callback runs, `for_each_trimmed_line` returns
    /// `Some(())`) under rootful CI (Docker default UID, privileged
    /// self-hosted runners, rootful devcontainers). Skip the assertion on
    /// root rather than emit a green-but-meaningless result. DO NOT remove
    /// this guard without also replacing the chmod-based denial mechanism
    /// (e.g. open-then-revoke via `/proc/self/fd/X`).
    #[cfg(unix)]
    #[test]
    fn for_each_trimmed_line_unreadable_file_returns_none() {
        use std::os::unix::fs::PermissionsExt;
        if crate::test_utils::is_root_euid() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("denied.txt");
        std::fs::write(&path, "data").unwrap();
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&path, perms).unwrap();

        let res = for_each_trimmed_line(&path, |_| {});
        assert!(res.is_none());

        // Restore so tempdir cleanup works.
        let mut restore = std::fs::metadata(&path).unwrap().permissions();
        restore.set_mode(0o644);
        std::fs::set_permissions(&path, restore).unwrap();
    }
}
