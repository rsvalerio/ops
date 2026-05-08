//! Detect whether tools/components are installed on the active toolchain.

use ops_core::config::tools::{ToolSource, ToolSpec};
use ops_core::output::format_error_tail;
use ops_core::subprocess::{
    default_timeout, resolve_cargo_bin, resolve_rustup_bin, run_with_timeout, RunError,
};
use std::collections::HashSet;
use std::ffi::OsString;
use std::process::Command;
use std::time::Duration;

use crate::ToolStatus;

/// ASYNC-6 / TASK-0914: default deadline for tool/listing probes
/// (`rustup show active-toolchain`, `cargo --list`, `rustup component list
/// --installed`). The whole `ops about` / `ops tools list` UX hangs on
/// these probes, so cap them well under the user's "is this CLI broken?"
/// threshold while still giving rustup time to refresh metadata on a slow
/// network. Override globally via `OPS_SUBPROCESS_TIMEOUT_SECS` — handled
/// by [`default_timeout`].
const PROBE_TIMEOUT: Duration = Duration::from_secs(15);

/// Run a probe Command under [`run_with_timeout`], logging timeout / IO
/// errors at `tracing::warn` and returning `None` so the caller can map
/// the failure to `ToolStatus::NotInstalled` without duplicating the
/// logging pattern at every call site.
///
/// READ-7 / TASK-0992: prior comments referenced a `ToolStatus::Unknown`
/// variant; that variant was declared but never constructed and has been
/// removed. Probe failures (timeout / spawn error) currently flow into
/// `NotInstalled`. If a future change wants to distinguish "probe failed"
/// from "tool genuinely missing", reintroduce the variant and wire it
/// through here at the same time.
fn run_probe_with_timeout(cmd: &mut Command, label: &'static str) -> Option<std::process::Output> {
    match run_with_timeout(cmd, default_timeout(PROBE_TIMEOUT), label) {
        Ok(out) => Some(out),
        Err(RunError::Timeout(e)) => {
            tracing::warn!(
                label,
                timeout_secs = e.timeout.as_secs(),
                "ASYNC-6 / TASK-0914: probe timed out; reporting tool as not installed"
            );
            None
        }
        Err(RunError::Io(e)) => {
            tracing::warn!(
                label,
                error = %e,
                "probe spawn failed; reporting tool as not installed"
            );
            None
        }
        Err(other) => {
            tracing::warn!(
                label,
                error = %other,
                "probe failed with unrecognized error variant; reporting tool as not installed"
            );
            None
        }
    }
}

pub fn get_active_toolchain() -> Option<String> {
    // `--quiet` is rustup's global flag, not a subcommand option, so it
    // appears before `show`. It silences "info: ..." progress lines so the
    // first line of stdout is reliably the toolchain name on every rustup.
    //
    // ASYNC-6 / TASK-0914: a wedged rustup proxy (broken sccache shim,
    // stuck registry probe) used to hang `ops about` indefinitely. Cap
    // the spawn at `PROBE_TIMEOUT` (overridable via
    // `OPS_SUBPROCESS_TIMEOUT_SECS`).
    let mut cmd = Command::new(resolve_rustup_bin());
    cmd.args(["--quiet", "show", "active-toolchain"]);
    let output = run_probe_with_timeout(&mut cmd, "rustup show active-toolchain")?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_active_toolchain(&stdout)
}

/// Parse the toolchain name out of `rustup show active-toolchain` stdout.
///
/// Handles both output shapes rustup has shipped:
///
/// * Legacy (rustup <1.28): a single line like `stable-aarch64-apple-darwin (default)`.
/// * Current (rustup ≥1.28): a multi-line block whose first non-empty line
///   is the bare toolchain name (e.g. `stable-aarch64-apple-darwin`)
///   followed by `active because: ...`.
///
/// Both shapes are handled by skipping blank/leading lines and returning the
/// first whitespace-separated token of the first non-empty line.
///
/// Rejects diagnostic lines that rustup prints when there is no active
/// toolchain (e.g. "error: ...", "info: ...") so they don't get used as a
/// toolchain identifier.
///
/// PATTERN-1 / TASK-1078: only the rustup diagnostic prefixes
/// (`error:`, `warning:`, `info:`, `note:`) cause rejection. A blanket
/// "contains ':'" check would reject legitimate identifiers — custom
/// toolchains registered via `rustup toolchain link` may contain `:` in
/// their names, and on Windows `rustup show active-toolchain` can surface
/// `C:\path\...` shaped tokens. Match the prefix on the full first
/// whitespace-bounded segment, not as a substring.
pub(crate) fn parse_active_toolchain(stdout: &str) -> Option<String> {
    const RUSTUP_DIAGNOSTIC_PREFIXES: &[&str] = &["error:", "warning:", "info:", "note:"];

    let line = stdout.lines().map(str::trim).find(|l| !l.is_empty())?;
    let token = line.split_whitespace().next()?;
    if RUSTUP_DIAGNOSTIC_PREFIXES.contains(&token) {
        return None;
    }
    Some(token.to_string())
}

pub fn check_cargo_tool_installed(name: &str) -> bool {
    let mut cmd = Command::new(resolve_cargo_bin());
    cmd.args(["--list"]);
    let output = match run_probe_with_timeout(&mut cmd, "cargo --list") {
        Some(o) => o,
        None => return false,
    };

    if !output.status.success() {
        // ERR-1 / TASK-1032: route through `format_error_tail` so the snippet
        // is byte-bounded (last N lines, decoded via `from_utf8_lossy`) instead
        // of `chars().take(200)` which counts Unicode scalar values, can blow
        // up to 600+ bytes on CJK/RTL output, and risks leaving a malformed
        // grapheme fragment in logs.
        let stderr_snippet = format_error_tail(&output.stderr, 10);
        tracing::warn!(
            tool = name,
            code = ?output.status.code(),
            stderr = ?stderr_snippet,
            "cargo --list exited non-zero; reporting tool as not installed"
        );
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Standalone binaries installed via `cargo install` (e.g. tokei) don't
    // appear in `cargo --list` — fall back to checking PATH.
    is_in_cargo_list(&stdout, name) || check_binary_installed(name)
}

/// Cached index of executable basenames found on `$PATH`.
///
/// PERF-3 / TASK-1046: `collect_tools` previously fell through to
/// [`check_binary_installed`] for every Cargo-source tool that did not appear
/// in `cargo --list` (any tool installed standalone via `cargo install`,
/// e.g. `tokei`, `bacon`). Each fallback re-walked the entire `$PATH`
/// (`std::env::split_paths` × `std::fs::metadata` per directory entry, plus
/// `$PATHEXT` on Windows), turning what should be O(N) probes into
/// O(N × |PATH| × |PATHEXT|).
///
/// Capturing the index once at the top of [`crate::collect_tools`] amortises
/// the walk into a single pass and reduces the per-tool fallback to a hash
/// lookup. The set stores the raw directory entry filenames (without
/// stripping `$PATHEXT`) — the lookup helper handles the suffix list, so the
/// index stays portable.
///
/// CONC-7 / TASK-1249: on Windows the filesystem is case-insensitive
/// (`tokei.EXE`, `Tokei.exe`, `tokei.exe` all refer to the same file)
/// but `OsString` equality is case-sensitive. To avoid false-negative
/// install probes for cargo-installed binaries with mixed-case names,
/// the index normalises basenames to lowercase under `cfg(windows)` —
/// both at insert time in [`capture_path_index_from`] and at lookup time
/// in [`is_in_path_index`]. Unix is unchanged: filenames are case-
/// sensitive on POSIX filesystems and verbatim equality is correct.
pub type PathIndex = HashSet<OsString>;

/// CONC-7 / TASK-1249: normalise an `OsString` basename to the index key
/// form. Lowercase on Windows; verbatim on Unix.
pub(crate) fn index_key(name: OsString) -> OsString {
    if cfg!(windows) {
        // `to_string_lossy` is safe here: Windows filenames go through
        // WTF-8 / UTF-16 and the `String::to_lowercase` ASCII-fold is
        // sufficient for the case-insensitive equality the filesystem
        // already enforces. Non-UTF-8 components fall through verbatim
        // (an unrealistic edge case on NTFS).
        OsString::from(name.to_string_lossy().to_lowercase())
    } else {
        name
    }
}

/// Build a one-shot index of executable basenames present on `$PATH`.
///
/// Returns `None` when `$PATH` is unset (matches [`find_on_path`]'s shape).
/// Unreadable directories are skipped with a `tracing::warn!` so a single
/// stale `$PATH` entry can't poison the whole probe sweep.
///
/// On Unix the executable bit is enforced via the same [`check_executable`]
/// path used by [`find_on_path_in`]; on Windows we accept any regular file
/// because the OS does not surface an executable bit and the suffix loop in
/// [`is_in_path_index`] already constrains matches to `$PATHEXT`.
pub fn capture_path_index() -> Option<PathIndex> {
    let path = std::env::var_os("PATH")?;
    Some(capture_path_index_from(&path))
}

pub(crate) fn capture_path_index_from(path_var: &std::ffi::OsStr) -> PathIndex {
    let mut set: PathIndex = HashSet::new();
    for dir in std::env::split_paths(path_var) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(e) => {
                // Missing PATH entries are common and noisy at info-level.
                // Only surface unexpected errors (e.g. permission denied).
                if e.kind() != std::io::ErrorKind::NotFound {
                    tracing::warn!(
                        path = %dir.display(),
                        error = %e,
                        "PATH entry unreadable while building path index; skipping"
                    );
                }
                continue;
            }
        };
        for entry in entries.flatten() {
            let candidate = entry.path();
            if matches!(check_executable(&candidate), ExecCheck::Yes) {
                set.insert(index_key(entry.file_name()));
            }
        }
    }
    set
}

/// Look up `name` in a precomputed [`PathIndex`].
///
/// On Windows the lookup also tries each `$PATHEXT` suffix, mirroring the
/// behaviour of [`find_on_path_in`].
pub(crate) fn is_in_path_index(index: &PathIndex, name: &str) -> bool {
    // CONC-7 / TASK-1249: lookup keys are normalised on Windows so a
    // probe for "tokei" matches an on-disk `Tokei.EXE`. See `index_key`.
    if index.contains(&index_key(OsString::from(name))) {
        return true;
    }
    if cfg!(windows) {
        for ext in pathext_suffixes() {
            let mut candidate = OsString::from(name);
            candidate.push(&ext);
            if index.contains(&index_key(candidate)) {
                return true;
            }
        }
    }
    false
}

/// PERF-3 / TASK-1046: `check_binary_installed` variant that consults a
/// precomputed [`PathIndex`] when supplied, falling back to the per-call
/// `$PATH` walk when `index` is `None`. Preserves the public API for
/// one-off callers (CLI subcommands, tests) while letting `collect_tools`
/// amortise the walk across N tools.
pub fn check_binary_installed_with(name: &str, index: Option<&PathIndex>) -> bool {
    match index {
        Some(idx) => is_in_path_index(idx, name),
        None => check_binary_installed(name),
    }
}

/// PATTERN-1 / TASK-1101: `cargo --list` enumerates *every* subcommand cargo
/// knows about — built-ins like `build` / `check` / `test` / `run` ship inside
/// the cargo binary itself, not as separately installable `cargo-*` crates.
/// Historically `is_in_cargo_list` happily matched these names, so a
/// `tools.toml` entry called `build` (a `ToolSource::Cargo` spec) would be
/// reported `Installed` despite no `cargo install cargo-build` ever having
/// run, and a subsequent `install_tool` would short-circuit on the
/// `is_installed` flag without actually installing anything. Filter the
/// known built-ins out of the membership check so a deliberate (or
/// accidental) collision falls through to the PATH-based binary probe and
/// ultimately drives an install.
///
/// Coverage is intentionally comprehensive but not exhaustive; cargo grows
/// new built-ins occasionally, and "missing some" only re-exposes the prior
/// false-positive for that specific name. The common ones — the full set of
/// build / test / dependency / publish workflow verbs — are pinned here.
const CARGO_BUILTIN_SUBCOMMANDS: &[&str] = &[
    "add",
    "bench",
    "build",
    "check",
    "clean",
    "clippy",
    "config",
    "doc",
    "fetch",
    "fix",
    "fmt",
    "generate-lockfile",
    "help",
    "init",
    "install",
    "locate-project",
    "login",
    "logout",
    "metadata",
    "new",
    "owner",
    "package",
    "pkgid",
    "publish",
    "read-manifest",
    "remove",
    "report",
    "run",
    "rustc",
    "rustdoc",
    "search",
    "test",
    "tree",
    "uninstall",
    "update",
    "vendor",
    "verify-project",
    "version",
    "yank",
];

pub(crate) fn is_in_cargo_list(stdout: &str, name: &str) -> bool {
    let cargo_name = name.strip_prefix("cargo-").unwrap_or(name);
    // TASK-0526: an empty short-name (caller passed "cargo-" or "") would
    // match any line whose first whitespace token is empty, i.e. any line
    // with leading whitespace. Reject early so a malformed [tools] entry
    // can't produce a false positive.
    if cargo_name.is_empty() {
        return false;
    }
    // PATTERN-1 / TASK-1101: see `CARGO_BUILTIN_SUBCOMMANDS` doc — a
    // `tools.toml` entry whose short-name collides with a cargo built-in
    // must NOT be treated as installed via the membership check, because
    // the built-in is shipped inside the cargo binary and was never
    // `cargo install`-ed. Returning false here pushes the caller through
    // the `check_binary_installed_with` PATH fallback, which only fires for
    // an actual `cargo-<name>` executable on `$PATH`.
    if CARGO_BUILTIN_SUBCOMMANDS.contains(&cargo_name) {
        return false;
    }
    stdout.lines().any(|line| {
        line.split_whitespace()
            .next()
            .is_some_and(|cmd| cmd == cargo_name)
    })
}

/// SEC-13: walk `PATH` directly instead of shelling out to `which`. The
/// previous implementation paid spawn overhead for every probe (called per
/// tool per status check) and silently returned `false` on Windows, where
/// `which` is not a built-in. Walking `PATH` ourselves is portable, faster,
/// and avoids invoking another binary at all.
///
/// On Windows we also try the executable suffixes listed in `PATHEXT`
/// (defaulting to `.COM;.EXE;.BAT;.CMD`); on Unix we only check the bare
/// name and rely on the executable bit.
pub fn check_binary_installed(name: &str) -> bool {
    find_on_path(name).is_some()
}

pub(crate) fn find_on_path(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    find_on_path_in(name, &path)
}

pub(crate) fn find_on_path_in(
    name: &str,
    path_var: &std::ffi::OsStr,
) -> Option<std::path::PathBuf> {
    for dir in std::env::split_paths(path_var) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(name);
        match check_executable(&candidate) {
            ExecCheck::Yes => return Some(candidate),
            ExecCheck::BrokenSymlink => {
                tracing::warn!(
                    path = %candidate.display(),
                    "PATH entry is a broken symlink; skipping"
                );
            }
            ExecCheck::NotExec | ExecCheck::Missing => {}
        }
        if cfg!(windows) {
            for ext in pathext_suffixes() {
                let mut with_ext = candidate.clone().into_os_string();
                with_ext.push(&ext);
                let p = std::path::PathBuf::from(with_ext);
                if matches!(check_executable(&p), ExecCheck::Yes) {
                    return Some(p);
                }
            }
        }
    }
    None
}

#[cfg(windows)]
fn pathext_suffixes() -> Vec<std::ffi::OsString> {
    let raw = std::env::var_os("PATHEXT")
        .unwrap_or_else(|| std::ffi::OsString::from(".COM;.EXE;.BAT;.CMD"));
    std::env::split_paths(&raw)
        .map(std::path::PathBuf::into_os_string)
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(not(windows))]
fn pathext_suffixes() -> Vec<std::ffi::OsString> {
    Vec::new()
}

/// ERR-1 / TASK-0607: distinguish between "metadata succeeded and target is
/// not executable", "metadata succeeded but the path is a broken symlink"
/// (`symlink_metadata` sees the symlink, `metadata` follows it and fails),
/// and "missing / metadata error". Lets the PATH walk keep looking while
/// surfacing the broken-symlink case to operators.
enum ExecCheck {
    Yes,
    NotExec,
    BrokenSymlink,
    Missing,
}

#[cfg(unix)]
fn check_executable(path: &std::path::Path) -> ExecCheck {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(m) if m.is_file() && m.permissions().mode() & 0o111 != 0 => ExecCheck::Yes,
        Ok(_) => ExecCheck::NotExec,
        Err(_) => match std::fs::symlink_metadata(path) {
            Ok(m) if m.file_type().is_symlink() => ExecCheck::BrokenSymlink,
            _ => ExecCheck::Missing,
        },
    }
}

#[cfg(not(unix))]
fn check_executable(path: &std::path::Path) -> ExecCheck {
    // On Windows, file existence + extension match (caller's PATHEXT loop)
    // is the standard heuristic; the OS does not surface an executable bit
    // through `Permissions`. Match the behaviour of `which` and similar
    // tooling.
    match std::fs::metadata(path) {
        Ok(m) if m.is_file() => ExecCheck::Yes,
        Ok(_) => ExecCheck::NotExec,
        Err(_) => match std::fs::symlink_metadata(path) {
            Ok(m) if m.file_type().is_symlink() => ExecCheck::BrokenSymlink,
            _ => ExecCheck::Missing,
        },
    }
}

pub fn check_rustup_component_installed(component: &str) -> bool {
    let mut cmd = Command::new(resolve_rustup_bin());
    cmd.args(["component", "list", "--installed"]);
    let output = match run_probe_with_timeout(&mut cmd, "rustup component list --installed") {
        Some(o) => o,
        None => return false,
    };

    if !output.status.success() {
        // ERR-1 / TASK-1032: see `check_cargo_tool_installed` — `format_error_tail`
        // is byte-bounded and char-boundary safe, unlike `chars().take(200)`.
        let stderr_snippet = format_error_tail(&output.stderr, 10);
        tracing::warn!(
            component = component,
            code = ?output.status.code(),
            stderr = ?stderr_snippet,
            "rustup component list exited non-zero; reporting component as not installed"
        );
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    is_component_in_list(&stdout, component)
}

/// `-{arch}-` patterns used to find the component-name / target-triple
/// boundary in lines like `clippy-preview-aarch64-apple-darwin`. Stored as
/// `&'static str` constants so prefix matching does not allocate per line.
/// Open-ended prefix matching would also hit unrelated siblings like a
/// hypothetical `clippy-foo-...`.
const RUSTUP_TARGET_ARCH_PATTERNS: &[&str] = &[
    "-aarch64-",
    "-arm-",
    "-armv6-",
    "-armv7-",
    "-armv7a-",
    "-asmjs-",
    "-i586-",
    "-i686-",
    "-loongarch64-",
    "-mips-",
    "-mips64-",
    "-mips64el-",
    "-mipsel-",
    "-nvptx64-",
    "-powerpc-",
    "-powerpc64-",
    "-powerpc64le-",
    "-riscv32-",
    "-riscv64-",
    "-s390x-",
    "-sparc-",
    "-sparc64-",
    "-thumbv6m-",
    "-thumbv7em-",
    "-thumbv7m-",
    "-thumbv7neon-",
    "-thumbv8m.base-",
    "-thumbv8m.main-",
    "-wasm32-",
    "-wasm64-",
    "-x86_64-",
];

fn strip_target_triple(line: &str) -> &str {
    for pat in RUSTUP_TARGET_ARCH_PATTERNS {
        if let Some(idx) = line.find(pat) {
            return &line[..idx];
        }
    }
    line
}

pub(crate) fn is_component_in_list(stdout: &str, component: &str) -> bool {
    let base = component.strip_suffix("-preview").unwrap_or(component);
    stdout.lines().any(|raw| {
        let line = raw.trim();
        // Drop trailing " (installed)" / " (default)" annotations and the
        // target triple, then compare exactly against base or base-preview.
        let head = line.split_whitespace().next().unwrap_or(line);
        let stripped = strip_target_triple(head);
        stripped == base || stripped.strip_suffix("-preview") == Some(base)
    })
}

pub fn check_tool_status(name: &str, spec: &ToolSpec) -> ToolStatus {
    check_tool_status_with(name, spec, None, None, None)
}

/// Variant of [`check_tool_status`] that reuses precomputed `cargo --list`,
/// `rustup component list --installed`, and `$PATH` index outputs, so the
/// caller can resolve them once per probe sweep and amortise the spawn /
/// directory-walk cost across all entries.
///
/// `cargo_list` is consulted only for `ToolSource::Cargo` specs; `rustup_components`
/// only for specs that name a `rustup_component`. `path_index` (PERF-3 /
/// TASK-1046) replaces the per-tool `$PATH` walk performed by
/// [`check_binary_installed`] when the cargo-list fallback path or a
/// `ToolSource::System` spec needs to confirm a binary; `None` runs the
/// per-tool walk as before.
pub fn check_tool_status_with(
    name: &str,
    spec: &ToolSpec,
    cargo_list: Option<&str>,
    rustup_components: Option<&str>,
    path_index: Option<&PathIndex>,
) -> ToolStatus {
    if let Some(component) = spec.rustup_component() {
        let installed = match rustup_components {
            Some(s) => is_component_in_list(s, component),
            None => check_rustup_component_installed(component),
        };
        if !installed {
            return ToolStatus::NotInstalled;
        }
    }

    let is_installed = match spec.source() {
        ToolSource::Cargo => match cargo_list {
            Some(s) => is_in_cargo_list(s, name) || check_binary_installed_with(name, path_index),
            None => check_cargo_tool_installed(name),
        },
        ToolSource::System => check_binary_installed_with(name, path_index),
    };

    if is_installed {
        ToolStatus::Installed
    } else {
        ToolStatus::NotInstalled
    }
}

/// Capture the raw stdout of `cargo --list` once. Returns `None` if the spawn or
/// non-zero exit prevents reuse; callers fall back to per-tool spawns.
pub fn capture_cargo_list() -> Option<String> {
    let mut cmd = Command::new(resolve_cargo_bin());
    cmd.args(["--list"]);
    let output = run_probe_with_timeout(&mut cmd, "cargo --list")?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Capture the raw stdout of `rustup component list --installed` once.
pub fn capture_rustup_components() -> Option<String> {
    let mut cmd = Command::new(resolve_rustup_bin());
    cmd.args(["component", "list", "--installed"]);
    let output = run_probe_with_timeout(&mut cmd, "rustup component list --installed")?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(all(test, windows))]
mod path_index_case_tests {
    use super::*;

    /// CONC-7 / TASK-1249: `is_in_path_index` must match a probe name
    /// against an on-disk basename that differs only by case under
    /// Windows. The index normalises basenames to lowercase at insert
    /// AND lookup time, so `tokei` finds `Tokei.EXE`, `tokei.exe`, etc.
    #[test]
    fn windows_lookup_matches_mixed_case_basename() {
        let mut idx: PathIndex = HashSet::new();
        idx.insert(index_key(OsString::from("Tokei.EXE")));
        assert!(
            is_in_path_index(&idx, "tokei"),
            "Windows lookup must be case-insensitive in both directions"
        );
        idx.insert(index_key(OsString::from("ripgrep.exe")));
        assert!(is_in_path_index(&idx, "RipGrep"));
    }
}

#[cfg(all(test, unix))]
mod path_index_unix_tests {
    use super::*;

    /// CONC-7 / TASK-1249: under Unix the index keeps verbatim filenames —
    /// POSIX filesystems are case-sensitive and a probe for `tokei` must
    /// NOT match an on-disk `Tokei` (which would be a different binary).
    #[test]
    fn unix_lookup_remains_case_sensitive() {
        let mut idx: PathIndex = HashSet::new();
        idx.insert(index_key(OsString::from("Tokei")));
        assert!(
            !is_in_path_index(&idx, "tokei"),
            "Unix lookup must stay case-sensitive: `tokei` and `Tokei` are distinct"
        );
        assert!(is_in_path_index(&idx, "Tokei"));
    }
}

#[cfg(test)]
mod probe_log_format_tests {
    use ops_core::output::format_error_tail;

    /// ERR-7 / TASK-0979: subprocess stderr snippets flow through the `?`
    /// formatter so cargo/rustup ANSI escapes or registry-served diagnostics
    /// containing newlines cannot forge log records or repaint the operator
    /// terminal. Pin the value-level escape without requiring a tracing-
    /// subscriber dev-dep.
    #[test]
    fn stderr_snippet_debug_escapes_control_characters() {
        let snippet = "warn\nerror: \u{1b}[31mhi\u{1b}[0m";
        let rendered = format!("{snippet:?}");
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

    /// ERR-1 / TASK-1032: the byte-bounded helper bounds the snippet by line
    /// count rather than Unicode scalar values and always returns well-formed
    /// UTF-8 even for stderr containing CJK/RTL characters or invalid byte
    /// sequences. Regression guard for the `chars().take(200)` pattern that
    /// could cut mid-grapheme and bloat logs in non-en_US locales.
    #[test]
    fn stderr_snippet_handles_non_ascii_without_mid_grapheme_cut() {
        // CJK characters: 3 bytes each in UTF-8. 10 lines of 5 chars.
        let mut stderr = Vec::new();
        for i in 0..50 {
            stderr.extend_from_slice(format!("行{i}は失敗\n").as_bytes());
        }
        let snippet = format_error_tail(&stderr, 10);
        // Must be valid UTF-8 (String guarantees this; assert no replacement
        // characters were introduced by from_utf8_lossy).
        assert!(!snippet.contains('\u{FFFD}'), "no replacement chars");
        // Bounded at 10 lines — last line is "行49は失敗".
        assert_eq!(snippet.lines().count(), 10);
        assert!(snippet.ends_with("行49は失敗"));
        // The previous `chars().take(200)` cap could leave a malformed
        // grapheme; assert the snippet ends on a complete CJK character.
        assert!(snippet.is_char_boundary(snippet.len()));
    }

    /// ERR-1 / TASK-1032 AC#2: the snippet stays bounded for pathological
    /// stderr payloads. Replaces the prior 200-char cap test; line-bounded
    /// truncation prevents log-volume blowups regardless of locale.
    #[test]
    fn stderr_snippet_caps_line_count() {
        let stderr = "x\n".repeat(10_000);
        let snippet = format_error_tail(stderr.as_bytes(), 10);
        assert_eq!(snippet.lines().count(), 10);
    }
}

#[cfg(all(test, unix))]
mod probe_timeout_tests {
    use super::*;

    /// ASYNC-6 / TASK-0914: prove that `run_probe_with_timeout` actually
    /// honours the deadline rather than blocking on the child. Spawn a
    /// `sh -c "sleep 30"` under a 1s deadline and assert the helper
    /// returns `None` well under the wall-clock limit. A regression that
    /// drops the timeout (e.g. reverts a probe to `cmd.output()`) hangs
    /// this test until the surrounding `cargo test` timer fires, which
    /// is exactly the wedge this fix prevents in production.
    #[test]
    fn timeout_returns_none_quickly() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "sleep 30"]);
        let start = std::time::Instant::now();
        let prev = std::env::var_os(ops_core::subprocess::TIMEOUT_ENV);
        // SAFETY: tests run concurrently — narrow the window by restoring
        // the previous value immediately after the call.
        // SAFETY: Rust 2024 marks set_var/remove_var as unsafe due to other-thread races.
        unsafe { std::env::set_var(ops_core::subprocess::TIMEOUT_ENV, "1") };
        let result = run_probe_with_timeout(&mut cmd, "sleep test");
        match prev {
            Some(v) => unsafe { std::env::set_var(ops_core::subprocess::TIMEOUT_ENV, v) },
            None => unsafe { std::env::remove_var(ops_core::subprocess::TIMEOUT_ENV) },
        }
        assert!(result.is_none(), "timeout must surface as None");
        assert!(
            start.elapsed() < std::time::Duration::from_secs(10),
            "must not hang past the deadline; elapsed = {:?}",
            start.elapsed()
        );
    }
}
