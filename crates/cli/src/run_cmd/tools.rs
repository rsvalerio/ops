//! External binaries a resolved step needs on `PATH`, for the `tools` list
//! of `ops explain --json` (TASK-2326).
//!
//! Derived statically from each step's expanded program and args — nothing
//! is spawned or probed, so the list says what a plan *needs*, not what is
//! installed:
//!
//! - an `ops` subcommand reports the tools that subcommand spawns (`sec` →
//!   Trivy, `deps` → cargo-edit / cargo-deny / cargo-machete, `msrv` →
//!   rustup), never `ops`
//!   itself;
//! - any other bare program name is a tool; a program given as a path
//!   (`./gradlew`, `/usr/bin/env`) lives in the repo or at a fixed location
//!   and is not reported;
//! - `cargo <sub>` also reports `cargo-<sub>` unless `<sub>` ships with
//!   cargo. A cargo alias from `.cargo/config.toml` reads as a plugin, and a
//!   global flag before the subcommand (`cargo -q nextest`) hides it.
//!
//! What a shell script runs (`sh -c "…"`) is opaque: only `sh` is reported.

use ops_core::config::ExecCommandSpec;

/// One external binary a step needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// The executable looked up on `PATH`.
    pub name: String,
    /// How to install it, when ops knows.
    pub install: Option<String>,
    /// The step still runs without it, skipping only what the tool backs.
    pub optional: bool,
}

impl Tool {
    fn required(name: impl Into<String>, install: Option<&str>) -> Self {
        Self {
            name: name.into(),
            install: install.map(str::to_string),
            optional: false,
        }
    }
}

/// Cargo's own subcommands (and their one-letter aliases): these need no
/// `cargo-<sub>` binary. `clippy` and `fmt` are absent on purpose — they are
/// the `cargo-clippy` / `cargo-fmt` binaries of rustup components.
const CARGO_BUILTINS: &[&str] = &[
    "add",
    "b",
    "bench",
    "build",
    "c",
    "check",
    "clean",
    "config",
    "d",
    "doc",
    "fetch",
    "fix",
    "generate-lockfile",
    "help",
    "info",
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
    "r",
    "read-manifest",
    "remove",
    "report",
    "rm",
    "run",
    "rustc",
    "rustdoc",
    "search",
    "t",
    "test",
    "tree",
    "uninstall",
    "update",
    "vendor",
    "verify-project",
    "version",
    "yank",
];

/// The tools one exec spec needs, given its already-expanded program and
/// args (see the module docs for the rules).
pub fn exec_tools(spec: &ExecCommandSpec, program: &str, args: &[String]) -> Vec<Tool> {
    if spec.display_program.as_deref() == Some("ops") || program == "ops" {
        return args
            .first()
            .map_or_else(Vec::new, |sub| ops_subcommand_tools(sub));
    }
    if program.contains(['/', '\\']) {
        return Vec::new();
    }
    let mut tools = vec![Tool::required(program, None)];
    if program == "cargo" {
        if let Some(plugin) = cargo_plugin(args) {
            tools.push(plugin);
        }
    }
    tools
}

/// The `cargo-<sub>` binary `cargo <args>` dispatches to, if any.
fn cargo_plugin(args: &[String]) -> Option<Tool> {
    let mut rest = args.iter().map(String::as_str);
    let mut sub = rest.next()?;
    if sub.starts_with('+') {
        sub = rest.next()?;
    }
    if sub.is_empty() || sub.starts_with('-') || CARGO_BUILTINS.contains(&sub) {
        return None;
    }
    let install = match sub {
        "clippy" => Some("rustup component add clippy"),
        "fmt" => Some("rustup component add rustfmt"),
        _ => None,
    };
    Some(Tool::required(format!("cargo-{sub}"), install))
}

/// Tools an `ops` subcommand spawns. Subcommands that spawn nothing report
/// nothing.
fn ops_subcommand_tools(sub: &str) -> Vec<Tool> {
    match sub {
        "sec" => vec![Tool::required("trivy", Some(crate::sec_cmd::TRIVY_INSTALL))],
        // `rustup run <rust-version> cargo check …` (TASK-2327).
        "msrv" => vec![Tool::required("rustup", Some("https://rustup.rs"))],
        #[cfg(feature = "stack-rust")]
        "deps" => ops_deps::external_tools()
            .into_iter()
            .map(|t| Tool {
                name: t.binary,
                install: Some(t.install),
                optional: t.optional,
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools(program: &str, args: &[&str]) -> Vec<String> {
        let args: Vec<String> = args.iter().map(ToString::to_string).collect();
        let spec = ExecCommandSpec::new(program, args.clone());
        exec_tools(&spec, program, &args)
            .into_iter()
            .map(|t| t.name)
            .collect()
    }

    #[test]
    fn cargo_plugins_are_reported_and_builtins_are_not() {
        assert_eq!(tools("cargo", &["build"]), ["cargo"]);
        assert_eq!(tools("cargo", &["t"]), ["cargo"]);
        assert_eq!(
            tools("cargo", &["nextest", "run"]),
            ["cargo", "cargo-nextest"]
        );
        assert_eq!(tools("cargo", &["+nightly", "fmt"]), ["cargo", "cargo-fmt"]);
        assert_eq!(tools("cargo", &["-q", "nextest"]), ["cargo"]);
        assert_eq!(tools("cargo", &[]), ["cargo"]);
    }

    #[test]
    fn path_programs_are_not_tools() {
        assert!(tools("./gradlew", &["build"]).is_empty());
        assert!(tools("/usr/bin/env", &["true"]).is_empty());
        assert_eq!(tools("npm", &["test"]), ["npm"]);
    }

    #[test]
    fn ops_subcommands_report_what_they_spawn() {
        let spec = ExecCommandSpec::ops_subcommand("sec");
        let got = exec_tools(&spec, &spec.program, &spec.args);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "trivy");
        assert!(got[0]
            .install
            .as_deref()
            .is_some_and(|i| i.contains("trivy")));
        assert!(tools("ops", &["end-of-file-fixer"]).is_empty());
        assert!(tools("ops", &["lint-actions"]).is_empty());
        assert_eq!(tools("ops", &["msrv"]), ["rustup"]);
        assert!(tools("ops", &[]).is_empty());
    }

    #[cfg(feature = "stack-rust")]
    #[test]
    fn deps_reports_its_cargo_plugins_with_machete_optional() {
        let got = exec_tools(
            &ExecCommandSpec::new("ops", ["deps"]),
            "ops",
            &["deps".to_string()],
        );
        let machete = got
            .iter()
            .find(|t| t.name == "cargo-machete")
            .expect("machete listed");
        assert!(machete.optional);
        assert!(got.iter().any(|t| t.name == "cargo-deny" && !t.optional));
    }
}
