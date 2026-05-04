//! Detect whether tools/components are installed on the active toolchain.

use ops_core::config::tools::{ToolSource, ToolSpec};
use ops_core::subprocess::{
    default_timeout, resolve_cargo_bin, resolve_rustup_bin, run_with_timeout, RunError,
};
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
/// the failure to `ToolStatus::Unknown` / "not installed" without
/// duplicating the logging pattern at every call site.
fn run_probe_with_timeout(cmd: &mut Command, label: &'static str) -> Option<std::process::Output> {
    match run_with_timeout(cmd, default_timeout(PROBE_TIMEOUT), label) {
        Ok(out) => Some(out),
        Err(RunError::Timeout(e)) => {
            tracing::warn!(
                label,
                timeout_secs = e.timeout.as_secs(),
                "ASYNC-6 / TASK-0914: probe timed out; reporting unknown/not-installed"
            );
            None
        }
        Err(RunError::Io(e)) => {
            tracing::warn!(
                label,
                error = %e,
                "probe spawn failed; reporting unknown/not-installed"
            );
            None
        }
        Err(other) => {
            tracing::warn!(
                label,
                error = %other,
                "probe failed with unrecognized error variant; reporting unknown/not-installed"
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
/// toolchain identifier. A valid toolchain token must not contain `:`.
pub(crate) fn parse_active_toolchain(stdout: &str) -> Option<String> {
    let line = stdout.lines().map(str::trim).find(|l| !l.is_empty())?;
    let token = line.split_whitespace().next()?;
    // Reject rustup diagnostic prefixes and any token containing `:` (e.g.
    // "error:", "info:", "no active toolchain configured").
    if token.contains(':') {
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
        let stderr_tail = String::from_utf8_lossy(&output.stderr);
        let stderr_snippet = stderr_tail.chars().take(200).collect::<String>();
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

pub(crate) fn is_in_cargo_list(stdout: &str, name: &str) -> bool {
    let cargo_name = name.strip_prefix("cargo-").unwrap_or(name);
    // TASK-0526: an empty short-name (caller passed "cargo-" or "") would
    // match any line whose first whitespace token is empty, i.e. any line
    // with leading whitespace. Reject early so a malformed [tools] entry
    // can't produce a false positive.
    if cargo_name.is_empty() {
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
        let stderr_tail = String::from_utf8_lossy(&output.stderr);
        let stderr_snippet = stderr_tail.chars().take(200).collect::<String>();
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
    check_tool_status_with(name, spec, None, None)
}

/// Variant of [`check_tool_status`] that reuses precomputed `cargo --list` and
/// `rustup component list --installed` outputs, so the caller can resolve them
/// once per probe sweep and amortise the spawn cost across all entries.
///
/// `cargo_list` is consulted only for `ToolSource::Cargo` specs; `rustup_components`
/// only for specs that name a `rustup_component`. Falling back to `None` runs the
/// per-tool subprocess as before.
pub fn check_tool_status_with(
    name: &str,
    spec: &ToolSpec,
    cargo_list: Option<&str>,
    rustup_components: Option<&str>,
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
            Some(s) => is_in_cargo_list(s, name) || check_binary_installed(name),
            None => check_cargo_tool_installed(name),
        },
        ToolSource::System => check_binary_installed(name),
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

#[cfg(test)]
mod probe_log_format_tests {
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

    /// ERR-7 / TASK-0979 AC#2: the snippet stays bounded at 200 chars so a
    /// pathological stderr payload cannot blow up log volume.
    #[test]
    fn stderr_snippet_take_200_caps_length() {
        let stderr = "x".repeat(10_000);
        let snippet = stderr.chars().take(200).collect::<String>();
        assert_eq!(snippet.len(), 200);
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
