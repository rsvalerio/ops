//! `about machine` subpage: the build-relevant state of this machine.
//!
//! Reports what a build timing depends on besides the code: core count and
//! load, other cargo/rustc/nextest processes competing for the CPU, the
//! effective cargo settings (`build.jobs`, rustc wrapper, target dir,
//! linker, rustflags, incremental) with the layer each came from, the filesystem behind
//! `TMPDIR` and the target dir (a tmpfs `/tmp` fails cold builds with exit
//! 101 and no compile error), and sccache counters when sccache wraps rustc.
//!
//! Every probe degrades to "unknown" rather than failing the page: a
//! missing `ps`, an unreadable mount table or a malformed cargo config
//! leaves its field `null` and logs a warning. Linux and macOS are both
//! supported; each OS-specific probe has a pure parser behind it so the
//! parsing is tested on every host.
//!
//! Split by concern: `layers` finds and parses the cargo config files,
//! `settings` resolves the effective settings from them, `cfg_expr`
//! evaluates `target.'cfg(..)'` keys, `probe` runs and parses the OS
//! probes, and `render` draws the text report. This module holds the
//! report types and the command entry point.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

mod cfg_expr;
mod layers;
mod probe;
mod render;
mod settings;

pub use cfg_expr::{cfg_matches, parse_rustc_cfg};
pub use layers::{config_layers, workspace_manifest, ConfigLayer, WorkspaceRoot};
pub use probe::{
    fs_type_for, parse_bsd_mount, parse_df, parse_locate_project, parse_proc_loadavg,
    parse_proc_mounts, parse_ps, parse_rustc_host, parse_sysctl_loadavg,
};
pub use render::format_machine_report;
pub use settings::{
    resolve_cargo_settings, CargoSettings, HostTarget, IncrementalProfiles, Setting,
};

use probe::{
    fs_report, is_sccache, load_average, locate_workspace_root, mount_table, probe,
    probe_host_target, sccache_stats,
};

/// Version of the `ops about machine --json` document shape.
pub const MACHINE_JSON_SCHEMA_VERSION: u32 = 1;

/// The `ops about machine --json` document.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineReport {
    /// Version of this JSON document's schema.
    pub schema_version: u32,
    /// Document discriminator; always `"about-machine"`.
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
    /// Effective cargo settings and where each one came from.
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
    /// Process id.
    pub pid: u32,
    /// Process name (final component of `comm`).
    pub name: String,
}

/// Filesystem facts for one path.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsReport {
    /// The path reported on, as given (its nearest existing ancestor is
    /// probed when it does not exist yet).
    pub path: String,
    /// Filesystem type of the longest matching mount.
    pub fs_type: Option<String>,
    /// Whether that filesystem is a tmpfs (fails cold builds).
    pub tmpfs: bool,
    /// Total size, from `df -Pk`.
    pub total_bytes: Option<u64>,
    /// Available size, from `df -Pk`.
    pub available_bytes: Option<u64>,
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
    let host = probe_host_target(&rustc);
    let cargo_bin = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let workspace_root = locate_workspace_root(&cargo_bin, cwd);
    let manifest = workspace_root.as_deref().and_then(workspace_manifest);
    let workspace =
        workspace_root.as_deref().map(|path| WorkspaceRoot { path, manifest: manifest.as_ref() });
    let cargo = resolve_cargo_settings(&layers, &env, cwd, workspace, host);

    // The invoking `cargo run` (if any) is our parent, not competing work.
    let mut exclude = vec![std::process::id()];
    #[cfg(unix)]
    exclude.push(std::os::unix::process::parent_id());
    let build_processes = probe(Command::new("ps").args(["-axo", "pid=,comm="]), "ps")
        .map_or_default(|t| parse_ps(&t, &exclude));

    let mounts = mount_table();
    let tmpdir = fs_report(&std::env::temp_dir(), &mounts);
    let target_dir = fs_report(Path::new(&cargo.target_dir.value), &mounts);
    let sccache = cargo
        .rustc_wrapper
        .as_ref()
        .filter(|w| is_sccache(&w.value))
        // Probe the `sccache` on PATH, never the configured path. The
        // invariant this page upholds: it never runs a program *path* read
        // from the checkout's config (`build.rustc-wrapper` here). It does
        // run the toolchain inside the checkout — `rustc` / `cargo` from
        // `RUSTC` / `CARGO` / PATH, so a rustup proxy honours a checked-in
        // `rust-toolchain.toml` — exactly as any cargo command run there
        // would; see `probe::probe_host_target`.
        .and_then(|_| sccache_stats());

    MachineReport {
        schema_version: MACHINE_JSON_SCHEMA_VERSION,
        kind: "about-machine",
        os: std::env::consts::OS,
        cores: std::thread::available_parallelism().ok().map(std::num::NonZeroUsize::get),
        load_average: load_average(),
        build_processes,
        cargo,
        tmpdir,
        target_dir,
        sccache,
    }
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
