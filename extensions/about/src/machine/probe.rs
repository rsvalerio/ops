//! OS and toolchain probes (`ps`, load average, mounts, `df`, `rustc`,
//! `cargo locate-project`, `sccache`), each with a pure parser so the
//! parsing is tested on every host.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use ops_core::subprocess::run_with_timeout;

use super::cfg_expr::parse_rustc_cfg;
use super::settings::HostTarget;
use super::{BuildProcess, FsReport};

/// Deadline for each small probe subprocess (`ps`, `rustc -vV`, `df`,
/// `mount`, `sysctl`, `sccache --show-stats`).
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Process names counted as competing build work.
const BUILD_PROCESS_NAMES: &[&str] = &["cargo", "rustc", "cargo-nextest"];

/// Parse `rustc -vV` output for the `host:` line.
#[must_use]
pub fn parse_rustc_host(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix("host:"))
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
}

/// Parse `cargo locate-project --workspace --message-format plain` output
/// (the workspace root's `Cargo.toml` path) into the workspace root dir.
#[must_use]
pub fn parse_locate_project(stdout: &str) -> Option<PathBuf> {
    let manifest = stdout.lines().next()?.trim();
    if manifest.is_empty() {
        return None;
    }
    Path::new(manifest).parent().map(Path::to_path_buf)
}

/// Parse Linux `/proc/loadavg` (`0.52 0.58 0.59 1/1234 5678`).
#[must_use]
pub fn parse_proc_loadavg(text: &str) -> Option<[f64; 3]> {
    parse_three_floats(text.split_whitespace())
}

/// Parse macOS `sysctl -n vm.loadavg` (`{ 1.23 1.45 1.67 }`).
#[must_use]
pub fn parse_sysctl_loadavg(text: &str) -> Option<[f64; 3]> {
    parse_three_floats(text.split_whitespace().filter(|t| *t != "{" && *t != "}"))
}

fn parse_three_floats<'a>(mut tokens: impl Iterator<Item = &'a str>) -> Option<[f64; 3]> {
    let mut next = || tokens.next()?.parse::<f64>().ok();
    Some([next()?, next()?, next()?])
}

/// Parse `ps -axo pid=,comm=` into the build processes it lists, skipping
/// the pids in `exclude`. `comm` may be a full path (macOS); only its
/// final component is matched.
#[must_use]
pub fn parse_ps(stdout: &str, exclude: &[u32]) -> Vec<BuildProcess> {
    stdout
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let (pid, comm) = line.split_once(char::is_whitespace)?;
            let pid: u32 = pid.parse().ok()?;
            let name = Path::new(comm.trim()).file_name()?.to_str()?.to_string();
            (BUILD_PROCESS_NAMES.contains(&name.as_str()) && !exclude.contains(&pid))
                .then_some(BuildProcess { pid, name })
        })
        .collect()
}

/// Parse Linux `/proc/self/mounts` into `(mount point, fs type)` pairs,
/// decoding the octal escapes (`\040` for a space) the kernel writes.
#[must_use]
pub fn parse_proc_mounts(text: &str) -> Vec<(PathBuf, String)> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let _device = fields.next()?;
            let mount_point = fields.next()?;
            let fs_type = fields.next()?;
            Some((PathBuf::from(unescape_octal(mount_point)), fs_type.to_string()))
        })
        .collect()
}

fn unescape_octal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find('\\') {
        let (head, tail) = rest.split_at(pos);
        out.push_str(head);
        let code = tail.get(1..4).and_then(|d| u8::from_str_radix(d, 8).ok());
        if let Some(byte) = code {
            out.push(char::from(byte));
            rest = tail.get(4..).unwrap_or("");
        } else {
            out.push('\\');
            rest = tail.get(1..).unwrap_or("");
        }
    }
    out.push_str(rest);
    out
}

/// Parse BSD/macOS `mount` output
/// (`/dev/disk3s1s1 on / (apfs, sealed, local, read-only, journaled)`).
#[must_use]
pub fn parse_bsd_mount(text: &str) -> Vec<(PathBuf, String)> {
    text.lines()
        .filter_map(|line| {
            let (_, rest) = line.split_once(" on ")?;
            let (mount_point, opts) = rest.rsplit_once(" (")?;
            let fs_type = opts.split([',', ')']).next()?.trim();
            Some((PathBuf::from(mount_point), fs_type.to_string()))
        })
        .collect()
}

/// The fs type of the mount with the longest mount point containing `path`.
#[must_use]
pub fn fs_type_for(mounts: &[(PathBuf, String)], path: &Path) -> Option<String> {
    mounts
        .iter()
        .filter(|(mp, _)| path.starts_with(mp))
        .max_by_key(|(mp, _)| mp.components().count())
        .map(|(_, t)| t.clone())
}

/// Parse `df -Pk <path>` into `(total, available)` bytes.
#[must_use]
pub fn parse_df(stdout: &str) -> Option<(u64, u64)> {
    let line = stdout.lines().nth(1)?;
    let mut fields = line.split_whitespace();
    let total_kib: u64 = fields.nth(1)?.parse().ok()?;
    let avail_kib: u64 = fields.nth(1)?.parse().ok()?;
    Some((total_kib.checked_mul(1024)?, avail_kib.checked_mul(1024)?))
}

/// Run a probe command, returning stdout on success; failures log and
/// yield `None`.
pub(super) fn probe(cmd: &mut Command, label: &str) -> Option<String> {
    match run_with_timeout(cmd, PROBE_TIMEOUT, label) {
        Ok(out) if out.status.success() => Some(String::from_utf8_lossy(&out.stdout).into_owned()),
        Ok(out) => {
            tracing::debug!(label, status = %out.status, "about/machine: probe exited non-zero");
            None
        }
        Err(e) => {
            tracing::debug!(label, error = ?e.to_string(), "about/machine: probe failed");
            None
        }
    }
}

pub(super) fn load_average() -> Option<[f64; 3]> {
    if cfg!(target_os = "macos") {
        probe(Command::new("sysctl").args(["-n", "vm.loadavg"]), "sysctl vm.loadavg")
            .and_then(|t| parse_sysctl_loadavg(&t))
    } else {
        std::fs::read_to_string("/proc/loadavg").ok().and_then(|t| parse_proc_loadavg(&t))
    }
}

pub(super) fn mount_table() -> Vec<(PathBuf, String)> {
    if cfg!(target_os = "macos") {
        probe(&mut Command::new("mount"), "mount").map(|t| parse_bsd_mount(&t)).unwrap_or_default()
    } else {
        std::fs::read_to_string("/proc/self/mounts")
            .map(|t| parse_proc_mounts(&t))
            .unwrap_or_default()
    }
}

/// The nearest existing ancestor of `path`, canonicalized.
fn existing_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors().find_map(|a| std::fs::canonicalize(a).ok())
}

pub(super) fn fs_report(path: &Path, mounts: &[(PathBuf, String)]) -> Option<FsReport> {
    let existing = existing_ancestor(path)?;
    let fs_type = fs_type_for(mounts, &existing);
    let sizes =
        probe(Command::new("df").arg("-Pk").arg(&existing), "df").and_then(|t| parse_df(&t));
    Some(FsReport {
        path: path.display().to_string(),
        tmpfs: fs_type.as_deref() == Some("tmpfs"),
        fs_type,
        total_bytes: sizes.map(|(t, _)| t),
        available_bytes: sizes.map(|(_, a)| a),
    })
}

pub(super) fn is_sccache(wrapper: &str) -> bool {
    Path::new(wrapper).file_stem().is_some_and(|s| s == "sccache")
}

/// Stats of the `sccache` on PATH.
pub(super) fn sccache_stats() -> Option<serde_json::Value> {
    let text = probe(
        Command::new("sccache").args(["--show-stats", "--stats-format=json"]),
        "sccache --show-stats",
    )?;
    serde_json::from_str(&text)
        .map_err(
            |e| tracing::warn!(error = ?e.to_string(), "about/machine: sccache stats are not JSON"),
        )
        .ok()
}

/// Probe `rustc` for the host triple (`-vV`) and its cfg atoms
/// (`--print cfg`); both probes degrade to empty on failure.
///
/// Runs in the checkout on purpose. When `rustc` is a rustup proxy, a
/// checked-in `rust-toolchain.toml` picks the toolchain — possibly a custom
/// `path` toolchain the checkout supplies. That is accepted, not neutralized:
/// the report describes the toolchain a build here would use, so pinning
/// `RUSTUP_TOOLCHAIN` would report the wrong host and cfg; and `ops` already
/// trusts the checkout's toolchain selection wherever it runs cargo there
/// (the Rust stack's `metadata` data provider runs `cargo metadata` there). What
/// the page refuses is narrower: running a program path named by the
/// checkout's cargo config (see the sccache probe in `collect_machine_report`).
pub(super) fn probe_host_target(rustc: &std::ffi::OsStr) -> HostTarget {
    HostTarget {
        triple: probe(Command::new(rustc).arg("-vV"), "rustc -vV")
            .and_then(|t| parse_rustc_host(&t)),
        cfg: probe(Command::new(rustc).args(["--print", "cfg"]), "rustc --print cfg")
            .map(|t| parse_rustc_cfg(&t))
            .unwrap_or_default(),
    }
}

/// The workspace root of the build at `cwd`, from
/// `cargo locate-project --workspace`.
pub(super) fn locate_workspace_root(cargo_bin: &std::ffi::OsStr, cwd: &Path) -> Option<PathBuf> {
    probe(
        Command::new(cargo_bin)
            .args(["locate-project", "--workspace", "--message-format", "plain"])
            .current_dir(cwd),
        "cargo locate-project --workspace",
    )
    .and_then(|t| parse_locate_project(&t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_load_average_on_both_platforms() {
        assert_eq!(parse_proc_loadavg("0.52 0.58 0.59 1/1234 5678\n"), Some([0.52, 0.58, 0.59]));
        assert_eq!(parse_sysctl_loadavg("{ 1.23 1.45 1.67 }\n"), Some([1.23, 1.45, 1.67]));
        assert_eq!(parse_proc_loadavg("garbage"), None);
    }

    #[test]
    fn parse_ps_keeps_build_processes_and_drops_excluded_pids() {
        let out = "    1 systemd\n  200 cargo\n  201 /Users/me/.rustup/toolchains/x/bin/rustc\n  202 cargo-nextest\n  300 cargo\n  400 cargo-watch\n";
        let procs = parse_ps(out, &[300]);
        let names: Vec<(u32, &str)> = procs.iter().map(|p| (p.pid, p.name.as_str())).collect();
        assert_eq!(names, [(200, "cargo"), (201, "rustc"), (202, "cargo-nextest")]);
    }

    #[test]
    fn mount_tables_parse_and_longest_prefix_wins() {
        let linux = "/dev/nvme0n1p2 / ext4 rw 0 0\ntmpfs /tmp tmpfs rw 0 0\n/dev/x /mnt/my\\040disk xfs rw 0 0\n";
        let mounts = parse_proc_mounts(linux);
        assert_eq!(fs_type_for(&mounts, Path::new("/tmp/build")), Some("tmpfs".to_string()));
        assert_eq!(fs_type_for(&mounts, Path::new("/home/u")), Some("ext4".to_string()));
        assert_eq!(fs_type_for(&mounts, Path::new("/mnt/my disk/t")), Some("xfs".to_string()));
        assert_eq!(fs_type_for(&mounts, Path::new("/tmpx")), Some("ext4".to_string()));

        let mac = "/dev/disk3s1s1 on / (apfs, sealed, local, read-only, journaled)\n/dev/disk3s5 on /System/Volumes/Data (apfs, local, journaled, nobrowse)\n";
        let mounts = parse_bsd_mount(mac);
        assert_eq!(
            fs_type_for(&mounts, Path::new("/System/Volumes/Data/tmp")),
            Some("apfs".to_string())
        );
    }

    #[test]
    fn parse_df_reads_posix_columns() {
        let out = "Filesystem 1024-blocks Used Available Capacity Mounted on\ntmpfs 16384 1024 15360 7% /tmp\n";
        assert_eq!(parse_df(out), Some((16384 * 1024, 15360 * 1024)));
        assert_eq!(parse_df("header only\n"), None);
    }

    #[test]
    fn parse_rustc_host_reads_the_host_line() {
        let out = "rustc 1.90.0 (abc 2025-01-01)\nbinary: rustc\nhost: aarch64-apple-darwin\nrelease: 1.90.0\n";
        assert_eq!(parse_rustc_host(out), Some("aarch64-apple-darwin".to_string()));
    }

    #[test]
    fn unescape_octal_decodes_kernel_escapes() {
        assert_eq!(unescape_octal("/mnt/a\\040b"), "/mnt/a b");
        assert_eq!(unescape_octal("/plain"), "/plain");
        assert_eq!(unescape_octal("/trail\\"), "/trail\\");
    }
}
