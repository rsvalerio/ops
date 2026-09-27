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

use std::collections::BTreeMap;
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
    /// Incremental compilation: `CARGO_INCREMENTAL`, then
    /// `CARGO_BUILD_INCREMENTAL` / `build.incremental`, each overriding every
    /// profile. When none is set the value is `profile` with source
    /// `default`: each profile's own `incremental` applies (cargo's
    /// defaults: on for `dev`, off for `release`).
    pub incremental: Setting,
    /// The `dev` and `release` profiles' own `incremental`, when no global
    /// override applies (`None` otherwise): see [`IncrementalProfiles`].
    pub incremental_profiles: Option<IncrementalProfiles>,
    /// Every config file consulted, highest precedence first.
    pub config_files: Vec<String>,
}

/// Each built-in profile's effective `incremental`.
///
/// Cargo resolves it from `CARGO_PROFILE_<NAME>_INCREMENTAL`, then `[profile.<name>] incremental`
/// in the nearest config file, then in the workspace `Cargo.toml`, else
/// cargo's default (`true` for `dev`, `false` for `release`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct IncrementalProfiles {
    pub dev: Setting,
    pub release: Setting,
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
/// `cwd` and each ancestor's `.cargo/config` or `.cargo/config.toml` (the
/// legacy `config` when both exist, as cargo picks), then `$CARGO_HOME`'s
/// when no ancestor already covered it. Unreadable or malformed files are
/// skipped with a warning.
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
            // Cargo reads the legacy extensionless `config` when both exist
            // (and warns), so it is looked up first.
            let path = ["config", "config.toml"]
                .iter()
                .map(|name| dir.join(name))
                .find(|p| p.is_file())?;
            let base = dir.parent().map_or_else(|| dir.clone(), Path::to_path_buf);
            read_layer(path, base)
        })
        .collect()
}

/// The workspace root from `cargo locate-project`, and its parsed
/// `Cargo.toml` when readable.
#[derive(Debug, Clone, Copy)]
pub struct WorkspaceRoot<'a> {
    pub path: &'a Path,
    pub manifest: Option<&'a ConfigLayer>,
}

/// The workspace `Cargo.toml`, as a layer: its `[profile.*]` tables sit
/// below every config file's. `None` (with a warning) when unreadable.
#[must_use]
pub fn workspace_manifest(workspace_root: &Path) -> Option<ConfigLayer> {
    read_layer(
        workspace_root.join("Cargo.toml"),
        workspace_root.to_path_buf(),
    )
}

/// Read and parse one TOML file; unreadable or malformed files are skipped
/// with a warning.
fn read_layer(path: PathBuf, base: PathBuf) -> Option<ConfigLayer> {
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
    Some(ConfigLayer { path, base, table })
}

/// The value at the dotted `keys` in one table.
fn get_path<'a>(table: &'a toml::Table, keys: &[&str]) -> Option<&'a toml::Value> {
    let (first, rest) = keys.split_first()?;
    let mut value = table.get(*first)?;
    for key in rest {
        value = value.as_table()?.get(*key)?;
    }
    Some(value)
}

/// The first layer holding the dotted `keys`, with its value.
fn lookup<'a>(
    layers: &'a [ConfigLayer],
    keys: &[&str],
) -> Option<(&'a toml::Value, &'a ConfigLayer)> {
    layers
        .iter()
        .find_map(|layer| Some((get_path(&layer.table, keys)?, layer)))
}

/// Cargo's merge of a list-valued key (rustflags) across layers, given
/// highest precedence first: arrays concatenate with lower-precedence items
/// first, while a string replaces whatever lower layers contributed.
/// Every contributing source is named, in value order.
///
/// # Errors
///
/// A string in one layer and an array in another: cargo refuses to merge
/// them and fails the build, so there is no effective value to report.
fn merge_list(entries: Vec<(&toml::Value, String)>) -> Result<Option<Setting>, String> {
    if let [(first, first_source), rest @ ..] = entries.as_slice() {
        if let Some((_, source)) = rest.iter().find(|(v, _)| v.is_array() != first.is_array()) {
            return Err(format!(
                "cargo cannot merge a string and an array for the same key ({first_source}, {source})"
            ));
        }
    }
    let mut parts: Vec<Setting> = Vec::new();
    for (value, source) in entries.into_iter().rev() {
        if !value.is_array() {
            parts.clear();
        }
        parts.push(Setting {
            value: render_value(value),
            source,
        });
    }
    Ok(join_settings(&parts))
}

/// Space-join the values and comma-join the sources of `parts`, skipping
/// empty values (an empty array contributes nothing).
fn join_settings(parts: &[Setting]) -> Option<Setting> {
    let parts: Vec<&Setting> = parts.iter().filter(|p| !p.value.is_empty()).collect();
    (!parts.is_empty()).then(|| Setting {
        value: parts
            .iter()
            .map(|p| p.value.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        source: parts
            .iter()
            .map(|p| p.source.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    })
}

/// A list-valued key merged across every layer that sets it.
fn from_config_list(layers: &[ConfigLayer], keys: &[&str]) -> Result<Option<Setting>, String> {
    merge_list(
        layers
            .iter()
            .filter_map(|layer| {
                Some((
                    get_path(&layer.table, keys)?,
                    layer.path.display().to_string(),
                ))
            })
            .collect(),
    )
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

/// Parse `rustc --print cfg` output into its atoms (`unix`,
/// `target_os="linux"`, ...), one per line, whitespace-trimmed.
#[must_use]
pub fn parse_rustc_cfg(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// Whether a `target` table key like `cfg(all(unix, target_arch = "x86_64"))`
/// holds for a host whose `rustc --print cfg` atoms are `host_cfg`.
///
/// Supports cargo's grammar: bare names, `name = "value"`, `all(..)`,
/// `any(..)` and `not(..)`. A key that is not a `cfg(..)` expression, or that
/// does not parse, does not match.
#[must_use]
pub fn cfg_matches(key: &str, host_cfg: &[String]) -> bool {
    let Some(inner) = key
        .trim()
        .strip_prefix("cfg(")
        .and_then(|rest| rest.strip_suffix(')'))
    else {
        return false;
    };
    let mut parser = CfgParser { rest: inner };
    let Some(value) = parser.predicate(host_cfg) else {
        tracing::debug!(key, "about/machine: unparseable cfg target table");
        return false;
    };
    parser.rest.trim().is_empty() && value
}

/// Recursive-descent evaluator over a `cfg(..)` body.
struct CfgParser<'a> {
    rest: &'a str,
}

impl CfgParser<'_> {
    fn eat(&mut self, token: &str) -> bool {
        let trimmed = self.rest.trim_start();
        trimmed.strip_prefix(token).is_some_and(|after| {
            self.rest = after;
            true
        })
    }

    fn ident(&mut self) -> Option<&str> {
        let trimmed = self.rest.trim_start();
        let end = trimmed
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(trimmed.len());
        if end == 0 {
            return None;
        }
        let (ident, after) = trimmed.split_at(end);
        self.rest = after;
        Some(ident)
    }

    fn string(&mut self) -> Option<&str> {
        let trimmed = self.rest.trim_start().strip_prefix('"')?;
        let end = trimmed.find('"')?;
        let (value, after) = trimmed.split_at(end);
        self.rest = after.strip_prefix('"')?;
        Some(value)
    }

    fn list(&mut self, host_cfg: &[String]) -> Option<Vec<bool>> {
        if !self.eat("(") {
            return None;
        }
        let mut values = Vec::new();
        loop {
            if self.eat(")") {
                return Some(values);
            }
            values.push(self.predicate(host_cfg)?);
            if !self.eat(",") && !self.rest.trim_start().starts_with(')') {
                return None;
            }
        }
    }

    fn predicate(&mut self, host_cfg: &[String]) -> Option<bool> {
        let name = self.ident()?.to_string();
        match name.as_str() {
            "all" => Some(self.list(host_cfg)?.into_iter().all(|v| v)),
            "any" => Some(self.list(host_cfg)?.into_iter().any(|v| v)),
            "not" => {
                let values = self.list(host_cfg)?;
                match values.as_slice() {
                    [value] => Some(!value),
                    _ => None,
                }
            }
            _ if self.eat("=") => {
                let atom = format!("{name}=\"{}\"", self.string()?);
                Some(host_cfg.contains(&atom))
            }
            _ => Some(host_cfg.contains(&name)),
        }
    }
}

/// `target.'cfg(..)'` tables matching the host, across every layer in
/// precedence order, as `(table, layer, key)`.
fn matching_cfg_tables<'a>(
    layers: &'a [ConfigLayer],
    host_cfg: &[String],
) -> Vec<(&'a toml::Table, &'a ConfigLayer, &'a str)> {
    layers
        .iter()
        .flat_map(|layer| {
            layer
                .table
                .get("target")
                .and_then(toml::Value::as_table)
                .into_iter()
                .flatten()
                .filter(|(key, _)| cfg_matches(key, host_cfg))
                .filter_map(move |(key, value)| Some((value.as_table()?, layer, key.as_str())))
        })
        .collect()
}

fn cfg_source(layer: &ConfigLayer, key: &str) -> String {
    format!("{} [target.'{key}']", layer.path.display())
}

/// The host this report resolves `target.*` tables for.
#[derive(Debug, Clone, Default)]
pub struct HostTarget {
    /// Host triple from `rustc -vV`; the key for `target.<triple>.*`.
    pub triple: Option<String>,
    /// `rustc --print cfg` atoms, for `target.'cfg(..)'` tables.
    pub cfg: Vec<String>,
}

/// Resolve the effective cargo settings from env and config layers.
///
/// Follows cargo's precedence (env over config for the same key;
/// `CARGO_ENCODED_RUSTFLAGS` > `RUSTFLAGS` > target-table rustflags >
/// `build.rustflags` for rustflags). Target tables are `target.<host>` and
/// every `target.'cfg(..)'` table matching `host.cfg`: the triple's linker
/// wins over a cfg table's, and target rustflags from all matching tables
/// are joined, as cargo does. Array-valued rustflags keys are concatenated
/// across config layers (lower precedence first), not taken from the
/// nearest layer alone.
///
/// The default target dir is `<workspace_root>/target` — cargo anchors it
/// at the workspace root, not the cwd — falling back to `cwd` when the root
/// is unknown.
///
/// The workspace `Cargo.toml` is consulted only for profile settings.
#[must_use]
pub fn resolve_cargo_settings(
    layers: &[ConfigLayer],
    env: &dyn Fn(&str) -> Option<String>,
    cwd: &Path,
    workspace: Option<WorkspaceRoot<'_>>,
    host: HostTarget,
) -> CargoSettings {
    let workspace_root = workspace.map(|w| w.path);
    let manifest = workspace.and_then(|w| w.manifest);
    let HostTarget {
        triple: host,
        cfg: host_cfg,
    } = host;
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
            value: workspace_root
                .unwrap_or(cwd)
                .join("target")
                .display()
                .to_string(),
            source: "default".to_string(),
        });
    // Cargo: `CARGO_INCREMENTAL` overrides `build.incremental`, which
    // overrides every profile's `incremental`.
    let global_incremental = from_env(env, &["CARGO_INCREMENTAL", "CARGO_BUILD_INCREMENTAL"])
        .or_else(|| from_config(layers, &["build", "incremental"]));
    let incremental_profiles = global_incremental.is_none().then(|| IncrementalProfiles {
        dev: profile_incremental(layers, env, manifest, "dev", true),
        release: profile_incremental(layers, env, manifest, "release", false),
    });
    let incremental = global_incremental.unwrap_or_else(|| Setting {
        value: "profile".to_string(),
        source: "default".to_string(),
    });
    let (linker, target_rustflags) = resolve_target_tables(layers, env, host.as_deref(), &host_cfg);
    // Cargo merges config files at load, so a rejected merge fails every
    // build whatever env overrides are set: report no rustflags at all.
    let rustflags = match (
        target_rustflags,
        from_config_list(layers, &["build", "rustflags"]),
    ) {
        (Ok(target_rustflags), Ok(build_rustflags)) => env("CARGO_ENCODED_RUSTFLAGS")
            .filter(|v| !v.is_empty())
            .map(|v| Setting {
                value: v.split('\x1f').collect::<Vec<_>>().join(" "),
                source: "env:CARGO_ENCODED_RUSTFLAGS".to_string(),
            })
            .or_else(|| from_env(env, &["RUSTFLAGS"]))
            .or(target_rustflags)
            .or_else(|| from_env(env, &["CARGO_BUILD_RUSTFLAGS"]))
            .or(build_rustflags),
        (Err(e), _) | (_, Err(e)) => {
            tracing::warn!(error = %e, "about/machine: invalid rustflags config");
            None
        }
    };
    CargoSettings {
        host,
        jobs,
        rustc_wrapper,
        target_dir,
        linker,
        rustflags,
        incremental,
        incremental_profiles,
        config_files: layers
            .iter()
            .map(|l| l.path.display().to_string())
            .collect(),
    }
}

/// One profile's `incremental`: env, then config files, then the workspace
/// `Cargo.toml`, then cargo's `default`.
fn profile_incremental(
    layers: &[ConfigLayer],
    env: &dyn Fn(&str) -> Option<String>,
    manifest: Option<&ConfigLayer>,
    profile: &str,
    default: bool,
) -> Setting {
    let keys = ["profile", profile, "incremental"];
    from_env(
        env,
        &[&format!(
            "CARGO_PROFILE_{}_INCREMENTAL",
            profile.to_ascii_uppercase()
        )],
    )
    .or_else(|| from_config(layers, &keys))
    .or_else(|| from_config(std::slice::from_ref(manifest?), &keys))
    .unwrap_or_else(|| Setting {
        value: default.to_string(),
        source: "default".to_string(),
    })
}

/// Linker and rustflags from the host's target tables: env
/// `CARGO_TARGET_<TRIPLE>_*` first, then `target.<triple>`, then matching
/// `target.'cfg(..)'` tables. Rustflags are merged across layers per key,
/// then the triple's are followed by each matching cfg key's, sorted by key.
/// Rustflags are `Err` when a merge is one cargo rejects (see [`merge_list`]).
fn resolve_target_tables(
    layers: &[ConfigLayer],
    env: &dyn Fn(&str) -> Option<String>,
    triple: Option<&str>,
    host_cfg: &[String],
) -> (Option<Setting>, Result<Option<Setting>, String>) {
    let env_key = triple.map(triple_env_key);
    let env_setting = |suffix: &str| {
        env_key
            .as_deref()
            .and_then(|key| from_env(env, &[&format!("CARGO_TARGET_{key}_{suffix}")]))
    };
    let cfg_tables = matching_cfg_tables(layers, host_cfg);

    let linker = env_setting("LINKER")
        .or_else(|| triple.and_then(|t| from_config(layers, &["target", t, "linker"])))
        .or_else(|| {
            cfg_tables.iter().find_map(|(table, layer, key)| {
                table.get("linker").map(|value| Setting {
                    value: render_value(value),
                    source: cfg_source(layer, key),
                })
            })
        });

    let config_rustflags = (|| {
        let triple_flags = match triple {
            Some(t) => from_config_list(layers, &["target", t, "rustflags"])?,
            None => None,
        };
        // Cargo merges each `cfg(..)` key across layers, then walks the keys
        // in sorted order.
        let mut by_key: BTreeMap<&str, Vec<(&toml::Value, String)>> = BTreeMap::new();
        for (table, layer, key) in &cfg_tables {
            if let Some(value) = table.get("rustflags") {
                by_key
                    .entry(key)
                    .or_default()
                    .push((value, cfg_source(layer, key)));
            }
        }
        let mut parts: Vec<Setting> = triple_flags.into_iter().collect();
        for entries in by_key.into_values() {
            parts.extend(merge_list(entries)?);
        }
        Ok(join_settings(&parts))
    })();
    let rustflags = config_rustflags.map(|flags| env_setting("RUSTFLAGS").or(flags));
    (linker, rustflags)
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

/// Stats of the `sccache` on PATH.
fn sccache_stats() -> Option<serde_json::Value> {
    let text = probe(
        Command::new("sccache").args(["--show-stats", "--stats-format=json"]),
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
    let host = HostTarget {
        triple: probe(Command::new(&rustc).arg("-vV"), "rustc -vV")
            .and_then(|t| parse_rustc_host(&t)),
        cfg: probe(
            Command::new(&rustc).args(["--print", "cfg"]),
            "rustc --print cfg",
        )
        .map(|t| parse_rustc_cfg(&t))
        .unwrap_or_default(),
    };
    let cargo_bin = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let workspace_root = probe(
        Command::new(cargo_bin)
            .args(["locate-project", "--workspace", "--message-format", "plain"])
            .current_dir(cwd),
        "cargo locate-project --workspace",
    )
    .and_then(|t| parse_locate_project(&t));
    let manifest = workspace_root.as_deref().and_then(workspace_manifest);
    let workspace = workspace_root.as_deref().map(|path| WorkspaceRoot {
        path,
        manifest: manifest.as_ref(),
    });
    let cargo = resolve_cargo_settings(&layers, &env, cwd, workspace, host);

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
        // Probe the `sccache` on PATH, never the configured path: a repo's
        // `.cargo/config.toml` controls `build.rustc-wrapper`, and this
        // read-only page must not run a repo-supplied binary.
        .and_then(|_| sccache_stats());

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
        setting_line("incremental", Some(&report.cargo.incremental)),
    ];
    if let Some(profiles) = &report.cargo.incremental_profiles {
        lines.push(setting_line("  dev", Some(&profiles.dev)));
        lines.push(setting_line("  release", Some(&profiles.release)));
    }
    lines.extend([
        fs_line("TMPDIR", report.tmpdir.as_ref()),
        fs_line("target fs", report.target_dir.as_ref()),
    ]);
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

    /// TASK-2311 AC #2: `CARGO_INCREMENTAL` beats `CARGO_BUILD_INCREMENTAL`,
    /// which beats `build.incremental` in the nearest config layer; with
    /// none set the profile decides (`profile` / `default`).
    #[test]
    fn incremental_precedence_is_env_then_config_then_default() {
        let layers = [
            layer(
                "/w/.cargo/config.toml",
                "/w",
                "[build]\nincremental = false\n",
            ),
            layer(
                "/home/u/.cargo/config.toml",
                "/home/u",
                "[build]\nincremental = true\n",
            ),
        ];
        let resolve = |layers: &[ConfigLayer], env: &dyn Fn(&str) -> Option<String>| {
            resolve_cargo_settings(layers, env, Path::new("/w"), None, HostTarget::default())
                .incremental
        };
        let setting = |value: &str, source: &str| Setting {
            value: value.to_string(),
            source: source.to_string(),
        };
        let both_env = |name: &str| match name {
            "CARGO_INCREMENTAL" => Some("0".to_string()),
            "CARGO_BUILD_INCREMENTAL" => Some("true".to_string()),
            _ => None,
        };
        assert_eq!(
            resolve(&layers, &both_env),
            setting("0", "env:CARGO_INCREMENTAL")
        );
        let build_env =
            |name: &str| (name == "CARGO_BUILD_INCREMENTAL").then(|| "true".to_string());
        assert_eq!(
            resolve(&layers, &build_env),
            setting("true", "env:CARGO_BUILD_INCREMENTAL")
        );
        assert_eq!(
            resolve(&layers, &no_env),
            setting("false", "/w/.cargo/config.toml")
        );
        assert_eq!(resolve(&[], &no_env), setting("profile", "default"));
    }

    /// TASK-2320 AC #1: with no global override each profile resolves
    /// `CARGO_PROFILE_<NAME>_INCREMENTAL`, then the nearest config file,
    /// then the workspace `Cargo.toml`, then cargo's default; any global
    /// override leaves the profiles unreported.
    #[test]
    fn incremental_profiles_resolve_env_then_config_then_manifest_then_default() {
        let setting = |value: &str, source: &str| Setting {
            value: value.to_string(),
            source: source.to_string(),
        };
        let resolve = |layers: &[ConfigLayer],
                       manifest: Option<&ConfigLayer>,
                       env: &dyn Fn(&str) -> Option<String>| {
            resolve_cargo_settings(
                layers,
                env,
                Path::new("/w"),
                Some(WorkspaceRoot {
                    path: Path::new("/w"),
                    manifest,
                }),
                HostTarget::default(),
            )
            .incremental_profiles
        };
        let defaults = IncrementalProfiles {
            dev: setting("true", "default"),
            release: setting("false", "default"),
        };
        assert_eq!(resolve(&[], None, &no_env), Some(defaults));

        let manifest = layer(
            "/w/Cargo.toml",
            "/w",
            "[profile.dev]\nincremental = false\n[profile.release]\nincremental = true\n",
        );
        let config = [layer(
            "/w/.cargo/config.toml",
            "/w",
            "[profile.release]\nincremental = false\n",
        )];
        assert_eq!(
            resolve(&config, Some(&manifest), &no_env),
            Some(IncrementalProfiles {
                dev: setting("false", "/w/Cargo.toml"),
                release: setting("false", "/w/.cargo/config.toml"),
            })
        );
        let env =
            |name: &str| (name == "CARGO_PROFILE_DEV_INCREMENTAL").then(|| "true".to_string());
        assert_eq!(
            resolve(&config, Some(&manifest), &env)
                .expect("no global override")
                .dev,
            setting("true", "env:CARGO_PROFILE_DEV_INCREMENTAL")
        );

        let global = |name: &str| (name == "CARGO_INCREMENTAL").then(|| "1".to_string());
        assert_eq!(resolve(&config, Some(&manifest), &global), None);
        let build = [layer(
            "/w/.cargo/config.toml",
            "/w",
            "[build]\nincremental = true\n",
        )];
        assert_eq!(resolve(&build, Some(&manifest), &no_env), None);
    }

    /// TASK-2320: the text report lists each profile under `incremental`
    /// when the profiles decide, and the JSON carries them.
    #[test]
    fn profile_incremental_is_rendered_when_no_override_applies() {
        let mut report = sample_report();
        report.cargo.incremental = Setting {
            value: "profile".to_string(),
            source: "default".to_string(),
        };
        report.cargo.incremental_profiles = Some(IncrementalProfiles {
            dev: Setting {
                value: "true".to_string(),
                source: "default".to_string(),
            },
            release: Setting {
                value: "true".to_string(),
                source: "/w/Cargo.toml".to_string(),
            },
        });
        let text = format_machine_report(&report).join("\n");
        assert!(text.contains("    dev          true  [default]"), "{text}");
        assert!(
            text.contains("    release      true  [/w/Cargo.toml]"),
            "{text}"
        );
        let mut out = Vec::new();
        write_machine_report(&mut out, &report, true).expect("write");
        let value: serde_json::Value = serde_json::from_slice(&out).expect("json");
        assert_eq!(
            value["cargo"]["incrementalProfiles"]["release"],
            serde_json::json!({"value": "true", "source": "/w/Cargo.toml"})
        );
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
        let s = resolve_cargo_settings(
            &layers,
            &no_env,
            Path::new("/w"),
            None,
            HostTarget::default(),
        );
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
            None,
            HostTarget {
                triple: Some("x86_64-unknown-linux-gnu".to_string()),
                cfg: Vec::new(),
            },
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

    fn linux_cfg() -> Vec<String> {
        parse_rustc_cfg(
            "debug_assertions\npanic=\"unwind\"\ntarget_arch=\"x86_64\"\ntarget_os=\"linux\"\nunix\n",
        )
    }

    #[test]
    fn cfg_matches_evaluates_cargo_cfg_grammar() {
        let cfg = linux_cfg();
        assert!(cfg_matches("cfg(unix)", &cfg));
        assert!(!cfg_matches("cfg(windows)", &cfg));
        assert!(cfg_matches("cfg(target_os = \"linux\")", &cfg));
        assert!(!cfg_matches("cfg(target_os = \"macos\")", &cfg));
        assert!(cfg_matches(
            "cfg(all(unix, target_arch = \"x86_64\"))",
            &cfg
        ));
        assert!(cfg_matches("cfg(any(windows, unix))", &cfg));
        assert!(cfg_matches("cfg(not(windows))", &cfg));
        assert!(!cfg_matches(
            "cfg(all(unix, not(target_os = \"linux\")))",
            &cfg
        ));
        assert!(
            !cfg_matches("x86_64-unknown-linux-gnu", &cfg),
            "a triple is not a cfg"
        );
        assert!(
            !cfg_matches("cfg(all(unix", &cfg),
            "malformed never matches"
        );
        assert!(!cfg_matches("cfg(not(unix, windows))", &cfg));
    }

    /// TASK-2300 AC #1: a matching `target.'cfg(..)'` table supplies the
    /// linker (the triple table still wins) and its rustflags join the
    /// triple's, each naming its table as the source.
    #[test]
    fn cfg_target_tables_supply_linker_and_rustflags_with_source() {
        let layers = [layer(
            "/w/.cargo/config.toml",
            "/w",
            "[target.'cfg(target_os = \"linux\")']\nlinker = \"clang\"\nrustflags = [\"-Clink-arg=-fuse-ld=mold\"]\n\
             [target.'cfg(windows)']\nlinker = \"lld-link\"\n",
        )];
        let host = HostTarget {
            triple: Some("x86_64-unknown-linux-gnu".to_string()),
            cfg: linux_cfg(),
        };
        let s = resolve_cargo_settings(&layers, &no_env, Path::new("/w"), None, host.clone());
        assert_eq!(
            s.linker,
            Some(Setting {
                value: "clang".to_string(),
                source: "/w/.cargo/config.toml [target.'cfg(target_os = \"linux\")']".to_string(),
            })
        );
        assert_eq!(
            s.rustflags.map(|r| r.value),
            Some("-Clink-arg=-fuse-ld=mold".to_string())
        );

        let layers = [layer(
            "/w/.cargo/config.toml",
            "/w",
            "[target.x86_64-unknown-linux-gnu]\nlinker = \"gcc\"\nrustflags = [\"-Ctarget-cpu=native\"]\n\
             [target.'cfg(unix)']\nlinker = \"clang\"\nrustflags = [\"-Cforce-frame-pointers\"]\n",
        )];
        let s = resolve_cargo_settings(&layers, &no_env, Path::new("/w"), None, host);
        assert_eq!(s.linker.map(|l| l.value), Some("gcc".to_string()));
        let flags = s.rustflags.expect("joined rustflags");
        assert_eq!(flags.value, "-Ctarget-cpu=native -Cforce-frame-pointers");
        assert_eq!(
            flags.source,
            "/w/.cargo/config.toml, /w/.cargo/config.toml [target.'cfg(unix)']"
        );
    }

    /// TASK-2308: array-valued rustflags keys concatenate across layers,
    /// lower precedence first, naming every contributing file; a string
    /// value replaces what lower layers contributed.
    #[test]
    fn rustflags_arrays_merge_across_config_layers() {
        let host = HostTarget {
            triple: Some("x86_64-unknown-linux-gnu".to_string()),
            cfg: linux_cfg(),
        };
        let layers = [
            layer(
                "/w/.cargo/config.toml",
                "/w",
                "[build]\nrustflags = [\"-Ctarget-cpu=native\"]\n",
            ),
            layer(
                "/home/u/.cargo/config.toml",
                "/home/u",
                "[build]\nrustflags = [\"-Clink-arg=-fuse-ld=mold\"]\n",
            ),
        ];
        let flags = resolve_cargo_settings(&layers, &no_env, Path::new("/w"), None, host.clone())
            .rustflags
            .expect("merged rustflags");
        assert_eq!(flags.value, "-Clink-arg=-fuse-ld=mold -Ctarget-cpu=native");
        assert_eq!(
            flags.source,
            "/home/u/.cargo/config.toml, /w/.cargo/config.toml"
        );

        let layers = [
            layer(
                "/w/.cargo/config.toml",
                "/w",
                "[target.x86_64-unknown-linux-gnu]\nrustflags = [\"-Ctarget-cpu=native\"]\n\
                 [target.'cfg(unix)']\nrustflags = [\"-Cforce-frame-pointers\"]\n",
            ),
            layer(
                "/home/u/.cargo/config.toml",
                "/home/u",
                "[target.x86_64-unknown-linux-gnu]\nrustflags = [\"-Clink-arg=-fuse-ld=mold\"]\n\
                 [target.'cfg(unix)']\nrustflags = [\"-Cdebuginfo=1\"]\n",
            ),
        ];
        let flags = resolve_cargo_settings(&layers, &no_env, Path::new("/w"), None, host.clone())
            .rustflags
            .expect("merged target rustflags");
        assert_eq!(
            flags.value,
            "-Clink-arg=-fuse-ld=mold -Ctarget-cpu=native -Cdebuginfo=1 -Cforce-frame-pointers"
        );
        assert_eq!(
            flags.source,
            "/home/u/.cargo/config.toml, /w/.cargo/config.toml, \
             /home/u/.cargo/config.toml [target.'cfg(unix)'], /w/.cargo/config.toml [target.'cfg(unix)']"
        );

        // A string replaces lower layers only when they are strings too.
        let layers = [
            layer(
                "/w/.cargo/config.toml",
                "/w",
                "[build]\nrustflags = \"-Ctarget-cpu=native\"\n",
            ),
            layer(
                "/home/u/.cargo/config.toml",
                "/home/u",
                "[build]\nrustflags = \"-Clink-arg=-fuse-ld=mold\"\n",
            ),
        ];
        let flags = resolve_cargo_settings(&layers, &no_env, Path::new("/w"), None, host.clone())
            .rustflags
            .expect("string rustflags");
        assert_eq!(
            (flags.value.as_str(), flags.source.as_str()),
            ("-Ctarget-cpu=native", "/w/.cargo/config.toml")
        );

        // Cargo rejects a string merged with an array, in either order and
        // whatever env overrides are set: no effective rustflags.
        let env = |name: &str| (name == "RUSTFLAGS").then(|| "-Copt-level=1".to_string());
        for (near, far) in [
            ("\"-Ctarget-cpu=native\"", "[\"-Clink-arg=-fuse-ld=mold\"]"),
            ("[\"-Ctarget-cpu=native\"]", "\"-Clink-arg=-fuse-ld=mold\""),
        ] {
            for table in [
                "build",
                "target.x86_64-unknown-linux-gnu",
                "target.'cfg(unix)'",
            ] {
                let layers = [
                    layer(
                        "/w/.cargo/config.toml",
                        "/w",
                        &format!("[{table}]\nrustflags = {near}\n"),
                    ),
                    layer(
                        "/home/u/.cargo/config.toml",
                        "/home/u",
                        &format!("[{table}]\nrustflags = {far}\n"),
                    ),
                ];
                let s = resolve_cargo_settings(&layers, &env, Path::new("/w"), None, host.clone());
                assert_eq!(s.rustflags, None, "[{table}] {near} over {far}");
            }
        }
    }

    /// TASK-2300 AC #2: the default target dir is anchored at the workspace
    /// root, not the cwd of a member subdirectory.
    #[test]
    fn default_target_dir_resolves_against_the_workspace_root() {
        let s = resolve_cargo_settings(
            &[],
            &no_env,
            Path::new("/w/crates/member"),
            Some(WorkspaceRoot {
                path: Path::new("/w"),
                manifest: None,
            }),
            HostTarget::default(),
        );
        assert_eq!(s.target_dir.value, "/w/target");
        assert_eq!(s.target_dir.source, "default");
        let s = resolve_cargo_settings(
            &[],
            &no_env,
            Path::new("/w/crates/member"),
            None,
            HostTarget::default(),
        );
        assert_eq!(
            s.target_dir.value, "/w/crates/member/target",
            "cwd fallback"
        );
        assert_eq!(
            parse_locate_project("/w/Cargo.toml\n"),
            Some(PathBuf::from("/w"))
        );
        assert_eq!(parse_locate_project(""), None);
    }

    #[test]
    fn encoded_rustflags_beat_every_other_source() {
        let env = |name: &str| match name {
            "CARGO_ENCODED_RUSTFLAGS" => Some("-C\x1fopt-level=3".to_string()),
            "RUSTFLAGS" => Some("-O".to_string()),
            _ => None,
        };
        let s = resolve_cargo_settings(&[], &env, Path::new("/w"), None, HostTarget::default());
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

    /// With both `.cargo/config` and `.cargo/config.toml` in one directory,
    /// cargo uses `config` (and warns); so does the report, value and source.
    #[test]
    fn legacy_config_wins_over_config_toml_in_one_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join(".cargo")).expect("mkdir");
        std::fs::write(
            root.join(".cargo/config"),
            "[profile.dev]\nincremental = false\n",
        )
        .expect("write");
        std::fs::write(
            root.join(".cargo/config.toml"),
            "[profile.dev]\nincremental = true\n",
        )
        .expect("write");

        let layers = config_layers(root, None);
        let paths: Vec<PathBuf> = layers.iter().map(|l| l.path.clone()).collect();
        assert_eq!(paths, [root.join(".cargo/config")]);
        let profiles = resolve_cargo_settings(&layers, &no_env, root, None, HostTarget::default())
            .incremental_profiles
            .expect("no global override");
        assert_eq!(
            profiles.dev,
            Setting {
                value: "false".to_string(),
                source: root.join(".cargo/config").display().to_string(),
            }
        );
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
                incremental: Setting {
                    value: "0".to_string(),
                    source: "env:CARGO_INCREMENTAL".to_string(),
                },
                incremental_profiles: None,
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
        assert_eq!(
            value["cargo"]["incremental"],
            serde_json::json!({"value": "0", "source": "env:CARGO_INCREMENTAL"})
        );
        assert!(value["cargo"]["incrementalProfiles"].is_null());
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
        assert!(
            text.contains("incremental    0  [env:CARGO_INCREMENTAL]"),
            "{text}"
        );
        assert!(!text.contains("    dev "), "{text}");
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
