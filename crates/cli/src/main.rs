//! CLI entry point and orchestration for ops.

// Force the linker to retain extension crates that only register via linkme
// distributed slices (no other symbols are referenced from the main binary).
#[cfg(feature = "stack-go")]
extern crate ops_about_go;
#[cfg(any(feature = "stack-java-maven", feature = "stack-java-gradle"))]
extern crate ops_about_java;
#[cfg(feature = "stack-rust")]
extern crate ops_cargo_update;
#[cfg(feature = "stack-rust")]
extern crate ops_metadata;
#[cfg(feature = "coverage")]
extern crate ops_test_coverage;
#[cfg(feature = "tokei")]
extern crate ops_tokei;

mod about_cmd;
mod args;
mod extension_cmd;
mod hook_shared;
mod init_cmd;
mod new_command_cmd;
mod registry;
mod run_before_commit_cmd;
mod run_before_push_cmd;
mod run_cmd;
mod theme_cmd;
#[cfg(feature = "stack-rust")]
mod tools_cmd;
mod tty;

#[cfg(test)]
mod test_utils;
#[cfg(test)]
pub(crate) use test_utils::CwdGuard;

use clap::FromArgMatches;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use args::*;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "Error: {:#}", e);
            ExitCode::FAILURE
        }
    }
}

fn init_logging() {
    let log_level = std::env::var("OPS_LOG_LEVEL")
        .map(|v| {
            v.parse().unwrap_or_else(|e| {
                tracing::debug!(
                    value = %v,
                    error = %e,
                    "EFF-002: invalid OPS_LOG_LEVEL, falling back to info"
                );
                tracing_subscriber::filter::LevelFilter::INFO.into()
            })
        })
        .unwrap_or_else(|_| {
            tracing::trace!("EFF-002: OPS_LOG_LEVEL not set, using default info");
            tracing_subscriber::filter::LevelFilter::INFO.into()
        });
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(io::stderr))
        .with(
            EnvFilter::from_default_env()
                .add_directive(log_level)
                .add_directive("tokei=error".parse().expect("static directive is valid")),
        )
        .init();
}

fn run() -> anyhow::Result<ExitCode> {
    init_logging();

    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let effective_args = preprocess_args(args);

    // Load config early so stack detection and help output can use it.
    let early_config = match ops_core::config::load_config() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "failed to load config; using defaults");
            ops_core::config::Config::default()
        }
    };
    let detected_stack = {
        let cwd = match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => {
                let _ = writeln!(
                    std::io::stderr(),
                    "ops: error: could not determine working directory: {e}"
                );
                return Ok(ExitCode::FAILURE);
            }
        };
        ops_core::stack::Stack::resolve(early_config.stack.as_deref(), &cwd)
    };

    // If the user asked for top-level help (`ops -h` / `ops --help`), show
    // help with dynamic commands included and exit.  We intercept before clap
    // parsing because dynamic subcommands cannot be registered at parse time
    // (they would shadow the `External` catch-all).
    if is_toplevel_help(&effective_args) {
        let cmd = hide_irrelevant_commands(Cli::command(), detected_stack);
        let long = effective_args.iter().any(|a| a == "--help");
        print_categorized_help(cmd, &early_config, detected_stack, long);
        return Ok(ExitCode::SUCCESS);
    }

    let cmd = hide_irrelevant_commands(Cli::command(), detected_stack);
    let mut matches = cmd.get_matches_from(effective_args);
    let cli = Cli::from_arg_matches_mut(&mut matches).unwrap_or_else(|e: clap::Error| e.exit());

    dispatch(cli, &early_config, detected_stack)
}

fn dispatch(
    cli: Cli,
    early_config: &ops_core::config::Config,
    detected_stack: Option<ops_core::stack::Stack>,
) -> anyhow::Result<ExitCode> {
    match cli.subcommand {
        Some(CoreSubcommand::Init {
            force,
            output,
            themes,
            commands,
        }) => {
            let sections = ops_core::config::InitSections::from_flags(output, themes, commands);
            init_cmd::run_init(force, sections)?;
        }
        Some(CoreSubcommand::Theme { action }) => run_theme(action)?,
        Some(CoreSubcommand::Extension { action }) => run_extension(action)?,
        Some(CoreSubcommand::NewCommand) => new_command_cmd::run_new_command()?,
        Some(CoreSubcommand::RunBeforeCommit {
            changed_only,
            action,
        }) => return run_before_commit(action, changed_only),
        Some(CoreSubcommand::RunBeforePush {
            changed_only,
            action,
        }) => return run_before_push(action, changed_only),
        Some(CoreSubcommand::About { refresh, action }) => run_about(refresh, action)?,
        #[cfg(feature = "stack-rust")]
        Some(CoreSubcommand::Deps { refresh }) => run_deps(refresh)?,
        Some(CoreSubcommand::Tools { action }) => {
            #[cfg(feature = "stack-rust")]
            {
                return run_tools(action);
            }
            #[cfg(not(feature = "stack-rust"))]
            {
                let _ = action;
                anyhow::bail!("tools subcommand requires the stack-rust feature");
            }
        }
        Some(CoreSubcommand::External(args)) => {
            return run_cmd::run_external_command(&args, cli.dry_run, cli.verbose, cli.tap)
        }
        None => {
            let cmd = hide_irrelevant_commands(Cli::command(), detected_stack);
            print_categorized_help(cmd, early_config, detected_stack, false);
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Returns true when the effective args request top-level help (no subcommand).
/// E.g. `ops -h`, `ops --help`, `ops -d --help`, but NOT `ops build -h`.
fn is_toplevel_help(args: &[std::ffi::OsString]) -> bool {
    // Skip argv[0].  If any non-flag argument appears before -h/--help, the
    // user is asking for subcommand help, not top-level help.
    let mut saw_help = false;
    for a in args.iter().skip(1) {
        if a == "-h" || a == "--help" {
            saw_help = true;
        } else if !a.to_string_lossy().starts_with('-') {
            // A positional (subcommand) appeared — not top-level help.
            return false;
        }
    }
    saw_help
}

/// Category assigned to built-in (clap-defined) subcommands.
fn builtin_category(name: &str) -> Option<&'static str> {
    match name {
        "about" => Some("Insights"),
        "deps" => Some("Code Quality"),
        "init" | "theme" | "extension" | "tools" => Some("Setup"),
        _ => Some("Commands"),
    }
}

/// A command entry used for categorized help output.
struct CmdEntry {
    name: String,
    about: String,
    category: Option<String>,
}

/// Collect built-in clap subcommands and dynamic config/stack commands into a
/// unified list of [`CmdEntry`] values.
///
/// Built-in subcommands are drawn from visible clap subcommands; dynamic
/// commands come from the user config and the detected stack defaults.
/// Duplicates (config overriding a stack default, or a dynamic name matching a
/// built-in) are suppressed so each command appears at most once.
fn collect_command_entries(
    cmd: &clap::Command,
    config: &ops_core::config::Config,
    stack: Option<ops_core::stack::Stack>,
) -> Vec<CmdEntry> {
    use std::collections::HashSet;

    let mut entries: Vec<CmdEntry> = Vec::new();

    // Visible built-in subcommands.
    let mut seen: HashSet<String> = HashSet::new();
    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() {
            continue;
        }
        let name = sub.get_name().to_string();
        let about = sub.get_about().map(|s| s.to_string()).unwrap_or_default();
        let category = builtin_category(&name).map(|s| s.to_string());
        seen.insert(name.clone());
        entries.push(CmdEntry {
            name,
            about,
            category,
        });
    }

    // Dynamic commands (config + stack defaults).
    let stack_commands = stack.map(|s| s.default_commands()).unwrap_or_default();
    let sources: Vec<(&str, &ops_core::config::CommandSpec)> = config
        .commands
        .iter()
        .map(|(n, s)| (n.as_str(), s))
        .chain(stack_commands.iter().map(|(n, s)| (n.as_str(), s)))
        .collect();

    for (name, spec) in sources {
        if !seen.insert(name.to_string()) {
            continue;
        }
        let about = hook_shared::command_description(spec);
        entries.push(CmdEntry {
            name: name.to_string(),
            about,
            category: Some(spec.category().unwrap_or("Commands").to_string()),
        });
    }

    entries
}

/// Sort command entries by category rank (per `category_order`), then by
/// category name, then alphabetically by command name.  Uncategorized entries
/// sort last.
fn sort_entries_by_category(entries: &mut [CmdEntry], category_order: &[String]) {
    let cat_rank = |cat: Option<&str>| -> usize {
        match cat {
            None => usize::MAX,
            Some(c) => category_order
                .iter()
                .position(|o| o == c)
                .unwrap_or(usize::MAX - 1),
        }
    };
    entries.sort_by(|a, b| {
        let ra = cat_rank(a.category.as_deref());
        let rb = cat_rank(b.category.as_deref());
        ra.cmp(&rb)
            .then_with(|| a.category.cmp(&b.category))
            .then(a.name.cmp(&b.name))
    });
}

/// Render sorted command entries into a grouped-sections string suitable for
/// insertion into the help output.
fn render_grouped_sections(entries: &[CmdEntry]) -> String {
    use std::fmt::Write;

    let max_name_width = entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
    let mut grouped = String::new();
    let mut current_category: Option<Option<&str>> = None;

    for entry in entries {
        let cat = entry.category.as_deref();
        if current_category.as_ref() != Some(&cat) {
            let heading = cat.unwrap_or("Commands");
            writeln!(grouped, "\n{heading}:").unwrap();
            current_category = Some(cat);
        }
        writeln!(
            grouped,
            "  {:<width$}  {}",
            entry.name,
            entry.about,
            width = max_name_width
        )
        .unwrap();
    }

    grouped
}

/// Print help with all commands (built-in and dynamic) grouped by category.
///
/// Collects built-in subcommands from the clap `Command`, merges them with
/// config/stack dynamic commands, groups everything by category, and renders
/// a unified help output.
fn print_categorized_help(
    mut cmd: clap::Command,
    config: &ops_core::config::Config,
    stack: Option<ops_core::stack::Stack>,
    long: bool,
) {
    // Build the clap command so subcommand metadata is fully resolved.
    cmd.build();

    let mut entries = collect_command_entries(&cmd, config, stack);
    sort_entries_by_category(&mut entries, &config.output.category_order);
    let grouped = render_grouped_sections(&entries);

    // Hide all subcommands so clap only renders about/usage/options.
    for name in cmd
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect::<Vec<_>>()
    {
        cmd = cmd.mut_subcommand(&name, |sub| sub.hide(true));
    }

    let help_str = if long {
        cmd.render_long_help().to_string()
    } else {
        cmd.render_help().to_string()
    };

    // Insert grouped commands before the "Options:" section.
    let mut stdout = std::io::stdout();
    if let Some(pos) = help_str.find("\nOptions:") {
        let _ = write!(stdout, "{}", &help_str[..pos]);
        let _ = write!(stdout, "{grouped}");
        let _ = write!(stdout, "{}", &help_str[pos..]);
    } else {
        let _ = write!(stdout, "{help_str}");
        let _ = write!(stdout, "{grouped}");
    }
}

pub(crate) fn load_config_and_cwd() -> anyhow::Result<(ops_core::config::Config, PathBuf)> {
    let config = ops_core::config::load_config()?;
    let cwd = std::env::current_dir()?;
    Ok((config, cwd))
}

fn run_about(refresh: bool, action: Option<AboutAction>) -> anyhow::Result<()> {
    let (config, cwd) = load_config_and_cwd()?;
    let registry = crate::registry::build_data_registry(&config, &cwd)?;
    match action {
        Some(AboutAction::Setup) => about_cmd::run_about_setup(&registry),
        #[cfg(feature = "duckdb")]
        Some(AboutAction::Code) => ops_about::run_about_code(&registry),
        #[cfg(not(feature = "duckdb"))]
        Some(AboutAction::Code) => {
            anyhow::bail!("about code requires the duckdb feature");
        }
        Some(AboutAction::Crates | AboutAction::Modules) => ops_about::run_about_units(&registry),
        Some(AboutAction::Coverage) => ops_about::run_about_coverage(&registry),
        Some(AboutAction::Dependencies) => ops_about::run_about_deps(&registry),
        None => {
            let columns = config.output.columns;
            let opts = ops_about::AboutOptions {
                refresh,
                visible_fields: config.about.fields.clone(),
            };
            ops_about::run_about(&registry, &opts, columns, &cwd, &mut std::io::stdout())
        }
    }
}

#[cfg(feature = "stack-rust")]
fn run_deps(refresh: bool) -> anyhow::Result<()> {
    let (config, cwd) = load_config_and_cwd()?;
    let registry = crate::registry::build_data_registry(&config, &cwd)?;
    let opts = ops_deps::DepsOptions { refresh };
    ops_deps::run_deps(&registry, &opts)
}

fn run_theme(action: ThemeAction) -> anyhow::Result<()> {
    match action {
        ThemeAction::List => theme_cmd::run_theme_list(),
        ThemeAction::Select => theme_cmd::run_theme_select(),
    }
}

fn run_extension(action: ExtensionAction) -> anyhow::Result<()> {
    match action {
        ExtensionAction::List => extension_cmd::run_extension_list(),
        ExtensionAction::Show { name } => extension_cmd::run_extension_show(name.as_deref()),
    }
}

/// Prompt the user to run `ops <hook> install` when the hook command is not configured.
fn prompt_hook_install(hook_name: &str) -> anyhow::Result<ExitCode> {
    let _ = writeln!(
        std::io::stderr(),
        "No '{hook_name}' command configured in .ops.toml."
    );
    if !crate::tty::is_stdout_tty() {
        let _ = writeln!(
            std::io::stderr(),
            "Run `ops {hook_name} install` to set it up."
        );
        return Ok(ExitCode::FAILURE);
    }
    let answer = inquire::Confirm::new(&format!("Run `ops {hook_name} install` now?"))
        .with_default(true)
        .prompt()?;
    if answer {
        let status = std::process::Command::new(std::env::current_exe()?)
            .args([hook_name, "install"])
            .status()?;
        if status.success() {
            return Ok(ExitCode::SUCCESS);
        }
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

fn run_before_commit(
    action: Option<RunBeforeCommitAction>,
    changed_only: bool,
) -> anyhow::Result<ExitCode> {
    match action {
        Some(RunBeforeCommitAction::Install) => {
            run_before_commit_cmd::run_before_commit_install()?;
            Ok(ExitCode::SUCCESS)
        }
        None => {
            let config = ops_core::config::load_config().map_err(|e| {
                anyhow::anyhow!("failed to load config for run-before-commit check: {e}")
            })?;
            if !config.commands.contains_key("run-before-commit") {
                return prompt_hook_install("run-before-commit");
            }
            if ops_run_before_commit::should_skip() {
                let _ = writeln!(
                    std::io::stderr(),
                    "[run-before-commit] {}=1 — skipping",
                    ops_run_before_commit::SKIP_ENV_VAR
                );
                return Ok(ExitCode::SUCCESS);
            }
            if changed_only && !ops_run_before_commit::has_staged_files()? {
                let _ = writeln!(
                    std::io::stderr(),
                    "[run-before-commit] no staged files — skipping"
                );
                return Ok(ExitCode::SUCCESS);
            }
            let args = vec![std::ffi::OsString::from("run-before-commit")];
            run_cmd::run_external_command(&args, false, false, None)
        }
    }
}

fn run_before_push(
    action: Option<RunBeforePushAction>,
    _changed_only: bool,
) -> anyhow::Result<ExitCode> {
    match action {
        Some(RunBeforePushAction::Install) => {
            run_before_push_cmd::run_before_push_install()?;
            Ok(ExitCode::SUCCESS)
        }
        None => {
            let config = ops_core::config::load_config().map_err(|e| {
                anyhow::anyhow!("failed to load config for run-before-push check: {e}")
            })?;
            if !config.commands.contains_key("run-before-push") {
                return prompt_hook_install("run-before-push");
            }
            if ops_run_before_push::should_skip() {
                let _ = writeln!(
                    std::io::stderr(),
                    "[run-before-push] {}=1 — skipping",
                    ops_run_before_push::SKIP_ENV_VAR
                );
                return Ok(ExitCode::SUCCESS);
            }
            let args = vec![std::ffi::OsString::from("run-before-push")];
            run_cmd::run_external_command(&args, false, false, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- is_toplevel_help --

    fn os(args: &[&str]) -> Vec<std::ffi::OsString> {
        args.iter().map(|s| std::ffi::OsString::from(*s)).collect()
    }

    #[test]
    fn is_toplevel_help_short() {
        assert!(is_toplevel_help(&os(&["ops", "-h"])));
    }

    #[test]
    fn is_toplevel_help_long() {
        assert!(is_toplevel_help(&os(&["ops", "--help"])));
    }

    #[test]
    fn is_toplevel_help_with_flags() {
        assert!(is_toplevel_help(&os(&["ops", "-d", "--help"])));
    }

    #[test]
    fn is_toplevel_help_subcommand_help_short() {
        assert!(!is_toplevel_help(&os(&["ops", "build", "-h"])));
    }

    #[test]
    fn is_toplevel_help_subcommand_help_long() {
        assert!(!is_toplevel_help(&os(&["ops", "build", "--help"])));
    }

    #[test]
    fn is_toplevel_help_no_help_flag() {
        assert!(!is_toplevel_help(&os(&["ops", "-d"])));
    }

    #[test]
    fn is_toplevel_help_no_args() {
        assert!(!is_toplevel_help(&os(&["ops"])));
    }

    // -- builtin_category --

    #[test]
    fn builtin_category_about() {
        assert_eq!(builtin_category("about"), Some("Insights"));
    }

    #[test]
    fn builtin_category_deps() {
        assert_eq!(builtin_category("deps"), Some("Code Quality"));
    }

    #[test]
    fn builtin_category_setup_commands() {
        for name in &["init", "theme", "extension", "tools"] {
            assert_eq!(builtin_category(name), Some("Setup"), "failed for {name}");
        }
    }

    #[test]
    fn builtin_category_unknown_returns_commands() {
        assert_eq!(builtin_category("build"), Some("Commands"));
        assert_eq!(builtin_category("verify"), Some("Commands"));
    }

    // -- sort_entries_by_category --

    fn entry(name: &str, category: Option<&str>) -> CmdEntry {
        CmdEntry {
            name: name.to_string(),
            about: String::new(),
            category: category.map(|s| s.to_string()),
        }
    }

    #[test]
    fn sort_entries_by_category_respects_order() {
        let mut entries = vec![
            entry("tools", Some("Setup")),
            entry("build", Some("Commands")),
            entry("about", Some("Insights")),
        ];
        let order = vec![
            "Commands".to_string(),
            "Insights".to_string(),
            "Setup".to_string(),
        ];
        sort_entries_by_category(&mut entries, &order);
        assert_eq!(entries[0].name, "build");
        assert_eq!(entries[1].name, "about");
        assert_eq!(entries[2].name, "tools");
    }

    #[test]
    fn sort_entries_by_category_alphabetical_within_category() {
        let mut entries = vec![
            entry("test", Some("Commands")),
            entry("build", Some("Commands")),
            entry("verify", Some("Commands")),
        ];
        sort_entries_by_category(&mut entries, &["Commands".to_string()]);
        assert_eq!(entries[0].name, "build");
        assert_eq!(entries[1].name, "test");
        assert_eq!(entries[2].name, "verify");
    }

    #[test]
    fn sort_entries_by_category_uncategorized_last() {
        let mut entries = vec![entry("mystery", None), entry("build", Some("Commands"))];
        sort_entries_by_category(&mut entries, &["Commands".to_string()]);
        assert_eq!(entries[0].name, "build");
        assert_eq!(entries[1].name, "mystery");
    }

    #[test]
    fn sort_entries_by_category_unknown_category_before_uncategorized() {
        let mut entries = vec![
            entry("mystery", None),
            entry("lint", Some("UnknownCat")),
            entry("build", Some("Commands")),
        ];
        sort_entries_by_category(&mut entries, &["Commands".to_string()]);
        assert_eq!(entries[0].name, "build");
        assert_eq!(entries[1].name, "lint");
        assert_eq!(entries[2].name, "mystery");
    }

    // -- render_grouped_sections --

    #[test]
    fn render_grouped_sections_groups_by_category() {
        let entries = vec![
            entry("build", Some("Commands")),
            entry("test", Some("Commands")),
            entry("about", Some("Insights")),
        ];
        let output = render_grouped_sections(&entries);
        assert!(output.contains("\nCommands:\n"));
        assert!(output.contains("\nInsights:\n"));
        assert!(output.contains("  build"));
        assert!(output.contains("  test"));
        assert!(output.contains("  about"));
    }

    #[test]
    fn render_grouped_sections_aligns_names() {
        let entries = vec![
            entry("ab", Some("Commands")),
            entry("longname", Some("Commands")),
        ];
        let output = render_grouped_sections(&entries);
        // Both names should have the same alignment width
        assert!(output.contains("  ab      "));
        assert!(output.contains("  longname"));
    }

    #[test]
    fn render_grouped_sections_uncategorized_shows_commands_heading() {
        let entries = vec![entry("mystery", None)];
        let output = render_grouped_sections(&entries);
        assert!(output.contains("\nCommands:\n"));
    }

    #[test]
    fn render_grouped_sections_empty_entries() {
        let entries: Vec<CmdEntry> = vec![];
        let output = render_grouped_sections(&entries);
        assert!(output.is_empty());
    }

    // -- collect_command_entries --

    #[test]
    fn collect_command_entries_includes_dynamic_commands() {
        let mut cmd = clap::Command::new("ops");
        cmd = cmd.subcommand(clap::Command::new("init").about("Initialize config"));

        let mut config = ops_core::config::Config::default();
        config.commands.insert(
            "build".to_string(),
            ops_core::config::CommandSpec::Exec(ops_core::config::ExecCommandSpec {
                program: "cargo".to_string(),
                args: vec!["build".to_string()],
                ..Default::default()
            }),
        );

        let entries = collect_command_entries(&cmd, &config, None);
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"init"), "should include built-in init");
        assert!(names.contains(&"build"), "should include dynamic build");
    }

    #[test]
    fn collect_command_entries_deduplicates() {
        let mut cmd = clap::Command::new("ops");
        cmd = cmd.subcommand(clap::Command::new("build").about("Built-in build"));

        let mut config = ops_core::config::Config::default();
        config.commands.insert(
            "build".to_string(),
            ops_core::config::CommandSpec::Exec(ops_core::config::ExecCommandSpec {
                program: "make".to_string(),
                ..Default::default()
            }),
        );

        let entries = collect_command_entries(&cmd, &config, None);
        let build_count = entries.iter().filter(|e| e.name == "build").count();
        assert_eq!(build_count, 1, "build should appear only once");
    }

    #[test]
    fn collect_command_entries_hides_hidden_subcommands() {
        let mut cmd = clap::Command::new("ops");
        cmd = cmd.subcommand(clap::Command::new("visible").about("Visible"));
        cmd = cmd.subcommand(clap::Command::new("hidden").about("Hidden").hide(true));

        let config = ops_core::config::Config::default();
        let entries = collect_command_entries(&cmd, &config, None);
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"visible"));
        assert!(!names.contains(&"hidden"));
    }

    #[test]
    fn collect_command_entries_with_stack_adds_defaults() {
        let cmd = clap::Command::new("ops");
        let config = ops_core::config::Config::default();
        let entries = collect_command_entries(&cmd, &config, Some(ops_core::stack::Stack::Rust));
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        // Rust stack should add default commands like build, test, verify
        assert!(names.contains(&"build"), "should include Rust stack build");
        assert!(names.contains(&"test"), "should include Rust stack test");
    }
}

#[cfg(feature = "stack-rust")]
fn run_tools(action: ToolsAction) -> anyhow::Result<ExitCode> {
    match action {
        ToolsAction::List => {
            tools_cmd::run_tools_list()?;
            Ok(ExitCode::SUCCESS)
        }
        ToolsAction::Check => tools_cmd::run_tools_check(),
        ToolsAction::Install { name } => tools_cmd::run_tools_install(name.as_deref()),
    }
}
