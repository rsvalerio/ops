//! `ops import-makefile` — interactively import Makefile targets as commands.
//!
//! Parses the project's `Makefile`, presents a multi-select checklist of its
//! targets, and appends each selected one to `.ops.toml` as
//! `[commands.<target>] program = "make" args = ["<target>"]` so it becomes
//! runnable as `ops <target>`. Running through `make` (rather than inlining
//! the recipe) preserves prerequisite chains like `check: fmt clippy test`
//! and multi-line recipes verbatim.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context as _;

use ops_core::config::{command_names, edit_ops_toml, ensure_table, insert_command};

use crate::tty::SelectOption;
use crate::SIGINT_EXIT;

/// Filenames probed for in `make`'s own lookup order.
const MAKEFILE_NAMES: &[&str] = &["GNUmakefile", "makefile", "Makefile"];

/// A parsed Makefile target: its name plus the optional `## description`
/// doc comment (the `make help` self-documentation convention).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MakeTarget {
    pub name: String,
    pub description: Option<String>,
}

pub fn run_import_makefile(
    workspace_root: &Path,
    file: Option<PathBuf>,
) -> anyhow::Result<ExitCode> {
    run_import_makefile_with_tty_check(workspace_root, file, crate::tty::is_stdout_tty)
}

fn run_import_makefile_with_tty_check<F>(
    workspace_root: &Path,
    file: Option<PathBuf>,
    is_tty: F,
) -> anyhow::Result<ExitCode>
where
    F: FnOnce() -> bool,
{
    crate::tty::require_tty_with("import-makefile", is_tty)?;

    let mut stdout = io::stdout();
    let importable = load_importable_targets(workspace_root, file, &mut stdout)?;

    let chosen = match prompt_target_selection(&importable)? {
        // Esc / Ctrl-C is a user-initiated cancel, surfaced with the shared
        // SIGINT exit convention (mirrors `prompt_hook_install`) rather
        // than an `ops: error:` frame and exit 1.
        None => {
            writeln!(stdout, "Cancelled; .ops.toml left untouched.")?;
            return Ok(ExitCode::from(SIGINT_EXIT));
        }
        Some(chosen) if chosen.is_empty() => {
            writeln!(stdout, "Nothing selected; .ops.toml left untouched.")?;
            return Ok(ExitCode::SUCCESS);
        }
        Some(chosen) => chosen,
    };

    append_targets_to_config(workspace_root, &chosen)?;
    write_imported_confirmation(&mut stdout, &chosen)?;
    Ok(ExitCode::SUCCESS)
}

/// Resolve, read, and parse the Makefile; emit the include-directive note
/// and per-target skip notes to `stdout`; error when no importable targets
/// remain.
fn load_importable_targets<W: Write>(
    workspace_root: &Path,
    file: Option<PathBuf>,
    stdout: &mut W,
) -> anyhow::Result<Vec<MakeTarget>> {
    let makefile_path = resolve_makefile_path(workspace_root, file)?;
    // ERR-4: .with_context preserves the io::Error as source() (rendered
    // identically via anyhow's chained Display) instead of flattening it
    // into the message string.
    let content = std::fs::read_to_string(&makefile_path)
        .with_context(|| format!("could not read {}", makefile_path.display()))?;

    let includes = count_include_directives(&content);
    let targets = parse_targets(&content);
    if targets.is_empty() {
        if includes > 0 {
            anyhow::bail!(
                "no targets found in {} ({includes} include directive(s) were not followed; \
                 targets defined in included makefiles are not visible)",
                makefile_path.display()
            );
        }
        anyhow::bail!("no targets found in {}", makefile_path.display());
    }

    let (importable, skipped) = partition_importable(workspace_root, targets)?;
    if includes > 0 {
        writeln!(
            stdout,
            "Note: {includes} include directive(s) not followed; \
             targets defined in included makefiles are not listed."
        )?;
    }
    write_skipped_notes(stdout, &skipped)?;
    if importable.is_empty() {
        anyhow::bail!(
            "no importable targets left in {} (all skipped)",
            makefile_path.display()
        );
    }
    Ok(importable)
}

/// Run the interactive `MultiSelect` over the importable targets. Returns
/// `None` when the user cancels (Esc / Ctrl-C — the help message advertises
/// "esc to cancel", so honour it), `Some(chosen)` otherwise (possibly empty
/// when everything was deselected).
fn prompt_target_selection(importable: &[MakeTarget]) -> anyhow::Result<Option<Vec<&MakeTarget>>> {
    let options: Vec<SelectOption> = importable
        .iter()
        .map(|t| SelectOption {
            name: t.name.clone(),
            description: t
                .description
                .clone()
                .unwrap_or_else(|| format!("make {}", t.name)),
        })
        .collect();
    // Preselect everything — unchecking is cheaper than re-checking a long list.
    let all: Vec<usize> = (0..options.len()).collect();
    let selected = match inquire::MultiSelect::new("Select Makefile targets to import:", options)
        .with_default(&all)
        .with_help_message("space to toggle, enter to confirm, esc to cancel")
        .prompt()
    {
        Ok(selected) => selected,
        Err(
            inquire::InquireError::OperationCanceled | inquire::InquireError::OperationInterrupted,
        ) => return Ok(None),
        Err(e) => {
            return Err(anyhow::Error::new(e).context("target selection prompt failed"));
        }
    };
    let chosen = selected
        .iter()
        .map(|opt| {
            importable
                .iter()
                .find(|t| t.name == opt.name)
                .expect("selected option originates from importable targets")
        })
        .collect();
    Ok(Some(chosen))
}

/// Resolve which Makefile to parse: an explicit `--file` wins, otherwise the
/// first of `GNUmakefile` / `makefile` / `Makefile` under `workspace_root`.
fn resolve_makefile_path(workspace_root: &Path, file: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(file) = file {
        let path = if file.is_absolute() {
            file
        } else {
            workspace_root.join(file)
        };
        if !path.is_file() {
            anyhow::bail!("Makefile not found: {}", path.display());
        }
        return Ok(path);
    }
    MAKEFILE_NAMES
        .iter()
        .map(|name| workspace_root.join(name))
        .find(|p| p.is_file())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no Makefile found in {} (looked for {})",
                workspace_root.display(),
                MAKEFILE_NAMES.join(", ")
            )
        })
}

/// Make directives whose first token must never be parsed as a target
/// name, even when the line contains a `:` (e.g. `export FOO: bar` is a
/// target-specific variable assignment, not a rule for `export`/`FOO`).
const DIRECTIVES: &[&str] = &[
    "export", "unexport", "override", "private", "vpath", "define", "endef", "undefine", "include",
    "-include", "sinclude", "ifeq", "ifneq", "ifdef", "ifndef", "else", "endif",
];

/// True when `line` is a recipe line under the current `.RECIPEPREFIX`.
/// Shared by [`parse_targets`] and [`count_include_directives`] so the two
/// scanners cannot diverge on what counts as a recipe line (PATTERN-1 /
/// TASK-1653).
fn is_recipe_line(line: &str, recipe_prefix: char) -> bool {
    line.starts_with('\t') || line.starts_with(recipe_prefix)
}

/// Observe a (trim-started) line for a `.RECIPEPREFIX` reassignment,
/// updating `recipe_prefix` when it carries an assignment. Returns `true`
/// when the line is a `.RECIPEPREFIX` directive (the caller skips it).
///
/// GNU make 3.82+: `.RECIPEPREFIX = >` switches the recipe-line marker
/// from tab to `>`; an empty assignment switches back to tab. Guards
/// against `.RECIPEPREFIX`-prefixed identifiers, which return `false` and
/// fall through to the caller's own filters.
fn observe_recipe_prefix(trimmed: &str, recipe_prefix: &mut char) -> bool {
    let Some(rest) = trimmed.strip_prefix(".RECIPEPREFIX") else {
        return false;
    };
    if !(rest.is_empty() || rest.starts_with([' ', '\t', '=', ':', '?', '+'])) {
        return false;
    }
    if let Some((_, value)) = rest.split_once('=') {
        *recipe_prefix = value.trim().chars().next().unwrap_or('\t');
    }
    true
}

/// Count `include` / `-include` / `sinclude` directive lines. The parser is
/// single-file on purpose, so targets defined in included makefiles are
/// invisible — callers surface the count as a note (partial picker) or an
/// error hint (no targets at all) instead of failing silently.
///
/// Tracks `.RECIPEPREFIX` like [`parse_targets`] so a custom-prefix recipe
/// line such as `>include extra.conf` is not miscounted as a directive.
pub(crate) fn count_include_directives(content: &str) -> usize {
    let mut recipe_prefix = '\t';
    let mut count = 0;
    for line in content.lines() {
        if is_recipe_line(line, recipe_prefix) {
            continue;
        }
        if observe_recipe_prefix(line.trim_start(), &mut recipe_prefix) {
            continue;
        }
        if matches!(
            line.split_whitespace().next(),
            Some("include" | "-include" | "sinclude")
        ) {
            count += 1;
        }
    }
    count
}

/// Extract target names (and `## description` doc comments) from Makefile
/// text. Line-oriented on purpose: recipe lines (tab- or
/// `.RECIPEPREFIX`-indented), comments, directives ([`DIRECTIVES`]),
/// variable assignments (`:=`, `::=`, `?=`, `+=`, `=`), special targets
/// (`.PHONY` etc.), and pattern rules (`%`) are skipped, as is anything
/// needing make-time expansion (`$`). Duplicate names keep the first
/// occurrence (matching how the `make help` grep convention lists them).
pub(crate) fn parse_targets(content: &str) -> Vec<MakeTarget> {
    let mut targets: Vec<MakeTarget> = Vec::new();
    let mut recipe_prefix = '\t';
    for line in content.lines() {
        if is_recipe_line(line, recipe_prefix) || line.trim_start().starts_with('#') {
            continue;
        }
        let trimmed = line.trim_start();
        if observe_recipe_prefix(trimmed, &mut recipe_prefix) {
            continue;
        }
        if DIRECTIVES.contains(&trimmed.split_whitespace().next().unwrap_or("")) {
            continue;
        }
        let Some(colon) = line.find(':') else {
            continue;
        };
        // `FOO := bar` / `FOO ::= bar` are assignments, not rules.
        let after = &line[colon + 1..];
        let after = after.strip_prefix(':').unwrap_or(after); // double-colon rule
        if after.starts_with('=') {
            continue;
        }
        let head = &line[..colon];
        // `FOO ?= a:b` style assignments put `=` before the colon.
        if head.contains('=') {
            continue;
        }
        let description = line
            .split_once("## ")
            .map(|(_, d)| d.trim().to_string())
            .filter(|d| !d.is_empty());
        for name in head.split_whitespace() {
            if name.starts_with('.') || name.contains('%') || name.contains('$') {
                continue;
            }
            if targets.iter().any(|t| t.name == name) {
                continue;
            }
            targets.push(MakeTarget {
                name: name.to_string(),
                description: description.clone(),
            });
        }
    }
    targets
}

/// Why a target was withheld from the checklist.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkipReason {
    /// Name fails `validate_command_name` (built-in collision, bad chars).
    InvalidName(String),
    /// `[commands.<name>]` already exists in `.ops.toml`.
    AlreadyConfigured,
    /// Name is reserved by ops and must never be imported.
    Reserved,
}

/// Names reserved by ops itself. `help` is not a clap-registered subcommand
/// (the `External` catch-all disables clap's implicit help), so it slips past
/// `validate_command_name` — but importing it would shadow the conventional
/// `ops help` meaning, so it is explicitly barred here.
const RESERVED_NAMES: &[&str] = &["help"];

/// A target withheld from the checklist, paired with why.
#[derive(Debug)]
pub(crate) struct SkippedTarget {
    pub target: MakeTarget,
    pub reason: SkipReason,
}

/// Split parsed targets into importable ones and skipped ones (with reason).
fn partition_importable(
    workspace_root: &Path,
    targets: Vec<MakeTarget>,
) -> anyhow::Result<(Vec<MakeTarget>, Vec<SkippedTarget>)> {
    // Missing `.ops.toml` means no commands; a malformed one is a hard
    // error so we never offer an import we would refuse to write.
    let existing = command_names(&workspace_root.join(".ops.toml"))?;
    let mut importable = Vec::new();
    let mut skipped = Vec::new();
    for target in targets {
        if RESERVED_NAMES.contains(&target.name.as_str()) {
            skipped.push(SkippedTarget {
                target,
                reason: SkipReason::Reserved,
            });
        } else if existing.iter().any(|n| n == &target.name) {
            skipped.push(SkippedTarget {
                target,
                reason: SkipReason::AlreadyConfigured,
            });
        } else if let Err(e) = crate::new_command_cmd::validate_command_name(&target.name) {
            skipped.push(SkippedTarget {
                target,
                reason: SkipReason::InvalidName(format!("{e:#}")),
            });
        } else {
            importable.push(target);
        }
    }
    Ok((importable, skipped))
}

fn write_skipped_notes<W: Write>(w: &mut W, skipped: &[SkippedTarget]) -> io::Result<()> {
    for SkippedTarget { target, reason } in skipped {
        match reason {
            SkipReason::AlreadyConfigured => writeln!(
                w,
                "Skipping '{}': already defined in .ops.toml",
                target.name
            )?,
            SkipReason::InvalidName(msg) => {
                writeln!(w, "Skipping '{}': {msg}", target.name)?;
            }
            SkipReason::Reserved => {
                writeln!(w, "Skipping '{}': reserved name", target.name)?;
            }
        }
    }
    Ok(())
}

fn write_imported_confirmation<W: Write>(w: &mut W, chosen: &[&MakeTarget]) -> io::Result<()> {
    let names: Vec<&str> = chosen.iter().map(|t| t.name.as_str()).collect();
    let run_hints: Vec<String> = names.iter().map(|n| format!("ops {n}")).collect();
    writeln!(
        w,
        "Imported {} command(s) to .ops.toml: {}. Run with: {}",
        names.len(),
        names.join(", "),
        run_hints.join(", ")
    )
}

/// Append every chosen target to `.ops.toml` in a single edit so the write
/// is all-or-nothing — a duplicate (raced in since the partition) aborts the
/// whole batch rather than leaving half the selection behind.
fn append_targets_to_config(workspace_root: &Path, chosen: &[&MakeTarget]) -> anyhow::Result<()> {
    let config_path = workspace_root.join(".ops.toml");
    edit_ops_toml(&config_path, |doc| {
        let commands = ensure_table(doc, "commands")?;
        for target in chosen {
            insert_command(
                commands,
                &target.name,
                "make",
                &[target.name.as_str()],
                target.description.as_deref(),
            )?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# Developer entrypoint. `make help` lists targets.
SHELL := /bin/bash
.PHONY: help build release test check deb clean

help: ## Show this help
\t@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST)

build: ## Compile (debug)
\tcargo build --all --all-features

release: ## Compile (release)
\tcargo build --release --bin oxydraw

test: ## Run the test suite
\tcargo test --all --all-features

check: fmt clippy test ## All gates: fmt + clippy + test

deb: ## Build the .deb package (see packaging/)
\t$(MAKE) -C packaging build

clean: ## Remove build artifacts
\tcargo clean
\t$(MAKE) -C packaging clean
";

    fn names(targets: &[MakeTarget]) -> Vec<&str> {
        targets.iter().map(|t| t.name.as_str()).collect()
    }

    #[test]
    fn parse_targets_extracts_documented_targets() {
        let targets = parse_targets(SAMPLE);
        assert_eq!(
            names(&targets),
            vec!["help", "build", "release", "test", "check", "deb", "clean"]
        );
    }

    #[test]
    fn parse_targets_captures_doc_comment() {
        let targets = parse_targets(SAMPLE);
        let build = targets.iter().find(|t| t.name == "build").unwrap();
        assert_eq!(build.description.as_deref(), Some("Compile (debug)"));
    }

    #[test]
    fn parse_targets_target_with_prerequisites_keeps_description() {
        let targets = parse_targets(SAMPLE);
        let check = targets.iter().find(|t| t.name == "check").unwrap();
        assert_eq!(
            check.description.as_deref(),
            Some("All gates: fmt + clippy + test")
        );
    }

    #[test]
    fn parse_targets_skips_assignments_special_targets_and_recipes() {
        let targets = parse_targets(SAMPLE);
        let all = names(&targets);
        assert!(!all.contains(&"SHELL"), "`SHELL := …` is an assignment");
        assert!(!all.contains(&".PHONY"), "special targets start with '.'");
        assert!(
            !all.iter()
                .any(|n| n.contains("grep") || n.contains("cargo")),
            "recipe lines must not be parsed as targets: {all:?}"
        );
    }

    #[test]
    fn parse_targets_skips_pattern_rules_and_expansions() {
        let targets = parse_targets("%.o: %.c\n\tcc -c $<\n$(BIN): main.o\n\tcc -o $@ main.o\n");
        assert!(targets.is_empty(), "got {targets:?}");
    }

    #[test]
    fn parse_targets_handles_double_colon_rules_and_undocumented_targets() {
        // `build` after `::` is a prerequisite, not a target.
        let targets = parse_targets("all:: build\nfmt:\n\tcargo fmt\n");
        assert_eq!(names(&targets), vec!["all", "fmt"]);
        assert!(targets.iter().all(|t| t.description.is_none()));
    }

    #[test]
    fn parse_targets_skips_conditional_assignment_with_colon_in_value() {
        let targets = parse_targets("PATH_EXTRA ?= /opt/bin:/usr/local/bin\nbuild:\n\tmake\n");
        assert_eq!(names(&targets), vec!["build"]);
    }

    #[test]
    fn parse_targets_skips_directive_lines() {
        // `export FOO: bar` is a target-specific variable assignment;
        // neither `export` nor `FOO` is a target.
        let targets = parse_targets(
            "export FOO: bar\noverride CFLAGS: -O2\nifeq (a:b, a:b)\nendif\nbuild:\n\tmake\n",
        );
        assert_eq!(names(&targets), vec!["build"]);
    }

    #[test]
    fn parse_targets_honours_recipeprefix() {
        // GNU make 3.82+: with `.RECIPEPREFIX = >` recipe lines start with
        // `>` instead of tab; a colon in the recipe must not become a target.
        let targets = parse_targets(".RECIPEPREFIX = >\ndeploy:\n>echo deploying to host:port\n");
        assert_eq!(names(&targets), vec!["deploy"]);
    }

    #[test]
    fn parse_targets_recipeprefix_empty_resets_to_tab() {
        let targets = parse_targets(".RECIPEPREFIX = >\na:\n>x:y\n.RECIPEPREFIX =\nb:\n\tz:w\n");
        assert_eq!(names(&targets), vec!["a", "b"]);
    }

    #[test]
    fn count_include_directives_counts_all_variants_outside_recipes() {
        let content = "include a.mk\n-include b.mk\nsinclude c.mk\nbuild:\n\tinclude d.mk\n";
        assert_eq!(count_include_directives(content), 3);
        assert_eq!(count_include_directives("build:\n\tmake\n"), 0);
    }

    #[test]
    fn count_include_directives_honours_recipeprefix() {
        // PATTERN-1 / TASK-1653: under `.RECIPEPREFIX = >`, `>include …` is
        // a recipe line, not an include directive.
        assert_eq!(
            count_include_directives(".RECIPEPREFIX = >\ndeploy:\n>include extra.conf\n"),
            0
        );
        // Real directives outside recipes still count; the empty
        // reassignment switches back to tab.
        assert_eq!(
            count_include_directives(
                ".RECIPEPREFIX = >\ninclude a.mk\n>include b.mk\n.RECIPEPREFIX =\ninclude c.mk\n"
            ),
            2
        );
    }

    #[test]
    fn parse_targets_dedups_keeping_first_description() {
        let targets = parse_targets("build: ## first\n\techo a\nbuild: dep ## second\n");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].description.as_deref(), Some("first"));
    }

    #[test]
    fn append_targets_writes_make_commands_with_help() {
        let dir = tempfile::tempdir().expect("tempdir");
        let build = MakeTarget {
            name: "build".into(),
            description: Some("Compile (debug)".into()),
        };
        let fmt = MakeTarget {
            name: "fmt".into(),
            description: None,
        };

        append_targets_to_config(dir.path(), &[&build, &fmt]).expect("append");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("[commands.build]"));
        assert!(content.contains(r#"program = "make""#));
        assert!(content.contains(r#"args = ["build"]"#));
        assert!(content.contains(r#"help = "Compile (debug)""#));
        assert!(content.contains("[commands.fmt]"));
        assert!(content.contains(r#"args = ["fmt"]"#));
    }

    #[test]
    fn append_targets_is_all_or_nothing_on_duplicate() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(".ops.toml"),
            "[commands.test]\nprogram = \"cargo\"\nargs = [\"test\"]\n",
        )
        .unwrap();
        let before = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();

        let build = MakeTarget {
            name: "build".into(),
            description: None,
        };
        let test = MakeTarget {
            name: "test".into(),
            description: None,
        };
        let result = append_targets_to_config(dir.path(), &[&build, &test]);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
        assert_eq!(
            std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap(),
            before,
            "a failed batch must leave .ops.toml untouched"
        );
    }

    #[test]
    fn partition_skips_existing_and_builtin_colliding_names() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(".ops.toml"),
            "[commands.test]\nprogram = \"cargo\"\nargs = [\"test\"]\n",
        )
        .unwrap();
        let targets = vec![
            MakeTarget {
                name: "build".into(),
                description: None,
            },
            MakeTarget {
                name: "test".into(),
                description: None,
            },
            // `init` collides with the built-in `ops init` subcommand.
            MakeTarget {
                name: "init".into(),
                description: None,
            },
            // `help` is not a clap built-in (the `External` catch-all
            // disables clap's implicit help subcommand) but is reserved by
            // ops, so it must never be importable.
            MakeTarget {
                name: "help".into(),
                description: None,
            },
        ];

        let (importable, skipped) = partition_importable(dir.path(), targets).expect("partition");

        assert_eq!(names(&importable), vec!["build"]);
        assert_eq!(skipped.len(), 3);
        assert!(skipped
            .iter()
            .any(|s| s.target.name == "test" && s.reason == SkipReason::AlreadyConfigured));
        assert!(skipped
            .iter()
            .any(|s| s.target.name == "init" && matches!(s.reason, SkipReason::InvalidName(_))));
        assert!(skipped
            .iter()
            .any(|s| s.target.name == "help" && s.reason == SkipReason::Reserved));
    }

    #[test]
    fn partition_errors_on_malformed_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".ops.toml"), "not = = valid\n{{{").unwrap();
        let targets = vec![MakeTarget {
            name: "build".into(),
            description: None,
        }];
        assert!(partition_importable(dir.path(), targets).is_err());
    }

    #[test]
    fn resolve_makefile_prefers_make_lookup_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("Makefile"), "a:\n").unwrap();
        std::fs::write(dir.path().join("GNUmakefile"), "a:\n").unwrap();
        let path = resolve_makefile_path(dir.path(), None).expect("resolve");
        assert!(path.ends_with("GNUmakefile"));
    }

    #[test]
    fn resolve_makefile_explicit_file_relative_to_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("packaging")).unwrap();
        std::fs::write(dir.path().join("packaging/Makefile"), "deb:\n").unwrap();
        let path = resolve_makefile_path(dir.path(), Some(PathBuf::from("packaging/Makefile")))
            .expect("resolve");
        assert!(path.ends_with("packaging/Makefile"));
    }

    #[test]
    fn resolve_makefile_missing_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(resolve_makefile_path(dir.path(), None).is_err());
        assert!(resolve_makefile_path(dir.path(), Some(PathBuf::from("nope.mk"))).is_err());
    }

    #[test]
    fn import_makefile_non_tty_returns_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = run_import_makefile_with_tty_check(dir.path(), None, || false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("interactive terminal"));
    }

    #[test]
    fn write_skipped_notes_renders_both_reasons() {
        let mut buf: Vec<u8> = Vec::new();
        let skipped = vec![
            SkippedTarget {
                target: MakeTarget {
                    name: "test".into(),
                    description: None,
                },
                reason: SkipReason::AlreadyConfigured,
            },
            SkippedTarget {
                target: MakeTarget {
                    name: "init".into(),
                    description: None,
                },
                reason: SkipReason::InvalidName("collides with a built-in".into()),
            },
            SkippedTarget {
                target: MakeTarget {
                    name: "help".into(),
                    description: None,
                },
                reason: SkipReason::Reserved,
            },
        ];
        write_skipped_notes(&mut buf, &skipped).expect("write");
        let out = String::from_utf8(buf).expect("utf8");
        assert!(out.contains("Skipping 'test': already defined"));
        assert!(out.contains("Skipping 'init': collides with a built-in"));
        assert!(out.contains("Skipping 'help': reserved name"));
    }

    #[test]
    fn write_imported_confirmation_lists_names_and_run_hints() {
        let mut buf: Vec<u8> = Vec::new();
        let build = MakeTarget {
            name: "build".into(),
            description: None,
        };
        let release = MakeTarget {
            name: "release".into(),
            description: None,
        };
        write_imported_confirmation(&mut buf, &[&build, &release]).expect("write");
        let out = String::from_utf8(buf).expect("utf8");
        assert!(out.contains("Imported 2 command(s)"));
        assert!(out.contains("ops build"));
        assert!(out.contains("ops release"));
        assert!(out.ends_with('\n'));
    }
}
