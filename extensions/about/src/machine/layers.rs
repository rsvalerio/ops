//! Cargo config layer discovery: the `.cargo/config[.toml]` files of `cwd`
//! and its ancestors, `$CARGO_HOME`'s, and the files their `include`
//! arrays name, each parsed into a [`ConfigLayer`].

use std::path::{Path, PathBuf};

/// One parsed cargo config file.
#[derive(Debug, Clone)]
pub struct ConfigLayer {
    /// The file, as reported in `source`.
    pub path: PathBuf,
    /// The directory relative config paths resolve against: the parent of
    /// the `.cargo` directory holding the file.
    pub base: PathBuf,
    /// The parsed TOML table.
    pub table: toml::Table,
}

/// Find and parse every cargo config layer for `cwd`, highest precedence first.
///
/// `cwd` and each ancestor's `.cargo/config` or `.cargo/config.toml` (the
/// legacy `config` when both exist, as cargo picks), then `$CARGO_HOME`'s
/// when no ancestor already covered it. Each file's `include`s follow it as
/// layers of their own (see `push_with_includes`). Unreadable or malformed
/// files are skipped with a warning.
#[must_use]
pub fn config_layers(cwd: &Path, cargo_home: Option<&Path>) -> Vec<ConfigLayer> {
    let mut dirs: Vec<PathBuf> = cwd.ancestors().map(|a| a.join(".cargo")).collect();
    if let Some(home) = cargo_home {
        if !dirs.iter().any(|d| d == home) {
            dirs.push(home.to_path_buf());
        }
    }
    let mut layers = Vec::new();
    for dir in &dirs {
        // Cargo reads the legacy extensionless `config` when both exist
        // (and warns), so it is tried first. Selection follows *existence*,
        // not parse success: a present-but-malformed `config` contributes
        // no layer (the parse warning fires) rather than silently falling
        // through to `config.toml`, matching cargo's refusal to load a
        // config whose chosen file does not parse.
        let base = dir.parent().map_or_else(|| dir.clone(), Path::to_path_buf);
        let selected = ["config", "config.toml"].iter().find(|name| dir.join(name).exists());
        let layer = selected.and_then(|name| read_layer(dir.join(name), base.clone()));
        if let Some(layer) = layer {
            push_with_includes(layer, &mut layers, &mut Vec::new());
        }
    }
    layers
}

/// Push `layer`, then the files its `include` array names, highest
/// precedence first — cargo merges includes in listed order and the
/// including file on top, so the last include ranks right below `layer`.
/// Each include is expanded the same way, its path resolved against the
/// including file's directory.
///
/// Where cargo refuses to load the configuration — a missing non-optional
/// include, a malformed `include` entry, a path not ending in `.toml`, an
/// include cycle — the offending include is skipped with a warning.
fn push_with_includes(layer: ConfigLayer, out: &mut Vec<ConfigLayer>, chain: &mut Vec<PathBuf>) {
    let includes = include_paths(&layer);
    chain.push(identity(&layer.path));
    out.push(layer);
    for (path, optional) in includes.into_iter().rev() {
        if chain.contains(&identity(&path)) {
            tracing::warn!(path = ?path.display(), "about/machine: cargo config include cycle");
            continue;
        }
        // Cargo resolves an included file's relative paths like any other
        // config file's: against the parent of the directory holding it.
        let base =
            path.parent().and_then(Path::parent).map_or_else(PathBuf::new, Path::to_path_buf);
        // Optionality is decided by the read result rather than an
        // `is_file` pre-probe: the probe and the read re-resolve the path
        // separately, and a swap between them races. A missing optional
        // include is an expected miss; a non-optional one that did not
        // load is warned about here (matching cargo's refusal to load a
        // config whose required include is absent).
        match read_layer(path.clone(), base) {
            Some(included) => push_with_includes(included, out, chain),
            None if optional => {}
            None => tracing::warn!(
                // Debug, not Display: a path is filesystem-controlled data,
                // and `?` escapes any newline or ANSI sequence it carries.
                path = ?path.display(),
                "about/machine: cargo config include not loaded"
            ),
        }
    }
    chain.pop();
}

/// A file's identity for cycle detection: its canonical path (so `..` and
/// symlinks cannot disguise a revisit), or the path itself when it cannot
/// be resolved.
fn identity(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Drop `.` and fold `dir/..` lexically, so a reported source reads
/// `.cargo/config.toml` rather than `.cargo/sub/../config.toml`.
fn fold_dots(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir
                if matches!(out.components().next_back(), Some(Component::Normal(_))) =>
            {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// The `include` entries of `layer`, in listed order, as `(path, optional)`
/// with each path resolved against the including file's directory. Cargo
/// accepts an array of strings or of `{ path, optional }` tables, each
/// path ending in `.toml`; anything else is skipped with a warning.
fn include_paths(layer: &ConfigLayer) -> Vec<(PathBuf, bool)> {
    let Some(value) = layer.table.get("include") else {
        return Vec::new();
    };
    let warn = |what: &str| {
        tracing::warn!(
            path = ?layer.path.display(),
            reason = what,
            "about/machine: ignoring cargo config include"
        );
    };
    let Some(entries) = value.as_array() else {
        warn("not an array");
        return Vec::new();
    };
    let dir = layer.path.parent().unwrap_or_else(|| Path::new(""));
    entries
        .iter()
        .filter_map(|entry| {
            let (path, optional) = match entry {
                toml::Value::String(path) => (path.as_str(), false),
                toml::Value::Table(t) => {
                    let Some(path) = t.get("path").and_then(toml::Value::as_str) else {
                        warn("table entry without a `path` string");
                        return None;
                    };
                    let optional =
                        t.get("optional").and_then(toml::Value::as_bool).unwrap_or(false);
                    (path, optional)
                }
                _ => {
                    warn("entry is neither a string nor a table");
                    return None;
                }
            };
            if Path::new(path).extension().is_none_or(|ext| ext != "toml") {
                warn("path does not end in `.toml`");
                return None;
            }
            Some((fold_dots(&dir.join(path)), optional))
        })
        .collect()
}

/// The workspace root from `cargo locate-project`, and its parsed
/// `Cargo.toml` when readable.
#[derive(Debug, Clone, Copy)]
pub struct WorkspaceRoot<'a> {
    /// The workspace root directory.
    pub path: &'a Path,
    /// The parsed workspace `Cargo.toml`, when readable.
    pub manifest: Option<&'a ConfigLayer>,
}

/// The workspace `Cargo.toml`, as a layer: its `[profile.*]` tables sit
/// below every config file's. `None` (with a warning) when unreadable.
#[must_use]
pub fn workspace_manifest(workspace_root: &Path) -> Option<ConfigLayer> {
    read_layer(workspace_root.join("Cargo.toml"), workspace_root.to_path_buf())
}

/// Read and parse one TOML file. A file that is simply absent is a silent
/// `None` — absence is an expected miss for config-layer discovery and
/// optional includes; unreadable or malformed files are skipped with a
/// warning.
fn read_layer(path: PathBuf, base: PathBuf) -> Option<ConfigLayer> {
    let text = match read_capped_regular_file(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            tracing::warn!(path = ?path.display(), error = ?e.to_string(), "about/machine: reading cargo config failed");
            return None;
        }
    };
    let table = match text.parse::<toml::Table>() {
        Ok(table) => table,
        Err(e) => {
            tracing::warn!(path = ?path.display(), error = ?e.to_string(), "about/machine: parsing cargo config failed");
            return None;
        }
    };
    Some(ConfigLayer { path, base, table })
}

/// Read `path` to a string, refusing anything but a regular file and
/// anything over [`ops_core::text::manifest_max_bytes`].
///
/// The paths are repo-controlled (ancestor `.cargo/config[.toml]` files and
/// whatever their `include` arrays name, absolute paths included), so a FIFO
/// must not block the page and a huge file must not be read whole. The read
/// goes through [`ops_core::text::read_capped_bytes_following`], which opens
/// without blocking on a FIFO and type-checks the opened handle.
///
/// Unlike [`ops_core::text::read_capped_to_string`] this follows symlinks:
/// cargo does, and a symlinked `~/.cargo/config.toml` (a dotfiles checkout)
/// is ordinary. The report must describe the config cargo actually loads,
/// and reading through a link only ever reports on the file, never writes it.
///
/// Errors keep the `io::Error` shape the caller matches on: an absent file is
/// [`std::io::ErrorKind::NotFound`], an oversized or non-UTF-8 file is
/// [`std::io::ErrorKind::InvalidData`], and open or read failures pass
/// through verbatim.
fn read_capped_regular_file(path: &Path) -> std::io::Result<String> {
    use ops_core::text::CappedReadError;
    use std::io::{Error, ErrorKind};
    let cap = ops_core::text::manifest_max_bytes();
    let bytes = ops_core::text::read_capped_bytes_following(path, cap).map_err(|e| match e {
        CappedReadError::NotFound(e) | CappedReadError::Open(e) | CappedReadError::Read(e) => e,
        CappedReadError::TooLarge { cap } => Error::new(
            ErrorKind::InvalidData,
            format!(
                "file exceeds {cap}-byte cap (override via {})",
                ops_core::text::MANIFEST_MAX_BYTES_ENV
            ),
        ),
        CappedReadError::InvalidUtf8(e) => Error::new(ErrorKind::InvalidData, e),
        other => Error::new(ErrorKind::InvalidData, other.to_string()),
    })?;
    String::from_utf8(bytes).map_err(|e| Error::new(ErrorKind::InvalidData, e.utf8_error()))
}

#[cfg(test)]
mod tests {
    use super::super::settings::{resolve_cargo_settings, HostTarget, Setting};
    use super::*;

    fn no_env(_: &str) -> Option<String> {
        None
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
            [nested.join(".cargo/config.toml"), root.join(".cargo/config")],
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
        std::fs::write(root.join(".cargo/config"), "[profile.dev]\nincremental = false\n")
            .expect("write");
        std::fs::write(root.join(".cargo/config.toml"), "[profile.dev]\nincremental = true\n")
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

    /// A malformed legacy `config` must not fall through to `config.toml`:
    /// cargo picks by existence, so a present-but-unparseable file
    /// contributes no layer at all rather than silently switching to the
    /// other name.
    #[test]
    fn malformed_legacy_config_does_not_fall_through_to_config_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join(".cargo")).expect("mkdir");
        std::fs::write(root.join(".cargo/config"), "not = = toml").expect("write");
        std::fs::write(root.join(".cargo/config.toml"), "[build]\njobs = 3\n").expect("write");

        let layers = config_layers(root, None);
        assert!(
            layers.is_empty(),
            "config.toml must not be consulted while the legacy config exists"
        );
    }

    /// Cargo merges `include = ["a.toml", "b.toml"]` in listed order with
    /// the including file on top, so layers rank: including file, `b`,
    /// `b`'s own includes (relative to `b`), then `a`. Optional missing
    /// files, non-`.toml` paths and cycles are skipped.
    #[test]
    fn config_includes_follow_their_file_in_cargo_precedence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let cargo = root.join(".cargo");
        std::fs::create_dir_all(cargo.join("sub")).expect("mkdir");
        let write = |rel: &str, text: &str| std::fs::write(cargo.join(rel), text).expect("write");
        write(
            "config.toml",
            "include = [\"a.toml\", { path = \"missing.toml\", optional = true }, \"x.json\", \"sub/b.toml\"]\n",
        );
        write("a.toml", "[profile.dev]\nincremental = true\n");
        write(
            "sub/b.toml",
            "include = [\"c.toml\", \"../config.toml\"]\n[profile.release]\nincremental = true\n",
        );
        write("sub/c.toml", "[profile.dev]\nincremental = false\n");

        let layers = config_layers(root, None);
        let paths: Vec<PathBuf> = layers.iter().map(|l| l.path.clone()).collect();
        assert_eq!(
            paths,
            [
                cargo.join("config.toml"),
                cargo.join("sub/b.toml"),
                cargo.join("sub/c.toml"),
                cargo.join("a.toml"),
            ]
        );
        assert_eq!(layers[1].base, cargo, "base is the included file's grandparent");

        let profiles = resolve_cargo_settings(&layers, &no_env, root, None, HostTarget::default())
            .incremental_profiles
            .expect("no global override");
        assert_eq!(
            profiles.dev,
            Setting {
                value: "false".to_string(),
                source: cargo.join("sub/c.toml").display().to_string(),
            },
            "the later include (b, via c) overrides the earlier one (a)"
        );
        assert_eq!(profiles.release.value, "true");

        // The including file's own value beats every include.
        write("config.toml", "include = [\"a.toml\"]\n[profile.dev]\nincremental = false\n");
        let layers = config_layers(root, None);
        let profiles = resolve_cargo_settings(&layers, &no_env, root, None, HostTarget::default())
            .incremental_profiles
            .expect("no global override");
        assert_eq!(profiles.dev.source, cargo.join("config.toml").display().to_string());
    }

    #[test]
    fn fold_dots_folds_parent_and_current_components() {
        assert_eq!(
            fold_dots(Path::new("/w/.cargo/sub/../config.toml")),
            PathBuf::from("/w/.cargo/config.toml")
        );
        assert_eq!(fold_dots(Path::new("/w/./.cargo/a.toml")), PathBuf::from("/w/.cargo/a.toml"));
        assert_eq!(fold_dots(Path::new("../x.toml")), PathBuf::from("../x.toml"));
    }

    /// Path fields in this module's tracing calls are recorded with `?`, so
    /// a repo-controlled config or include path carrying a newline or ANSI
    /// escape cannot forge a log record.
    #[test]
    fn config_path_debug_escapes_control_characters() {
        let path = PathBuf::from("/w/.cargo/evil\n\u{1b}[31mWARN forged.toml");
        crate::test_support::assert_debug_escapes_control_chars(path.display());
    }

    /// An `include` naming a FIFO is skipped promptly (with a warning)
    /// instead of blocking the page on `open(2)`; the including file and
    /// its other includes still load.
    #[cfg(unix)]
    #[test]
    fn fifo_include_is_skipped_without_blocking() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let cargo = root.join(".cargo");
        std::fs::create_dir_all(&cargo).expect("mkdir");
        std::fs::write(cargo.join("config.toml"), "include = [\"hang.toml\", \"ok.toml\"]\n")
            .expect("write");
        std::fs::write(cargo.join("ok.toml"), "[build]\njobs = 2\n").expect("write");
        let status = std::process::Command::new("mkfifo")
            .arg(cargo.join("hang.toml"))
            .status()
            .expect("run mkfifo");
        assert!(status.success(), "mkfifo failed");

        let (tx, rx) = std::sync::mpsc::channel();
        let cwd = root.to_path_buf();
        std::thread::spawn(move || {
            let paths: Vec<PathBuf> =
                config_layers(&cwd, None).into_iter().map(|l| l.path).collect();
            let _ = tx.send(paths);
        });
        let paths = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("config_layers blocked on a FIFO include");
        assert_eq!(paths, [cargo.join("config.toml"), cargo.join("ok.toml")]);
    }

    /// A directory where a config file is expected is refused, not read.
    #[test]
    fn non_regular_config_file_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = read_capped_regular_file(dir.path()).expect_err("a directory is not a config");
        // Not `NotFound`: `read_layer` must warn about it, not skip it silently.
        assert_eq!(err.kind(), std::io::ErrorKind::IsADirectory);
    }
}
