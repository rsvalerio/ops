//! Help rendering with categorized grouping of built-in and dynamic commands.

use std::io::Write;

use crate::hook_shared;

/// Returns true when the effective args request top-level help (no subcommand).
/// E.g. `ops -h`, `ops --help`, `ops -d --help`, but NOT `ops build -h`.
pub(crate) fn is_toplevel_help(args: &[std::ffi::OsString]) -> bool {
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
///
/// Every known built-in maps to an explicit category; unknown names fall back
/// to "Commands". Returns a plain `&'static str` rather than `Option` because
/// no caller ever needs to distinguish "unmapped" from a default — the former
/// is simply a name this function does not yet know about.
pub(crate) fn builtin_category(name: &str) -> &'static str {
    match name {
        "about" => "Insights",
        "deps" => "Code Quality",
        "init" | "theme" | "extension" | "tools" | "run-before-commit" | "run-before-push" => {
            "Setup"
        }
        _ => "Commands",
    }
}

/// A command entry used for categorized help output.
pub(crate) struct CmdEntry {
    pub name: String,
    pub about: String,
    pub category: Option<String>,
}

/// Collect built-in clap subcommands and dynamic config/stack commands into a
/// unified list of [`CmdEntry`] values.
pub(crate) fn collect_command_entries(
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
        let category = Some(builtin_category(&name).to_string());
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
pub(crate) fn sort_entries_by_category(entries: &mut [CmdEntry], category_order: &[String]) {
    /// Explicit rank classes for `cat_rank`. Higher values sort later.
    /// Known categories map to their index in `category_order`; unknown
    /// categories sort after all known ones; `None` sorts last of all.
    ///
    /// Using an enum instead of `usize::MAX` / `usize::MAX - 1` sentinels
    /// makes the three-way split load-bearing in the type, not buried in
    /// magic numbers.
    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    enum CatRank {
        Known(usize),
        Unknown,
        Uncategorized,
    }
    let cat_rank = |cat: Option<&str>| -> CatRank {
        match cat {
            None => CatRank::Uncategorized,
            Some(c) => category_order
                .iter()
                .position(|o| o == c)
                .map_or(CatRank::Unknown, CatRank::Known),
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
pub(crate) fn render_grouped_sections(entries: &[CmdEntry]) -> String {
    let max_name_width = entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
    let mut grouped = String::new();
    let mut current_category: Option<Option<&str>> = None;

    for entry in entries {
        let cat = entry.category.as_deref();
        if current_category.as_ref() != Some(&cat) {
            let heading = cat.unwrap_or("Commands");
            grouped.push_str(&format!("\n{heading}:\n"));
            current_category = Some(cat);
        }
        grouped.push_str(&format!(
            "  {:<width$}  {}\n",
            entry.name,
            entry.about,
            width = max_name_width
        ));
    }

    grouped
}

/// Build the categorized help string with `grouped` spliced before the
/// `Options:` section (or appended if no `Options:` block exists). Extracted
/// from [`print_categorized_help`] so it's exercised directly in unit tests
/// rather than only via stdout.
pub(crate) fn render_categorized_help(
    mut cmd: clap::Command,
    config: &ops_core::config::Config,
    stack: Option<ops_core::stack::Stack>,
    long: bool,
) -> String {
    cmd.build();

    let mut entries = collect_command_entries(&cmd, config, stack);
    sort_entries_by_category(&mut entries, &config.output.category_order);
    let grouped = render_grouped_sections(&entries);

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

    splice_grouped_into_help(&help_str, &grouped)
}

fn splice_grouped_into_help(help_str: &str, grouped: &str) -> String {
    let mut out = String::with_capacity(help_str.len() + grouped.len());
    if let Some(pos) = help_str.find("\nOptions:") {
        out.push_str(&help_str[..pos]);
        out.push_str(grouped);
        out.push_str(&help_str[pos..]);
    } else {
        out.push_str(help_str);
        out.push_str(grouped);
    }
    out
}

/// Print help with all commands (built-in and dynamic) grouped by category.
pub(crate) fn print_categorized_help(
    cmd: clap::Command,
    config: &ops_core::config::Config,
    stack: Option<ops_core::stack::Stack>,
    long: bool,
) {
    let out = render_categorized_help(cmd, config, stack, long);
    let _ = write!(std::io::stdout(), "{out}");
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn builtin_category_about() {
        assert_eq!(builtin_category("about"), "Insights");
    }

    #[test]
    fn builtin_category_deps() {
        assert_eq!(builtin_category("deps"), "Code Quality");
    }

    #[test]
    fn builtin_category_setup_commands() {
        for name in &[
            "init",
            "theme",
            "extension",
            "tools",
            "run-before-commit",
            "run-before-push",
        ] {
            assert_eq!(builtin_category(name), "Setup", "failed for {name}");
        }
    }

    #[test]
    fn builtin_category_unknown_returns_commands() {
        assert_eq!(builtin_category("build"), "Commands");
        assert_eq!(builtin_category("verify"), "Commands");
    }

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

    #[test]
    fn collect_command_entries_includes_dynamic_commands() {
        let mut cmd = clap::Command::new("ops");
        cmd = cmd.subcommand(clap::Command::new("init").about("Initialize config"));

        let mut config = ops_core::config::Config::default();
        config.commands.insert(
            "build".to_string(),
            ops_core::config::CommandSpec::Exec(ops_core::config::ExecCommandSpec::new(
                "cargo",
                ["build"],
            )),
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
            ops_core::config::CommandSpec::Exec(ops_core::config::ExecCommandSpec::new(
                "make",
                Vec::<String>::new(),
            )),
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
    fn render_categorized_help_splices_grouped_before_options() {
        let mut cmd = clap::Command::new("ops")
            .about("ops test")
            .subcommand(clap::Command::new("init").about("Initialize"));
        cmd = cmd.arg(clap::Arg::new("verbose").short('v').long("verbose"));
        let config = ops_core::config::Config::default();

        let out = render_categorized_help(cmd, &config, None, false);
        // The grouped section must appear before the Options: block, not after.
        let options_pos = out
            .find("Options:")
            .expect("rendered help contains Options:");
        let commands_pos = out
            .find("\nSetup:\n")
            .or_else(|| out.find("\nCommands:\n"))
            .expect("grouped heading was spliced in");
        assert!(
            commands_pos < options_pos,
            "grouped section should precede Options:\n{out}"
        );
        assert!(out.contains("init"), "init entry is rendered: {out}");
    }

    #[test]
    fn splice_grouped_into_help_appends_when_no_options_section() {
        let help = "usage: ops\n\nAbout:\n  blah\n";
        let grouped = "\nCommands:\n  build  Build it\n";
        let out = splice_grouped_into_help(help, grouped);
        assert!(out.ends_with(grouped), "grouped appended: {out}");
        assert!(out.starts_with("usage: ops"));
    }

    #[test]
    fn collect_command_entries_with_stack_adds_defaults() {
        let cmd = clap::Command::new("ops");
        let config = ops_core::config::Config::default();
        let entries = collect_command_entries(&cmd, &config, Some(ops_core::stack::Stack::Rust));
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"build"), "should include Rust stack build");
        assert!(names.contains(&"test"), "should include Rust stack test");
    }
}
