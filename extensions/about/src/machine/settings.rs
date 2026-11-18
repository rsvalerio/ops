//! Effective cargo settings resolved from env and the config layers,
//! following cargo's precedence rules.

use std::collections::BTreeMap;
use std::path::Path;

use super::cfg_expr::cfg_matches;
use super::layers::{ConfigLayer, WorkspaceRoot};

/// An effective setting and where it came from: `env:NAME`, a config file
/// path, or `default`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Setting {
    /// The effective value, rendered as a string.
    pub value: String,
    /// Where it came from: `env:NAME`, a config file path, or `default`.
    pub source: String,
}

/// Effective cargo settings across env and every config layer.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CargoSettings {
    /// Host triple from `rustc -vV`; the key for `target.<triple>.*`.
    pub host: Option<String>,
    /// Parallel build jobs (`CARGO_BUILD_JOBS` / `build.jobs`), when set.
    pub jobs: Option<Setting>,
    /// Compiler wrapper such as `sccache` (`RUSTC_WRAPPER` / `build.rustc-wrapper`), when set.
    pub rustc_wrapper: Option<Setting>,
    /// Effective target directory; `<workspace root>/target` when unset.
    pub target_dir: Setting,
    /// Linker from the host's matching `target.*` tables, when set.
    pub linker: Option<Setting>,
    /// Extra `rustc` flags (`RUSTFLAGS` and friends), when set.
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
    /// The `dev` profile's effective `incremental`.
    pub dev: Setting,
    /// The `release` profile's effective `incremental`.
    pub release: Setting,
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
    layers.iter().find_map(|layer| Some((get_path(&layer.table, keys)?, layer)))
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
        parts.push(Setting { value: render_value(value), source });
    }
    Ok(join_settings(&parts))
}

/// Space-join the values and comma-join the sources of `parts`, skipping
/// empty values (an empty array contributes nothing).
fn join_settings(parts: &[Setting]) -> Option<Setting> {
    let parts: Vec<&Setting> = parts.iter().filter(|p| !p.value.is_empty()).collect();
    (!parts.is_empty()).then(|| Setting {
        value: parts.iter().map(|p| p.value.as_str()).collect::<Vec<_>>().join(" "),
        source: parts.iter().map(|p| p.source.as_str()).collect::<Vec<_>>().join(", "),
    })
}

/// A list-valued key merged across every layer that sets it.
fn from_config_list(layers: &[ConfigLayer], keys: &[&str]) -> Result<Option<Setting>, String> {
    merge_list(
        layers
            .iter()
            .filter_map(|layer| {
                Some((get_path(&layer.table, keys)?, layer.path.display().to_string()))
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
        env(name)
            .filter(|v| !v.is_empty())
            .map(|value| Setting { value, source: format!("env:{name}") })
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
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
        .collect()
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
    let HostTarget { triple: host, cfg: host_cfg } = host;
    let jobs =
        from_env(env, &["CARGO_BUILD_JOBS"]).or_else(|| from_config(layers, &["build", "jobs"]));
    let rustc_wrapper = from_env(env, &["RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WRAPPER"])
        .or_else(|| from_config(layers, &["build", "rustc-wrapper"]));
    let target_dir = resolve_target_dir(layers, env, cwd, workspace_root);
    // Cargo: `CARGO_INCREMENTAL` overrides `build.incremental`, which
    // overrides every profile's `incremental`.
    let global_incremental = from_env(env, &["CARGO_INCREMENTAL", "CARGO_BUILD_INCREMENTAL"])
        .or_else(|| from_config(layers, &["build", "incremental"]));
    let incremental_profiles = global_incremental.is_none().then(|| IncrementalProfiles {
        dev: profile_incremental(layers, env, manifest, "dev", true),
        release: profile_incremental(layers, env, manifest, "release", false),
    });
    let incremental = global_incremental
        .unwrap_or_else(|| Setting { value: "profile".to_string(), source: "default".to_string() });
    let (linker, target_rustflags) = resolve_target_tables(layers, env, host.as_deref(), &host_cfg);
    let rustflags =
        resolve_rustflags(env, target_rustflags, from_config_list(layers, &["build", "rustflags"]));
    CargoSettings {
        host,
        jobs,
        rustc_wrapper,
        target_dir,
        linker,
        rustflags,
        incremental,
        incremental_profiles,
        config_files: layers.iter().map(|l| l.path.display().to_string()).collect(),
    }
}

/// The effective target dir: `CARGO_TARGET_DIR` / `CARGO_BUILD_TARGET_DIR`
/// resolved against `cwd`, then `build.target-dir` resolved against the
/// layer's base, then `<workspace_root>/target` — cargo anchors the default
/// at the workspace root, not the cwd — falling back to `cwd` when the root
/// is unknown.
fn resolve_target_dir(
    layers: &[ConfigLayer],
    env: &dyn Fn(&str) -> Option<String>,
    cwd: &Path,
    workspace_root: Option<&Path>,
) -> Setting {
    from_env(env, &["CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR"])
        .map(|s| Setting { value: cwd.join(&s.value).display().to_string(), source: s.source })
        .or_else(|| {
            lookup(layers, &["build", "target-dir"]).map(|(value, layer)| Setting {
                value: layer.base.join(render_value(value)).display().to_string(),
                source: layer.path.display().to_string(),
            })
        })
        .unwrap_or_else(|| Setting {
            value: workspace_root.unwrap_or(cwd).join("target").display().to_string(),
            source: "default".to_string(),
        })
}

/// The effective rustflags: `CARGO_ENCODED_RUSTFLAGS` > `RUSTFLAGS` >
/// target-table rustflags > `CARGO_BUILD_RUSTFLAGS` > `build.rustflags`.
///
/// Cargo merges config files at load, so a rejected merge fails every
/// build whatever env overrides are set: report no rustflags at all.
fn resolve_rustflags(
    env: &dyn Fn(&str) -> Option<String>,
    target_rustflags: Result<Option<Setting>, String>,
    build_rustflags: Result<Option<Setting>, String>,
) -> Option<Setting> {
    match (target_rustflags, build_rustflags) {
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
            tracing::warn!(error = ?e, "about/machine: invalid rustflags config");
            None
        }
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
    from_env(env, &[&format!("CARGO_PROFILE_{}_INCREMENTAL", profile.to_ascii_uppercase())])
        .or_else(|| from_config(layers, &keys))
        .or_else(|| from_config(std::slice::from_ref(manifest?), &keys))
        .unwrap_or_else(|| Setting { value: default.to_string(), source: "default".to_string() })
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
        env_key.as_deref().and_then(|key| from_env(env, &[&format!("CARGO_TARGET_{key}_{suffix}")]))
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
                by_key.entry(key).or_default().push((value, cfg_source(layer, key)));
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

#[cfg(test)]
mod tests {
    use super::super::cfg_expr::tests::linux_cfg;
    use super::super::parse_locate_project;
    use super::*;
    use std::path::PathBuf;

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

    /// `CARGO_INCREMENTAL` beats `CARGO_BUILD_INCREMENTAL`,
    /// which beats `build.incremental` in the nearest config layer; with
    /// none set the profile decides (`profile` / `default`).
    #[test]
    fn incremental_precedence_is_env_then_config_then_default() {
        let layers = [
            layer("/w/.cargo/config.toml", "/w", "[build]\nincremental = false\n"),
            layer("/home/u/.cargo/config.toml", "/home/u", "[build]\nincremental = true\n"),
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
        assert_eq!(resolve(&layers, &both_env), setting("0", "env:CARGO_INCREMENTAL"));
        let build_env =
            |name: &str| (name == "CARGO_BUILD_INCREMENTAL").then(|| "true".to_string());
        assert_eq!(resolve(&layers, &build_env), setting("true", "env:CARGO_BUILD_INCREMENTAL"));
        assert_eq!(resolve(&layers, &no_env), setting("false", "/w/.cargo/config.toml"));
        assert_eq!(resolve(&[], &no_env), setting("profile", "default"));
    }

    /// With no global override each profile resolves
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
                Some(WorkspaceRoot { path: Path::new("/w"), manifest }),
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
        let config =
            [layer("/w/.cargo/config.toml", "/w", "[profile.release]\nincremental = false\n")];
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
            resolve(&config, Some(&manifest), &env).expect("no global override").dev,
            setting("true", "env:CARGO_PROFILE_DEV_INCREMENTAL")
        );

        let global = |name: &str| (name == "CARGO_INCREMENTAL").then(|| "1".to_string());
        assert_eq!(resolve(&config, Some(&manifest), &global), None);
        let build = [layer("/w/.cargo/config.toml", "/w", "[build]\nincremental = true\n")];
        assert_eq!(resolve(&build, Some(&manifest), &no_env), None);
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
        let s =
            resolve_cargo_settings(&layers, &no_env, Path::new("/w"), None, HostTarget::default());
        assert_eq!(
            s.jobs,
            Some(Setting {
                value: "2".to_string(),
                source: "/home/u/.cargo/config.toml".to_string()
            })
        );
        assert_eq!(s.target_dir.value, "/w/out");
        assert_eq!(s.target_dir.source, "/w/.cargo/config.toml");
        assert_eq!(s.rustflags.map(|r| r.value), Some("-C target-cpu=native".to_string()));
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
            HostTarget { triple: Some("x86_64-unknown-linux-gnu".to_string()), cfg: Vec::new() },
        );
        assert_eq!(s.rustc_wrapper.map(|w| w.source), Some("env:RUSTC_WRAPPER".to_string()));
        assert_eq!(s.linker.map(|l| l.value), Some("gcc".to_string()));
        assert_eq!(
            s.rustflags.map(|r| (r.value, r.source)),
            Some(("-Clink-arg=-fuse-ld=mold".to_string(), "/w/.cargo/config.toml".to_string()))
        );
        assert_eq!(s.target_dir.source, "default");
        assert_eq!(s.target_dir.value, "/w/target");
    }

    /// A matching `target.'cfg(..)'` table supplies the
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
        let host =
            HostTarget { triple: Some("x86_64-unknown-linux-gnu".to_string()), cfg: linux_cfg() };
        let s = resolve_cargo_settings(&layers, &no_env, Path::new("/w"), None, host.clone());
        assert_eq!(
            s.linker,
            Some(Setting {
                value: "clang".to_string(),
                source: "/w/.cargo/config.toml [target.'cfg(target_os = \"linux\")']".to_string(),
            })
        );
        assert_eq!(s.rustflags.map(|r| r.value), Some("-Clink-arg=-fuse-ld=mold".to_string()));

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

    /// Array-valued rustflags keys concatenate across layers,
    /// lower precedence first, naming every contributing file; a string
    /// value replaces what lower layers contributed.
    #[test]
    fn rustflags_arrays_merge_across_config_layers() {
        let host =
            HostTarget { triple: Some("x86_64-unknown-linux-gnu".to_string()), cfg: linux_cfg() };
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
        assert_eq!(flags.source, "/home/u/.cargo/config.toml, /w/.cargo/config.toml");

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
            layer("/w/.cargo/config.toml", "/w", "[build]\nrustflags = \"-Ctarget-cpu=native\"\n"),
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
            for table in ["build", "target.x86_64-unknown-linux-gnu", "target.'cfg(unix)'"] {
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

    /// The default target dir is anchored at the workspace
    /// root, not the cwd of a member subdirectory.
    #[test]
    fn default_target_dir_resolves_against_the_workspace_root() {
        let s = resolve_cargo_settings(
            &[],
            &no_env,
            Path::new("/w/crates/member"),
            Some(WorkspaceRoot { path: Path::new("/w"), manifest: None }),
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
        assert_eq!(s.target_dir.value, "/w/crates/member/target", "cwd fallback");
        assert_eq!(parse_locate_project("/w/Cargo.toml\n"), Some(PathBuf::from("/w")));
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
        assert_eq!(s.rustflags.map(|r| r.value), Some("-C opt-level=3".to_string()));
    }
}
