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
        // `--` is clap's end-of-options marker. Anything after it is a
        // positional / pass-through, even when it begins with `-`. So
        // `ops -- --help` should NOT be treated as top-level help; the
        // subcommand catch-all should receive the args verbatim.
        if a == "--" {
            return false;
        }
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
    use ops_core::output::display_width;
    // READ-2 / TASK-0734: width must be measured in display columns, not
    // bytes — `String::len` undercounts CJK / wide / combining characters
    // and mis-aligns the column when extension-supplied command names
    // contain non-ASCII text. Pair this with a manual space-pad below so
    // `{:<width$}` (which sizes in `char` count, not display width) cannot
    // re-introduce the same drift.
    let max_name_width = entries
        .iter()
        .map(|e| display_width(&e.name))
        .max()
        .unwrap_or(0);
    let mut grouped = String::new();
    let mut current_category: Option<Option<&str>> = None;

    for entry in entries {
        let cat = entry.category.as_deref();
        if current_category.as_ref() != Some(&cat) {
            let heading = cat.unwrap_or("Commands");
            grouped.push_str(&format!("\n{heading}:\n"));
            current_category = Some(cat);
        }
        let name_cols = display_width(&entry.name);
        let pad = max_name_width.saturating_sub(name_cols);
        grouped.push_str("  ");
        grouped.push_str(&entry.name);
        for _ in 0..pad {
            grouped.push(' ');
        }
        grouped.push_str("  ");
        grouped.push_str(&entry.about);
        grouped.push('\n');
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
    // CL-3 / TASK-0708: anchor on the blank line + heading-at-column-0 form
    // (`\n\nOptions:`) so a subcommand `about` that itself contains the
    // substring "Options:" cannot win the search and have the grouped
    // section spliced into the middle of an unrelated help line. clap
    // always emits a blank line before each top-level section heading; if
    // that ever changes the splice falls through to the append branch
    // rather than corrupting the help layout silently.
    if let Some(pos) = help_str.find("\n\nOptions:") {
        let split = pos + 1;
        out.push_str(&help_str[..split]);
        out.push_str(grouped);
        out.push_str(&help_str[split..]);
    } else {
        out.push_str(help_str);
        out.push_str(grouped);
    }
    out
}

/// Render categorized help and write it to `writer`. Extracted from
/// [`print_categorized_help`] so the write path can be exercised against a
/// failing writer (ERR-1 / TASK-0760) without needing to redirect stdout.
pub(crate) fn write_categorized_help(
    writer: &mut dyn Write,
    cmd: clap::Command,
    config: &ops_core::config::Config,
    stack: Option<ops_core::stack::Stack>,
    long: bool,
) -> std::io::Result<()> {
    let out = render_categorized_help(cmd, config, stack, long);
    write!(writer, "{out}")
}

/// Print help with all commands (built-in and dynamic) grouped by category.
pub(crate) fn print_categorized_help(
    cmd: clap::Command,
    config: &ops_core::config::Config,
    stack: Option<ops_core::stack::Stack>,
    long: bool,
) {
    // ERR-1 / TASK-0760: mirrors the parse_log_level rationale at
    // main.rs:89-93 — if stdout has gone away (closed pipe like
    // `ops --help | head -5`, or a consumer that exited mid-write), the
    // user has already lost the channel we would report the error on.
    // Surfacing the error here would only turn a benign EPIPE into a
    // non-zero startup exit; swallow it so the help-rendering path stays
    // pipe-friendly. The error is still reachable via `write_categorized_help`
    // for callers that need to react to it.
    let _ = write_categorized_help(&mut std::io::stdout(), cmd, config, stack, long);
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
    fn is_toplevel_help_double_dash_separator_not_toplevel() {
        // PATTERN-1 / TASK-0514: `--` is clap's end-of-options marker, so
        // anything after it must reach the subcommand catch-all unchanged.
        assert!(!is_toplevel_help(&os(&["ops", "--", "--help"])));
        assert!(!is_toplevel_help(&os(&["ops", "--"])));
        assert!(!is_toplevel_help(&os(&["ops", "-d", "--", "--help"])));
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
    fn render_grouped_sections_aligns_wide_command_names_by_display_width() {
        // READ-2 / TASK-0734: extension- and config-defined command names
        // are unrestricted, so a name like `ビルド` (display width 6, byte
        // length 9) or `🚀deploy` (wide emoji + ASCII) must not skew the
        // column padding. Pre-fix this used `String::len` (bytes) and
        // `{:<width$}` (chars), so wide names landed too far left and the
        // following column was mis-aligned.
        let entries = vec![
            entry("\u{30D3}\u{30EB}\u{30C9}", Some("Commands")), // ビルド, display width 6
            entry("ascii", Some("Commands")),                    // display width 5
        ];
        let mut entries_with_about = entries;
        for e in &mut entries_with_about {
            e.about = "desc".to_string();
        }

        let output = render_grouped_sections(&entries_with_about);
        // Each command line should pad the name out to the wider column
        // (6) before the two-space gap and the description, so descriptions
        // line up. We assert this by checking the column at which "desc"
        // starts on each line is identical when measured in display width.
        let lines: Vec<&str> = output.lines().filter(|l| l.contains("desc")).collect();
        assert_eq!(lines.len(), 2, "rendered lines:\n{output}");

        let desc_columns: Vec<usize> = lines
            .iter()
            .map(|l| {
                let idx = l.find("desc").expect("desc present");
                ops_core::output::display_width(&l[..idx])
            })
            .collect();
        assert_eq!(
            desc_columns[0], desc_columns[1],
            "description columns not aligned by display width: {desc_columns:?}\n{output}"
        );
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

        let mut config = ops_core::config::Config::empty();
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

        let mut config = ops_core::config::Config::empty();
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

        let config = ops_core::config::Config::empty();
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
        let config = ops_core::config::Config::empty();

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
    fn splice_grouped_into_help_ignores_options_substring_in_subcommand_about() {
        // CL-3 / TASK-0708: prior to the line-anchored find, a subcommand
        // whose `about` text contained the substring "Options:" (e.g.
        // "Override CLI options:") would match before the real section
        // heading and the grouped section would land in the middle of the
        // subcommand description. The new anchor (blank line + heading at
        // column 0) rejects the embedded substring.
        let help = concat!(
            "Usage: ops [OPTIONS] [COMMAND]\n",
            "\n",
            "Commands:\n",
            "  cfg   Override CLI options: see manual\n",
            "\n",
            "Options:\n",
            "  -v, --verbose\n",
        );
        let grouped = "\nSetup:\n  init  Initialize\n";
        let out = splice_grouped_into_help(help, grouped);

        let grouped_pos = out.find("\nSetup:\n").expect("grouped heading was spliced");
        let real_options_pos = out
            .find("\nOptions:\n")
            .expect("real Options: section is preserved");
        assert!(
            grouped_pos < real_options_pos,
            "grouped section spliced before real Options block, not into the subcommand line:\n{out}"
        );
        // The subcommand description line must remain intact — the grouped
        // section must NOT have been inserted in the middle of it.
        assert!(
            out.contains("cfg   Override CLI options: see manual\n"),
            "subcommand line was corrupted: {out}"
        );
    }

    #[test]
    fn splice_grouped_into_help_appends_when_no_options_section() {
        let help = "usage: ops\n\nAbout:\n  blah\n";
        let grouped = "\nCommands:\n  build  Build it\n";
        let out = splice_grouped_into_help(help, grouped);
        assert!(out.ends_with(grouped), "grouped appended: {out}");
        assert!(out.starts_with("usage: ops"));
    }

    /// ERR-1 / TASK-0760: a writer that fails (e.g. closed pipe) must
    /// surface the error from `write_categorized_help` instead of being
    /// silently dropped. The print wrapper is the only place that swallows
    /// it, with a documented rationale.
    #[test]
    fn write_categorized_help_surfaces_writer_errors() {
        struct FailingWriter;
        impl Write for FailingWriter {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "consumer gone",
                ))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let cmd = clap::Command::new("ops").subcommand(clap::Command::new("init"));
        let config = ops_core::config::Config::empty();
        let err = write_categorized_help(&mut FailingWriter, cmd, &config, None, false)
            .expect_err("failing writer must surface its error");
        assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn collect_command_entries_with_stack_adds_defaults() {
        let cmd = clap::Command::new("ops");
        let config = ops_core::config::Config::empty();
        let entries = collect_command_entries(&cmd, &config, Some(ops_core::stack::Stack::Rust));
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"build"), "should include Rust stack build");
        assert!(names.contains(&"test"), "should include Rust stack test");
    }
}
