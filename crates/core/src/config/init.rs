//! `ops init` scaffolding: the embedded default `.ops.toml` and the
//! section-filtered template rendered from it.

use super::root::Config;
use anyhow::Context;
use std::path::Path;

/// Default config content from `src/.default.ops.toml` (embedded at build; used as base config and for `cargo ops init`).
/// Build fails if the file is missing.
#[must_use]
pub const fn default_ops_toml() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/.default.ops.toml"
    ))
}

/// Controls which sections are included in `ops init` output.
#[derive(Debug, Clone)]
pub struct InitSections {
    pub output: bool,
    pub themes: bool,
    pub commands: bool,
}

impl InitSections {
    /// Build from CLI flags. When no flags are given, default to output-only.
    #[must_use]
    pub const fn from_flags(output: bool, themes: bool, commands: bool) -> Self {
        if !output && !themes && !commands {
            Self {
                output: true,
                themes: false,
                commands: false,
            }
        } else {
            Self {
                output,
                themes,
                commands,
            }
        }
    }
}

/// Build init template with only the requested sections.
///
/// # Errors
///
/// If the embedded default config fails to parse, or the assembled template
/// fails to serialize back to TOML.
pub fn init_template(workspace_root: &Path, sections: &InitSections) -> anyhow::Result<String> {
    let full: Config =
        toml::from_str(default_ops_toml()).context("failed to parse internal default config")?;

    let mut config = Config::empty();

    if sections.output {
        config.output = full.output;
    }

    if sections.themes {
        config.themes = full.themes;
    }

    if sections.commands {
        if let Some(stack) = crate::stack::Stack::detect(workspace_root) {
            for (id, spec) in stack.default_commands() {
                config.commands.insert(id, spec);
            }
            config.stack = Some(stack.as_str().to_string());
        }
    }

    toml::to_string_pretty(&config).context("failed to serialize init config")
}
