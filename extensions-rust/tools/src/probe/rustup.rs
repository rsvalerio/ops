//! `rustup` toolchain / component probes.

use ops_core::output::format_error_tail;
use ops_core::subprocess::resolve_rustup_bin;
use std::process::Command;

use super::timeout::run_probe_with_timeout;

pub fn get_active_toolchain() -> Option<String> {
    // `--quiet` is rustup's global flag, not a subcommand option, so it
    // appears before `show`. ASYNC-6 / TASK-0914: capped at PROBE_TIMEOUT.
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
/// PATTERN-1 / TASK-1078: only the rustup diagnostic prefixes
/// (`error:`, `warning:`, `info:`, `note:`) cause rejection. Match the
/// prefix on the full first whitespace-bounded segment, not as a substring.
pub(crate) fn parse_active_toolchain(stdout: &str) -> Option<String> {
    const RUSTUP_DIAGNOSTIC_PREFIXES: &[&str] = &["error:", "warning:", "info:", "note:"];

    let line = stdout.lines().map(str::trim).find(|l| !l.is_empty())?;
    let token = line.split_whitespace().next()?;
    if RUSTUP_DIAGNOSTIC_PREFIXES.contains(&token) {
        return None;
    }
    Some(token.to_string())
}

pub fn check_rustup_component_installed(component: &str) -> bool {
    let mut cmd = Command::new(resolve_rustup_bin());
    cmd.args(["component", "list", "--installed"]);
    let output = match run_probe_with_timeout(&mut cmd, "rustup component list --installed") {
        Some(o) => o,
        None => return false,
    };

    if !output.status.success() {
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
/// boundary in lines like `clippy-preview-aarch64-apple-darwin`.
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
        let head = line.split_whitespace().next().unwrap_or(line);
        let stripped = strip_target_triple(head);
        stripped == base || stripped.strip_suffix("-preview") == Some(base)
    })
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
