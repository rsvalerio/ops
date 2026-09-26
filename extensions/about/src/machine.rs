//! `about machine` subpage: the build-relevant state of this machine.
//!
//! Reports what a build timing depends on besides the code: core count and
//! load, other cargo/rustc/nextest processes competing for the CPU, the
//! effective cargo settings (`build.jobs`, rustc wrapper, target dir,
//! linker, rustflags) with the layer each came from, the filesystem behind
//! `TMPDIR` and the target dir (a tmpfs `/tmp` fails cold builds with exit
//! 101 and no compile error), and sccache counters when sccache wraps rustc.
//!
//! Every probe degrades to "unknown" rather than failing the page: a
//! missing `ps`, an unreadable mount table or a malformed cargo config
//! leaves its field `null` and logs a warning. Linux and macOS are both
//! supported; each OS-specific probe has a pure parser behind it so the
//! parsing is tested on every host.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use ops_core::subprocess::run_with_timeout;

/// Version of the `ops about machine --json` document shape.
pub const MACHINE_JSON_SCHEMA_VERSION: u32 = 1;

/// Deadline for each small probe subprocess (`ps`, `rustc -vV`, `df`,
/// `mount`, `sysctl`, `sccache --show-stats`).
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Process names counted as competing build work.
const BUILD_PROCESS_NAMES: &[&str] = &["cargo", "rustc", "cargo-nextest"];

/// The `ops about machine --json` document.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineReport {
    pub schema_version: u32,
    pub kind: &'static str,
    /// `std::env::consts::OS` of the running binary.
    pub os: &'static str,
    /// Logical cores available to this process.
    pub cores: Option<usize>,
    /// 1, 5 and 15 minute load averages.
    pub load_average: Option<[f64; 3]>,
    /// Other `cargo` / `rustc` / `cargo-nextest` processes (this process
    /// and its parent excluded).
    pub build_processes: Vec<BuildProcess>,
    pub cargo: CargoSettings,
    /// Filesystem behind `TMPDIR` (or the platform temp dir).
    pub tmpdir: Option<FsReport>,
    /// Filesystem behind the effective target dir (its nearest existing
    /// ancestor when the dir does not exist yet).
    pub target_dir: Option<FsReport>,
    /// `sccache --show-stats --stats-format=json`, verbatim, when the
    /// effective rustc wrapper is sccache.
    pub sccache: Option<serde_json::Value>,
}

/// One competing build process.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BuildProcess {
    pub pid: u32,
    pub name: String,
}

/// An effective setting and where it came from: `env:NAME`, a config file
/// path, or `default`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Setting {
    pub value: String,
    pub source: String,
}

/// Effective cargo settings across env and every config layer.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CargoSettings {
    /// Host triple from `rustc -vV`; the key for `target.<triple>.*`.
    pub host: Option<String>,
    pub jobs: Option<Setting>,
    pub rustc_wrapper: Option<Setting>,
    pub target_dir: Setting,
    pub linker: Option<Setting>,
    pub rustflags: Option<Setting>,
    /// Every config file consulted, highest precedence first.
    pub config_files: Vec<String>,
}

/// Filesystem facts for one path.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsReport {
    pub path: String,
    pub fs_type: Option<String>,
    pub tmpfs: bool,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
}

/// One parsed cargo config file.
#[derive(Debug, Clone)]
pub struct ConfigLayer {
    /// The file, as reported in `source`.
    pub path: PathBuf,
    /// The directory relative config paths resolve against: the parent of
    /// the `.cargo` directory holding the file.
    pub base: PathBuf,
    pub table: toml::Table,
}

/// Find and parse every cargo config layer for `cwd`, highest precedence first.
///
/// `cwd` and each ancestor's `.cargo/config.toml` (or legacy
/// `.cargo/config`), then `$CARGO_HOME`'s when no ancestor already
/// covered it. Unreadable or malformed files are skipped with a warning.
#[must_use]
pub fn config_layers(cwd: &Path, cargo_home: Option<&Path>) -> Vec<ConfigLayer> {
    let mut dirs: Vec<PathBuf> = cwd.ancestors().map(|a| a.join(".cargo")).collect();
    if let Some(home) = cargo_home {
        if !dirs.iter().any(|d| d == home) {
            dirs.push(home.to_path_buf());
        }
    }
    dirs.iter()
        .filter_map(|dir| {
            let path = ["config.toml", "config"]
                .iter()
                .map(|name| dir.join(name))
                .find(|p| p.is_file())?;
            let text = match std::fs::read_to_string(&path) {
                Ok(text) => text,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "about/machine: reading cargo config failed");
                    return None;
                }
            };
            let table = match text.parse::<toml::Table>() {
                Ok(table) => table,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "about/machine: parsing cargo config failed");
                    return None;
                }
            };
            let base = dir.parent().map_or_else(|| dir.clone(), Path::to_path_buf);
            Some(ConfigLayer { path, base, table })
        })
        .collect()
}

/// The first layer holding the dotted `keys`, with its value.
fn lookup<'a>(
    layers: &'a [ConfigLayer],
    keys: &[&str],
) -> Option<(&'a toml::Value, &'a ConfigLayer)> {
    layers.iter().find_map(|layer| {
        let (first, rest) = keys.split_first()?;
        let mut value = layer.table.get(*first)?;
        for key in rest {
            value = value.as_table()?.get(*key)?;
        }
        Some((value, layer))
    })
}

/// Render a config value the way it would be passed on: strings as-is,
/// arrays space-joined, anything else in TOML notation.
fn render_value(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Array(items) => items.iter().map(render_value).collect::<Vec<_>>().join(" "),
        other => other.to_string(),
    }
}

fn from_env(env: &dyn Fn(&str) -> Option<String>, names: &[&str]) -> Option<Setting> {
    names.iter().find_map(|name| {
        env(name).filter(|v| !v.is_empty()).map(|value| Setting {
            value,
            source: format!("env:{name}"),
        })
    })
}

fn from_config(layers: &[ConfigLayer], keys: &[&str]) -> Option<Setting> {
    lookup(layers, keys).map(|(value, layer)| Setting {
        value: render_value(value),
        source: layer.path.display().to_string(),
    })
}

/// `x86_64-unknown-linux-gnu` → `X86_64_UNKNOWN_LINUX_GNU`, cargo's env
/// spelling of a target triple.
fn triple_env_key(triple: &str) -> String {
    triple
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// Resolve the effective cargo settings from env and config layers.
///
/// Follows cargo's precedence (env over config for the same key;
/// `CARGO_ENCODED_RUSTFLAGS` > `RUSTFLAGS` > `target.<host>.rustflags` >
/// `build.rustflags` for rustflags). `target.'cfg(..)'` tables are not
/// evaluated.
#[must_use]
pub fn resolve_cargo_settings(
    layers: &[ConfigLayer],
    env: &dyn Fn(&str) -> Option<String>,
    cwd: &Path,
    host: Option<String>,
) -> CargoSettings {
    let jobs =
        from_env(env, &["CARGO_BUILD_JOBS"]).or_else(|| from_config(layers, &["build", "jobs"]));
    let rustc_wrapper = from_env(env, &["RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WRAPPER"])
        .or_else(|| from_config(layers, &["build", "rustc-wrapper"]));
    let target_dir = from_env(env, &["CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR"])
        .map(|s| Setting {
            value: cwd.join(&s.value).display().to_string(),
            source: s.source,
        })
        .or_else(|| {
            lookup(layers, &["build", "target-dir"]).map(|(value, layer)| Setting {
                value: layer.base.join(render_value(value)).display().to_string(),
                source: layer.path.display().to_string(),
            })
        })
        .unwrap_or_else(|| Setting {
            value: cwd.join("target").display().to_string(),
            source: "default".to_string(),
        });
    let (linker, target_rustflags) = host.as_deref().map_or((None, None), |triple| {
        let key = triple_env_key(triple);
        let linker = from_env(env, &[&format!("CARGO_TARGET_{key}_LINKER")])
            .or_else(|| from_config(layers, &["target", triple, "linker"]));
        let flags = from_env(env, &[&format!("CARGO_TARGET_{key}_RUSTFLAGS")])
            .or_else(|| from_config(layers, &["target", triple, "rustflags"]));
        (linker, flags)
    });
    let rustflags = env("CARGO_ENCODED_RUSTFLAGS")
        .filter(|v| !v.is_empty())
        .map(|v| Setting {
            value: v.split('\x1f').collect::<Vec<_>>().join(" "),
            source: "env:CARGO_ENCODED_RUSTFLAGS".to_string(),
        })
        .or_else(|| from_env(env, &["RUSTFLAGS"]))
        .or(target_rustflags)
        .or_else(|| from_env(env, &["CARGO_BUILD_RUSTFLAGS"]))
        .or_else(|| from_config(layers, &["build", "rustflags"]));
    CargoSettings {
        host,
        jobs,
        rustc_wrapper,
        target_dir,
        linker,
        rustflags,
        config_files: layers
            .iter()
            .map(|l| l.path.display().to_string())
            .collect(),
    }
}

/// Parse `rustc -vV` output for the `host:` line.
#[must_use]
pub fn parse_rustc_host(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix("host:"))
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
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
            Some((
                PathBuf::from(unescape_octal(mount_point)),
                fs_type.to_string(),
            ))
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
fn probe(cmd: &mut Command, label: &str) -> Option<String> {
    match run_with_timeout(cmd, PROBE_TIMEOUT, label) {
        Ok(out) if out.status.success() => Some(String::from_utf8_lossy(&out.stdout).into_owned()),
        Ok(out) => {
            tracing::debug!(label, status = %out.status, "about/machine: probe exited non-zero");
            None
        }
        Err(e) => {
            tracing::debug!(label, error = %e, "about/machine: probe failed");
            None
        }
    }
}

fn load_average() -> Option<[f64; 3]> {
    if cfg!(target_os = "macos") {
        probe(
            Command::new("sysctl").args(["-n", "vm.loadavg"]),
            "sysctl vm.loadavg",
        )
        .and_then(|t| parse_sysctl_loadavg(&t))
    } else {
        std::fs::read_to_string("/proc/loadavg")
            .ok()
            .and_then(|t| parse_proc_loadavg(&t))
    }
}

fn mount_table() -> Vec<(PathBuf, String)> {
    if cfg!(target_os = "macos") {
        probe(&mut Command::new("mount"), "mount")
            .map(|t| parse_bsd_mount(&t))
            .unwrap_or_default()
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

fn fs_report(path: &Path, mounts: &[(PathBuf, String)]) -> Option<FsReport> {
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

fn is_sccache(wrapper: &str) -> bool {
    Path::new(wrapper)
        .file_stem()
        .is_some_and(|s| s == "sccache")
}

fn sccache_stats(wrapper: &str) -> Option<serde_json::Value> {
    let text = probe(
        Command::new(wrapper).args(["--show-stats", "--stats-format=json"]),
        "sccache --show-stats",
    )?;
    serde_json::from_str(&text)
        .map_err(|e| tracing::warn!(error = %e, "about/machine: sccache stats are not JSON"))
        .ok()
}

/// Collect the machine report for a build rooted at `cwd`.
#[must_use]
pub fn collect_machine_report(cwd: &Path) -> MachineReport {
    let env = |name: &str| std::env::var(name).ok();
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")));
    let layers = config_layers(cwd, cargo_home.as_deref());
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let host =
        probe(Command::new(rustc).arg("-vV"), "rustc -vV").and_then(|t| parse_rustc_host(&t));
    let cargo = resolve_cargo_settings(&layers, &env, cwd, host);

    // The invoking `cargo run` (if any) is our parent, not competing work.
    let mut exclude = vec![std::process::id()];
    #[cfg(unix)]
    exclude.push(std::os::unix::process::parent_id());
    let build_processes = probe(Command::new("ps").args(["-axo", "pid=,comm="]), "ps")
        .map(|t| parse_ps(&t, &exclude))
        .unwrap_or_default();

    let mounts = mount_table();
    let tmpdir = fs_report(&std::env::temp_dir(), &mounts);
    let target_dir = fs_report(Path::new(&cargo.target_dir.value), &mounts);
    let sccache = cargo
        .rustc_wrapper
        .as_ref()
        .filter(|w| is_sccache(&w.value))
        .and_then(|w| sccache_stats(&w.value));

    MachineReport {
        schema_version: MACHINE_JSON_SCHEMA_VERSION,
        kind: "about-machine",
        os: std::env::consts::OS,
        cores: std::thread::available_parallelism()
            .ok()
            .map(std::num::NonZeroUsize::get),
        load_average: load_average(),
        build_processes,
        cargo,
        tmpdir,
        target_dir,
        sccache,
    }
}

fn setting_line(label: &str, setting: Option<&Setting>) -> String {
    setting.map_or_else(
        || format!("  {label:<15}unset"),
        |s| format!("  {label:<15}{}  [{}]", s.value, s.source),
    )
}

fn fs_line(label: &str, fs: Option<&FsReport>) -> String {
    let Some(fs) = fs else {
        return format!("  {label:<15}unknown");
    };
    const MIB: u64 = 1024 * 1024;
    let size = match (fs.total_bytes, fs.available_bytes) {
        (Some(t), Some(a)) => format!(
            ", {} MiB ({} MiB free)",
            t.checked_div(MIB).unwrap_or(0),
            a.checked_div(MIB).unwrap_or(0)
        ),
        _ => String::new(),
    };
    let warn = if fs.tmpfs { "  WARNING: tmpfs" } else { "" };
    format!(
        "  {label:<15}{} ({}{size}){warn}",
        fs.path,
        fs.fs_type.as_deref().unwrap_or("unknown fs")
    )
}

/// Render the report as plain text lines.
#[must_use]
pub fn format_machine_report(report: &MachineReport) -> Vec<String> {
    let load = report.load_average.map_or_else(
        || "unknown".to_string(),
        |[a, b, c]| format!("{a:.2} {b:.2} {c:.2}"),
    );
    let procs = if report.build_processes.is_empty() {
        "none".to_string()
    } else {
        report
            .build_processes
            .iter()
            .map(|p| format!("{} ({})", p.name, p.pid))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut lines = vec![
        String::new(),
        "  MACHINE".to_string(),
        String::new(),
        format!("  {:<15}{}", "os", report.os),
        format!(
            "  {:<15}{}",
            "cores",
            report
                .cores
                .map_or_else(|| "unknown".to_string(), |c| c.to_string())
        ),
        format!("  {:<15}{load}", "load average"),
        format!("  {:<15}{procs}", "build procs"),
        format!(
            "  {:<15}{}",
            "host",
            report.cargo.host.as_deref().unwrap_or("unknown")
        ),
        setting_line("jobs", report.cargo.jobs.as_ref()),
        setting_line("rustc wrapper", report.cargo.rustc_wrapper.as_ref()),
        setting_line("target dir", Some(&report.cargo.target_dir)),
        setting_line("linker", report.cargo.linker.as_ref()),
        setting_line("rustflags", report.cargo.rustflags.as_ref()),
        fs_line("TMPDIR", report.tmpdir.as_ref()),
        fs_line("target fs", report.target_dir.as_ref()),
    ];
    if report.sccache.is_some() {
        lines.push(format!("  {:<15}stats available via --json", "sccache"));
    }
    lines
}

/// `ops about machine [--json]`.
///
/// # Errors
///
/// If the current directory cannot be determined or writing fails.
pub fn run_about_machine(json: bool) -> anyhow::Result<()> {
    use anyhow::Context as _;
    let cwd = std::env::current_dir()
        .context("about/machine: could not determine the current directory")?;
    let report = collect_machine_report(&cwd);
    let mut out = std::io::stdout();
    write_machine_report(&mut out, &report, json)
}

/// Write `report` as JSON or text.
///
/// # Errors
///
/// If serialization or the write fails.
pub fn write_machine_report(
    writer: &mut dyn Write,
    report: &MachineReport,
    json: bool,
) -> anyhow::Result<()> {
    if json {
        crate::write_json_document(writer, report)
    } else {
        writeln!(writer, "{}", format_machine_report(report).join("\n"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(path: &str, base: &str, toml_text: &str) -> ConfigLayer {
        ConfigLayer {
            path: PathBuf::from(path),
            base: PathBuf::from(base),
            table: toml_text.parse().expect("toml"),
        }
    }

    fn no_env(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn nearest_config_layer_wins_and_names_its_source() {
        let layers = [
            layer("/w/.cargo/config.toml", "/w", "[build]\ntarget-dir = \"out\"\n"),
            layer(
                "/home/u/.cargo/config.toml",
                "/home/u",
                "[build]\njobs = 2\ntarget-dir = \"/elsewhere\"\nrustflags = [\"-C\", \"target-cpu=native\"]\n",
            ),
        ];
        let s = resolve_cargo_settings(&layers, &no_env, Path::new("/w"), None);
        assert_eq!(
            s.jobs,
            Some(Setting {
                value: "2".to_string(),
                source: "/home/u/.cargo/config.toml".to_string()
            })
        );
        assert_eq!(s.target_dir.value, "/w/out");
        assert_eq!(s.target_dir.source, "/w/.cargo/config.toml");
        assert_eq!(
            s.rustflags.map(|r| r.value),
            Some("-C target-cpu=native".to_string())
        );
        assert_eq!(s.linker, None);
    }

    #[test]
    fn env_overrides_config_and_host_keys_the_target_table() {
        let layers = [layer(
            "/w/.cargo/config.toml",
            "/w",
            "[build]\nrustc-wrapper = \"sccache\"\n[target.x86_64-unknown-linux-gnu]\nlinker = \"clang\"\nrustflags = [\"-Clink-arg=-fuse-ld=mold\"]\n",
        )];
        let env = |name: &str| match name {
            "RUSTC_WRAPPER" => Some("/usr/bin/sccache".to_string()),
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER" => Some("gcc".to_string()),
            _ => None,
        };
        let s = resolve_cargo_settings(
            &layers,
            &env,
            Path::new("/w"),
            Some("x86_64-unknown-linux-gnu".to_string()),
        );
        assert_eq!(
            s.rustc_wrapper.map(|w| w.source),
            Some("env:RUSTC_WRAPPER".to_string())
        );
        assert_eq!(s.linker.map(|l| l.value), Some("gcc".to_string()));
        assert_eq!(
            s.rustflags.map(|r| (r.value, r.source)),
            Some((
                "-Clink-arg=-fuse-ld=mold".to_string(),
                "/w/.cargo/config.toml".to_string()
            ))
        );
        assert_eq!(s.target_dir.source, "default");
        assert_eq!(s.target_dir.value, "/w/target");
    }

    #[test]
    fn encoded_rustflags_beat_every_other_source() {
        let env = |name: &str| match name {
            "CARGO_ENCODED_RUSTFLAGS" => Some("-C\x1fopt-level=3".to_string()),
            "RUSTFLAGS" => Some("-O".to_string()),
            _ => None,
        };
        let s = resolve_cargo_settings(&[], &env, Path::new("/w"), None);
        assert_eq!(
            s.rustflags.map(|r| r.value),
            Some("-C opt-level=3".to_string())
        );
    }

    #[test]
    fn config_layers_walks_ancestors_then_cargo_home() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let nested = root.join("a/b");
        std::fs::create_dir_all(nested.join(".cargo")).expect("mkdir");
        std::fs::create_dir_all(root.join(".cargo")).expect("mkdir");
        std::fs::write(nested.join(".cargo/config.toml"), "[build]\njobs = 1\n").expect("write");
        std::fs::write(root.join(".cargo/config"), "[build]\njobs = 3\n").expect("write");
        let home = root.join("home");
        std::fs::create_dir_all(&home).expect("mkdir");
        std::fs::write(home.join("config.toml"), "not = = toml").expect("write");

        let layers = config_layers(&nested, Some(&home));
        let paths: Vec<PathBuf> = layers.iter().map(|l| l.path.clone()).collect();
        assert_eq!(
            paths,
            [
                nested.join(".cargo/config.toml"),
                root.join(".cargo/config")
            ],
            "malformed home config is skipped"
        );
        assert_eq!(layers[1].base, root);
    }

    #[test]
    fn parses_load_average_on_both_platforms() {
        assert_eq!(
            parse_proc_loadavg("0.52 0.58 0.59 1/1234 5678\n"),
            Some([0.52, 0.58, 0.59])
        );
        assert_eq!(
            parse_sysctl_loadavg("{ 1.23 1.45 1.67 }\n"),
            Some([1.23, 1.45, 1.67])
        );
        assert_eq!(parse_proc_loadavg("garbage"), None);
    }

    #[test]
    fn parse_ps_keeps_build_processes_and_drops_excluded_pids() {
        let out = "    1 systemd\n  200 cargo\n  201 /Users/me/.rustup/toolchains/x/bin/rustc\n  202 cargo-nextest\n  300 cargo\n  400 cargo-watch\n";
        let procs = parse_ps(out, &[300]);
        let names: Vec<(u32, &str)> = procs.iter().map(|p| (p.pid, p.name.as_str())).collect();
        assert_eq!(
            names,
            [(200, "cargo"), (201, "rustc"), (202, "cargo-nextest")]
        );
    }

    #[test]
    fn mount_tables_parse_and_longest_prefix_wins() {
        let linux = "/dev/nvme0n1p2 / ext4 rw 0 0\ntmpfs /tmp tmpfs rw 0 0\n/dev/x /mnt/my\\040disk xfs rw 0 0\n";
        let mounts = parse_proc_mounts(linux);
        assert_eq!(
            fs_type_for(&mounts, Path::new("/tmp/build")),
            Some("tmpfs".to_string())
        );
        assert_eq!(
            fs_type_for(&mounts, Path::new("/home/u")),
            Some("ext4".to_string())
        );
        assert_eq!(
            fs_type_for(&mounts, Path::new("/mnt/my disk/t")),
            Some("xfs".to_string())
        );
        assert_eq!(
            fs_type_for(&mounts, Path::new("/tmpx")),
            Some("ext4".to_string())
        );

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
        assert_eq!(
            parse_rustc_host(out),
            Some("aarch64-apple-darwin".to_string())
        );
    }

    fn sample_report() -> MachineReport {
        MachineReport {
            schema_version: MACHINE_JSON_SCHEMA_VERSION,
            kind: "about-machine",
            os: "linux",
            cores: Some(8),
            load_average: Some([1.0, 0.5, 0.25]),
            build_processes: vec![BuildProcess {
                pid: 7,
                name: "rustc".to_string(),
            }],
            cargo: CargoSettings {
                host: Some("x86_64-unknown-linux-gnu".to_string()),
                jobs: Some(Setting {
                    value: "2".to_string(),
                    source: "/home/u/.cargo/config.toml".to_string(),
                }),
                rustc_wrapper: None,
                target_dir: Setting {
                    value: "/w/target".to_string(),
                    source: "default".to_string(),
                },
                linker: None,
                rustflags: None,
                config_files: vec!["/home/u/.cargo/config.toml".to_string()],
            },
            tmpdir: Some(FsReport {
                path: "/tmp".to_string(),
                fs_type: Some("tmpfs".to_string()),
                tmpfs: true,
                total_bytes: Some(1024 * 1024 * 1024),
                available_bytes: Some(512 * 1024 * 1024),
            }),
            target_dir: None,
            sccache: None,
        }
    }

    /// TASK-2287: pins the `ops about machine --json` envelope and field
    /// names.
    #[test]
    fn json_document_pins_schema_and_field_names() {
        let mut out = Vec::new();
        write_machine_report(&mut out, &sample_report(), true).expect("write");
        let value: serde_json::Value = serde_json::from_slice(&out).expect("json");
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["kind"], "about-machine");
        let keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let mut expected = vec![
            "schemaVersion",
            "kind",
            "os",
            "cores",
            "loadAverage",
            "buildProcesses",
            "cargo",
            "tmpdir",
            "targetDir",
            "sccache",
        ];
        expected.sort_unstable();
        let mut keys_sorted = keys.clone();
        keys_sorted.sort_unstable();
        assert_eq!(keys_sorted, expected);
        assert_eq!(
            value["cargo"]["jobs"]["source"],
            "/home/u/.cargo/config.toml"
        );
        assert_eq!(value["cargo"]["targetDir"]["value"], "/w/target");
        assert_eq!(value["tmpdir"]["tmpfs"], true);
        assert_eq!(value["tmpdir"]["fsType"], "tmpfs");
        assert_eq!(value["buildProcesses"][0]["name"], "rustc");
        let text = String::from_utf8(out).unwrap();
        assert!(text
            .trim_start()
            .starts_with("{\n  \"schemaVersion\": 1,\n  \"kind\": \"about-machine\""));
    }

    #[test]
    fn text_report_flags_tmpfs_and_names_sources() {
        let text = format_machine_report(&sample_report()).join("\n");
        assert!(
            text.contains("jobs           2  [/home/u/.cargo/config.toml]"),
            "{text}"
        );
        assert!(text.contains("WARNING: tmpfs"), "{text}");
        assert!(text.contains("rustc (7)"), "{text}");
        assert!(text.contains("1024 MiB (512 MiB free)"), "{text}");
    }

    #[test]
    fn unescape_octal_decodes_kernel_escapes() {
        assert_eq!(unescape_octal("/mnt/a\\040b"), "/mnt/a b");
        assert_eq!(unescape_octal("/plain"), "/plain");
        assert_eq!(unescape_octal("/trail\\"), "/trail\\");
    }
}
