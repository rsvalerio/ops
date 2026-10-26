//! Renders the component specimens of the theme design systems in
//! `docs/design-system/<theme>/` through the real [`ConfigurableTheme`].
//!
//! Every specimen line is produced by the renderer itself (`render`,
//! `render_plan_header`, `render_error_detail`, `render_report`, the boxed
//! borders and `wrap_step_line`); running rows are drawn by indicatif from the
//! theme's own `running_template`, the way the runner draws them. The only
//! thing this example adds is colour: the renderer emits SGR codes only when
//! stderr is a terminal, so the specimens are rendered plain and each part is
//! tagged with the CSS classes of the SGR codes its colour spec compiles to
//! (`precompute_sgr_prefix`, the renderer's own parser).
//!
//! Themes come from the embedded `.default.ops.toml`, so a theme change
//! shows up in the specimens on the next run:
//!
//! ```text
//! cargo run -p ops-theme --example design_specimens [-- <out-dir>]
//! ```
//!
//! `<out-dir>` defaults to `docs/design-system`. For each theme it writes
//! `<theme>/project/components/<Component>/preview.html` and copies the
//! shared `bundle.css` to `<theme>/project/components/bundle.css`. It fails,
//! writing nothing for that theme, if a line exceeds the column budget, if a
//! running row's layout disagrees with its template, or if the theme uses a
//! colour its `tokens.json` does not define.

use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressState, ProgressStyle, TermLike};
use ops_core::config::theme_types::LayoutKind;
use ops_core::config::{default_ops_toml, Config};
use ops_core::output::{ErrorDetail, StepLine, StepStatus};
use ops_core::report::{Report, ReportRow, ReportStatus};
use ops_theme::style::precompute_sgr_prefix;
use ops_theme::{
    format_duration, format_running_elapsed, strip_ansi, BoxSnapshot, ConfigurableTheme,
    PlanHeaderStyle, ThemeConfig,
};
use unicode_width::UnicodeWidthChar;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// The column budget every specimen is rendered at.
const COLUMNS: u16 = 72;
const COMMAND_IDS: [&str; 3] = ["build", "lint", "test"];
/// A plan too long for one header line, to show the plan header wrapping.
const LONG_COMMAND_IDS: [&str; 12] = [
    "fmt-check",
    "trailing-whitespace-check",
    "end-of-file-fixer-check",
    "clippy",
    "build",
    "check-json",
    "check-yaml",
    "doc",
    "doc-default [crate=core]",
    "doc-default [crate=cli]",
    "deps",
    "test",
];

fn main() -> Result<()> {
    let out_dir = std::env::args_os()
        .nth(1)
        .map_or_else(default_out_dir, PathBuf::from);
    for (path, contents) in render_all(&out_dir)? {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&path, contents)?;
    }
    println!("wrote the specimens under {}", out_dir.display());
    Ok(())
}

/// `docs/design-system` in this checkout.
fn default_out_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/design-system")
}

/// Every file the generator writes under `out_dir`, with its contents:
/// each theme's previews and its copy of the shared `bundle.css`. Writes
/// nothing; fails on the first theme that does not pass its checks.
fn render_all(out_dir: &Path) -> Result<Vec<(PathBuf, String)>> {
    let bundle_css = fs::read_to_string(out_dir.join("bundle.css"))
        .map_err(|e| format!("reading {}: {e}", out_dir.join("bundle.css").display()))?;
    let config: Config = toml::from_str(default_ops_toml())?;
    let mut files = Vec::new();
    for (name, theme_config) in config.themes {
        let project = out_dir.join(&name).join("project");
        let tokens = fs::read_to_string(project.join("tokens.json"))
            .map_err(|e| format!("theme '{name}': reading tokens.json: {e}"))?;
        let cards = Specimens::new(&name, theme_config).cards()?;
        check_tokens(&name, &cards, &tokens, &bundle_css)?;
        let components = project.join("components");
        for card in &cards {
            files.push((
                components.join(card.component).join("preview.html"),
                card.html(),
            ));
        }
        files.push((components.join("bundle.css"), bundle_css.clone()));
    }
    Ok(files)
}

// ---------------------------------------------------------------- lines

/// One terminal cell of a specimen line: its glyph and the CSS classes of the
/// SGR codes in effect, plus whether it is the animated spinner cell.
#[derive(Clone, Debug, Default)]
struct Cell {
    ch: char,
    classes: String,
    spin: bool,
}

/// A rendered line, held as cells so parts can be coloured by position.
#[derive(Clone, Debug, Default)]
struct Line(Vec<Cell>);

impl Line {
    fn plain(text: &str) -> Self {
        Self(
            strip_ansi(text)
                .chars()
                .map(|ch| Cell {
                    ch,
                    ..Cell::default()
                })
                .collect(),
        )
    }

    const fn len(&self) -> usize {
        self.0.len()
    }

    fn columns(&self) -> usize {
        self.0
            .iter()
            .map(|c| c.ch.width().unwrap_or(0))
            .fold(0, usize::saturating_add)
    }

    fn text(&self) -> String {
        self.0.iter().map(|c| c.ch).collect()
    }

    /// Char index of `needle` at or after `from`.
    fn find(&self, needle: &str, from: usize) -> Option<usize> {
        let needle: Vec<char> = needle.chars().collect();
        if needle.is_empty() {
            return None;
        }
        let hay: Vec<char> = self.0.iter().map(|c| c.ch).collect();
        hay.windows(needle.len())
            .enumerate()
            .skip(from)
            .find(|(_, w)| *w == needle.as_slice())
            .map(|(i, _)| i)
    }

    /// Set the classes of `len` cells starting at `start` (a later paint wins,
    /// as a later SGR sequence does).
    fn paint(&mut self, start: usize, len: usize, classes: &str) {
        for cell in self.0.iter_mut().skip(start).take(len) {
            cell.classes = classes.to_string();
        }
    }

    fn html(&self) -> String {
        let mut out = String::new();
        let mut cells = self.0.iter().peekable();
        while let Some(first) = cells.next() {
            let mut run = vec![first];
            while let Some(next) = cells.peek() {
                if next.classes == first.classes && !next.spin && !first.spin {
                    run.extend(cells.next());
                } else {
                    break;
                }
            }
            let body: String = run.iter().map(|c| cell_html(c.ch)).collect();
            if first.spin {
                let _ = write!(
                    out,
                    r#"<span class="{}" data-spin data-cell>{}</span>"#,
                    first.classes,
                    escape(&first.ch.to_string())
                );
            } else if first.classes.is_empty() {
                out.push_str(&body);
            } else {
                let _ = write!(out, r#"<span class="{}">{body}</span>"#, first.classes);
            }
        }
        out
    }
}

/// A glyph as HTML. Every non-ASCII glyph is pinned to its terminal width
/// (`ops-c` one cell, `ops-c2` two): browsers draw box-drawing, braille and
/// emoji from fallback faces whose advances would otherwise drift a line off
/// the column grid.
fn cell_html(ch: char) -> String {
    if ch.is_ascii() {
        return escape(&ch.to_string());
    }
    let class = if ch.width() == Some(2) {
        "ops-c2"
    } else {
        "ops-c"
    };
    format!(r#"<span class="{class}">{ch}</span>"#)
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// CSS classes for a colour spec: one `sgr-<code>` per SGR code the theme's
/// own parser produces (`"bold green"` → `"sgr-1 sgr-32"`).
fn classes(spec: &str) -> String {
    precompute_sgr_prefix(spec)
        .map(|prefix| {
            prefix
                .trim_start_matches("\u{1b}[")
                .trim_end_matches('m')
                .split(';')
                .map(|code| format!("sgr-{code}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn chars(s: &str) -> usize {
    s.chars().count()
}

// ------------------------------------------------------------ specimens

/// Renders the specimens of one theme.
struct Specimens {
    name: String,
    config: ThemeConfig,
    theme: ConfigurableTheme,
}

/// One component's preview document.
struct Card {
    component: &'static str,
    group: &'static str,
    height: u16,
    subtitle: Option<&'static str>,
    terms: Vec<(String, Vec<Line>)>,
    caption: Option<String>,
    spinner_frames: Option<String>,
}

impl Specimens {
    fn new(name: &str, config: ThemeConfig) -> Self {
        Self {
            name: name.to_string(),
            theme: ConfigurableTheme::new(config.clone()),
            config,
        }
    }

    const fn boxed(&self) -> bool {
        matches!(self.config.layout_kind, LayoutKind::Boxed)
    }

    fn cards(&self) -> Result<Vec<Card>> {
        let mut cards = Vec::new();
        if self.boxed() {
            cards.push(self.box_frame());
        }
        cards.push(self.plan_header());
        if self.boxed() {
            cards.push(self.progress_column()?);
        }
        cards.push(self.step_line()?);
        cards.push(self.spinner()?);
        cards.push(self.error_block());
        cards.push(self.summary());
        cards.push(self.report());
        cards.push(self.run_transcript());
        for card in &cards {
            for (_, lines) in &card.terms {
                for line in lines {
                    if line.columns() > usize::from(COLUMNS) {
                        return Err(format!(
                            "theme '{}' {}: line is {} columns, over the {COLUMNS}-column \
                             budget: {:?}",
                            self.name,
                            card.component,
                            line.columns(),
                            line.text()
                        )
                        .into());
                    }
                }
            }
        }
        Ok(cards)
    }

    // ----- line builders

    /// Where the label and the separator run of a `render_slot` line sit, as
    /// `(label_start, sep_start, sep_end, trailing_start)` char indices.
    ///
    /// `render_slot` composes `{pad}{indent}{icon}{pad} {label}{sep}` then
    /// ` {trailing}` when there is one; the separator run is ` ` + glyphs,
    /// plus a closing ` ` only when the trailing slot is empty. Running rows
    /// carry neither the left pad nor the indent.
    fn slot_layout(
        &self,
        line: &Line,
        icon: &str,
        trailing: &str,
        running: bool,
    ) -> (usize, usize, usize, usize) {
        let parts = self.theme.icon_prefix_parts(icon, running);
        let pad = if running {
            0
        } else {
            chars(self.theme.left_pad_str())
        };
        let label_start = pad
            .saturating_add(chars(parts.indent))
            .saturating_add(chars(parts.icon))
            .saturating_add(chars(&parts.pad))
            .saturating_add(1);
        let text: Vec<char> = line.0.iter().map(|c| c.ch).collect();
        let content_end = text
            .iter()
            .rposition(|c| *c != ' ')
            .map_or(0, |i| i.saturating_add(1));
        let (glyphs_end, sep_end, trailing_start) = if trailing.is_empty() {
            (
                content_end,
                content_end.saturating_add(1),
                content_end.saturating_add(1),
            )
        } else {
            let trailing_start = content_end.saturating_sub(chars(trailing));
            let glyphs_end = trailing_start.saturating_sub(1);
            (glyphs_end, glyphs_end, trailing_start)
        };
        let sep_char = self.config.separator_char;
        let mut sep_start = glyphs_end;
        while sep_start > label_start && text.get(sep_start.saturating_sub(1)) == Some(&sep_char) {
            sep_start = sep_start.saturating_sub(1);
        }
        // The run's leading space.
        let sep_start = sep_start.saturating_sub(1).max(label_start);
        (label_start, sep_start, sep_end, trailing_start)
    }

    /// Colour a completed or pending row's parts the way `render_slot` does:
    /// label, separator run (with its spaces), trailing slot.
    fn paint_slot(&self, line: &mut Line, icon: &str, trailing: &str, trailing_classes: &str) {
        let (label_start, sep_start, sep_end, trailing_start) =
            self.slot_layout(line, icon, trailing, false);
        line.paint(
            label_start,
            sep_start.saturating_sub(label_start),
            &classes(&self.config.label_color),
        );
        line.paint(
            sep_start,
            sep_end.saturating_sub(sep_start),
            &classes(&self.config.separator_color),
        );
        if !trailing.is_empty() {
            line.paint(trailing_start, chars(trailing), trailing_classes);
        }
    }

    /// A completed or pending step row, framed when the layout is boxed.
    fn step(&self, status: StepStatus, label: &str, elapsed: Option<f64>) -> Line {
        let budget = COLUMNS.saturating_sub(self.theme.step_column_reserve());
        let step = StepLine::new(status, label, elapsed);
        let inner = strip_ansi(&self.theme.render(&step, budget));
        let trailing = elapsed
            .map(|s| self.theme.format_elapsed(s))
            .unwrap_or_default();
        let icon = self.theme.status_icon(status).to_string();
        let duration = classes(&self.config.duration_color);
        if self.boxed() {
            let cell = if status == StepStatus::Pending {
                "░"
            } else {
                "█"
            };
            let mut line = Line::plain(&self.theme.wrap_step_line(&inner, cell, COLUMNS));
            let offset = chars(self.theme.left_pad_str()).saturating_add(chars("│ ░  "));
            let mut inner_line = Line::plain(&inner);
            self.paint_slot(&mut inner_line, &icon, &trailing, &duration);
            for (i, cell) in inner_line.0.into_iter().enumerate() {
                if let Some(target) = line.0.get_mut(i.saturating_add(offset)) {
                    *target = cell;
                }
            }
            line
        } else {
            let mut line = Line::plain(&inner);
            self.paint_slot(&mut line, &icon, &trailing, &duration);
            line
        }
    }

    /// A running row, drawn by indicatif from the theme's `running_template`
    /// exactly as the runner builds its running style.
    fn running(&self, label: &str, elapsed: Duration) -> Result<Line> {
        let template = self.theme.running_template();
        let padded = format!("{}{template}", self.theme.left_pad_str());
        let style = ProgressStyle::with_template(&padded)?
            .tick_chars(self.theme.tick_chars())
            .with_key(
                "elapsed",
                |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                    let _ = write!(
                        w,
                        "{}",
                        format_running_elapsed(state.elapsed().as_secs_f64())
                    );
                },
            );
        let term = RecordingTerm::default();
        let bar = ProgressBar::with_draw_target(
            None,
            ProgressDrawTarget::term_like(Box::new(term.clone())),
        )
        .with_elapsed(elapsed);
        bar.set_style(style);
        let msg = self
            .theme
            .render(&StepLine::new(StepStatus::Running, label, None), COLUMNS);
        bar.set_message(msg.clone());
        bar.tick();
        let drawn = term.last_line().ok_or("indicatif drew no running row")?;
        let mut line = Line::plain(&drawn);

        // Walk the template to place each placeholder, then colour it.
        let mut pos = chars(self.theme.left_pad_str());
        let msg = strip_ansi(&msg);
        for token in parse_template(template) {
            match token {
                Token::Literal(text) => pos = pos.saturating_add(chars(&text)),
                Token::Placeholder { name, width, style } => {
                    let len = match name.as_str() {
                        "spinner" => 1,
                        "msg" => chars(&msg),
                        "elapsed" => {
                            chars(&format_running_elapsed(elapsed.as_secs_f64())).max(width)
                        }
                        other => return Err(format!("unexpected placeholder {{{other}}}").into()),
                    };
                    line.paint(pos, len, &classes(&style));
                    if name == "spinner" {
                        // Start on the first frame whatever tick indicatif
                        // drew, so the output is the same on every run.
                        let first = self.config.tick_chars.chars().next();
                        if let Some(cell) = line.0.get_mut(pos) {
                            cell.spin = true;
                            cell.ch = first.unwrap_or(cell.ch);
                        }
                    }
                    if name == "msg" {
                        self.paint_running_msg(&mut line, pos, &msg, &classes(&style));
                    }
                    pos = pos.saturating_add(len);
                }
            }
        }
        if pos != line.len() {
            return Err(format!(
                "theme '{}': running_template places {pos} cells but indicatif drew {}: {:?}",
                self.name,
                line.len(),
                line.text()
            )
            .into());
        }
        Ok(line)
    }

    /// Colour the theme's message inside a running row. The template's msg
    /// style opens first; the label and separator then emit their own SGR
    /// sequence, each ending in a reset, so a coloured part wins and
    /// whatever follows it falls back to the default until the next part
    /// sets a colour again.
    fn paint_running_msg(&self, line: &mut Line, at: usize, msg: &str, msg_classes: &str) {
        let msg_line = Line::plain(msg);
        let (label_start, sep_start, sep_end, _) = self.slot_layout(&msg_line, "", "", true);
        let mut current = msg_classes.to_string();
        let parts = [
            (0, label_start, String::new()),
            (label_start, sep_start, classes(&self.config.label_color)),
            (sep_start, sep_end, classes(&self.config.separator_color)),
        ];
        for (start, end, own) in parts {
            let paint = if own.is_empty() {
                current.clone()
            } else {
                own.clone()
            };
            line.paint(at.saturating_add(start), end.saturating_sub(start), &paint);
            if !own.is_empty() {
                current = String::new();
            }
        }
    }

    fn header(&self) -> Vec<Line> {
        self.header_for(&COMMAND_IDS)
    }

    fn header_for(&self, command_ids: &[&str]) -> Vec<Line> {
        let ids: Vec<String> = command_ids.iter().map(ToString::to_string).collect();
        let header = classes(&self.config.header_color);
        if self.boxed() {
            let mut line = Line::plain(
                &self
                    .theme
                    .box_top_border(snapshot(&ids, Outcome::Started))
                    .unwrap_or_default(),
            );
            let title = format!(
                " {}Running: {} ",
                self.theme.plan_header_prefix(),
                ids.join(", ")
            );
            let start = chars(self.theme.left_pad_str()).saturating_add(chars("╭─"));
            line.paint(start, chars(&title), &header);
            return vec![line];
        }
        let rendered = self.theme.render_plan_header(&ids, COLUMNS);
        // Every line between the leading blank and the closing line carries
        // the header colour: the first from the left pad, a continuation from
        // its first id (past the left pad, the `Tree` rail and the indent).
        let last = rendered.len().saturating_sub(1);
        rendered
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let mut line = Line::plain(text);
                if (1..last).contains(&i) {
                    let pad = chars(self.theme.left_pad_str());
                    let start = if i == 1 {
                        pad
                    } else {
                        line.0
                            .iter()
                            .skip(pad)
                            .position(|c| c.ch != ' ' && c.ch != '│')
                            .map_or(line.len(), |p| p.saturating_add(pad))
                    };
                    line.paint(start, line.len().saturating_sub(start), &header);
                }
                line
            })
            .collect()
    }

    /// The run summary: the bottom border when boxed, else the summary line.
    fn summary_line(&self, success: bool) -> Line {
        let summary = classes(&self.config.summary_color);
        if self.boxed() {
            // The success specimen is a two-step plan, like the flat `Done 2/2`.
            let steps = if success { 2 } else { COMMAND_IDS.len() };
            let ids: Vec<String> = COMMAND_IDS
                .iter()
                .take(steps)
                .map(ToString::to_string)
                .collect();
            let snap = if success {
                snapshot(&ids, Outcome::Passed)
            } else {
                snapshot(&ids, Outcome::Failed)
            };
            let mut line = Line::plain(&self.theme.box_bottom_border(snap).unwrap_or_default());
            let start = chars(self.theme.left_pad_str()).saturating_add(chars("╰─"));
            let end = line
                .find(" ─", start)
                .map_or(line.len(), |i| i.saturating_add(1));
            line.paint(start, end.saturating_sub(start), &summary);
            return line;
        }
        let body = if success {
            format!("Done 2/2 in {}", format_duration(0.54))
        } else {
            format!(
                "2 succeeded, 0 skipped, 1 failed of 3 in {}",
                format_duration(0.55)
            )
        };
        let mut line = Line::plain(&self.theme.render_summary_text(&body));
        line.paint(
            line.len().saturating_sub(chars(&body)),
            chars(&body),
            &summary,
        );
        line
    }

    fn error_lines(&self) -> Vec<Line> {
        let detail = ErrorDetail::new(
            "exit status: 1",
            vec![
                "assertion failed: left == right".to_string(),
                "  left: 3, right: 4".to_string(),
            ],
        );
        let chars_cfg = &self.config.error_block;
        let glyphs = classes(&chars_cfg.color);
        let from = chars(self.theme.left_pad_str()).saturating_add(chars(&chars_cfg.rail));
        self.theme
            .render_error_detail(&detail, COLUMNS)
            .iter()
            .map(|text| {
                let mut line = Line::plain(text);
                // The bracket glyph is the first of top/mid/bottom after the
                // rail; a boxed frame's own bar is the rail.
                let glyph = [&chars_cfg.top, &chars_cfg.mid, &chars_cfg.bottom]
                    .into_iter()
                    .filter(|g| !g.is_empty())
                    .filter_map(|g| line.find(g, from).map(|at| (at, chars(g))))
                    .min();
                if let Some((at, len)) = glyph {
                    line.paint(at, len, &glyphs);
                }
                line
            })
            .collect()
    }

    // ----- cards

    fn box_frame(&self) -> Card {
        let ids: Vec<String> = COMMAND_IDS
            .iter()
            .take(2)
            .map(ToString::to_string)
            .collect();
        let mut top = Line::plain(
            &self
                .theme
                .box_top_border(snapshot(&ids, Outcome::Started))
                .unwrap_or_default(),
        );
        let title = format!(
            " {}Running: {} ",
            self.theme.plan_header_prefix(),
            ids.join(", ")
        );
        top.paint(
            chars("╭─"),
            chars(&title),
            &classes(&self.config.header_color),
        );
        let mut bottom = Line::plain(
            &self
                .theme
                .box_bottom_border(snapshot(&ids, Outcome::Started))
                .unwrap_or_default(),
        );
        let end = bottom
            .find(" ─", 2)
            .map_or(bottom.len(), |i| i.saturating_add(1));
        bottom.paint(
            chars("╰─"),
            end.saturating_sub(chars("╰─")),
            &classes(&self.config.summary_color),
        );
        Card {
            component: "BoxFrame",
            group: "Run output",
            height: 220,
            subtitle: None,
            terms: vec![(
                "frame".into(),
                vec![
                    top,
                    self.step(StepStatus::Pending, "cargo build --all-targets", None),
                    self.step(StepStatus::Pending, "cargo test --workspace", None),
                    bottom,
                ],
            )],
            caption: Some(format!(
                "layout_kind = \"boxed\" · left_pad = {} · ╭─ title ─╮ · │ … │ · ╰─ summary ─╯",
                self.config.left_pad
            )),
            spinner_frames: None,
        }
    }

    fn plan_header(&self) -> Card {
        let style = match self.config.plan_header_style {
            PlanHeaderStyle::Plain => "plain",
            PlanHeaderStyle::Tree => "tree",
        };
        Card {
            component: "PlanHeader",
            group: "Run output",
            height: if self.boxed() { 120 } else { 260 },
            subtitle: None,
            terms: if self.boxed() {
                vec![("top border".into(), self.header())]
            } else {
                vec![
                    ("ops verify".into(), self.header()),
                    ("a long plan".into(), self.header_for(&LONG_COMMAND_IDS)),
                ]
            },
            caption: Some(format!(
                "plan_header_style = {style:?} · plan_header_prefix = {:?} · header_color = {:?}",
                self.config.plan_header_prefix, self.config.header_color
            )),
            spinner_frames: None,
        }
    }

    fn progress_column(&self) -> Result<Card> {
        Ok(Card {
            component: "ProgressColumn",
            group: "Run output",
            height: 200,
            subtitle: None,
            terms: vec![(
                "░ pending · ▓ running · █ done".into(),
                vec![
                    self.step(StepStatus::Pending, "cargo doc --no-deps", None),
                    self.running(
                        "cargo clippy --all-targets -- -D warnings",
                        Duration::from_millis(3_120),
                    )?,
                    self.step(
                        StepStatus::Succeeded,
                        "cargo build --all-targets",
                        Some(0.35),
                    ),
                ],
            )],
            caption: Some("the left column fills as the plan advances".into()),
            spinner_frames: Some(self.frames()),
        })
    }

    fn step_line(&self) -> Result<Card> {
        Ok(Card {
            component: "StepLine",
            group: "Run output",
            height: 250,
            subtitle: None,
            terms: vec![(
                format!("{COLUMNS} columns"),
                vec![
                    self.step(StepStatus::Pending, "cargo build --all-targets", None),
                    self.running(
                        "cargo clippy --all-targets -- -D warnings",
                        Duration::from_millis(3_120),
                    )?,
                    self.step(
                        StepStatus::Succeeded,
                        "cargo build --all-targets",
                        Some(0.35),
                    ),
                    self.step(StepStatus::Failed, "cargo test --workspace", Some(4.12)),
                    self.step(StepStatus::Skipped, "cargo doc --no-deps", None),
                ],
            )],
            caption: Some("pending · running · succeeded · failed · skipped".into()),
            spinner_frames: Some(self.frames()),
        })
    }

    fn spinner(&self) -> Result<Card> {
        Ok(Card {
            component: "Spinner",
            group: "Run output",
            height: 180,
            subtitle: None,
            terms: vec![(
                "running rows".into(),
                vec![
                    self.running("cargo build --all-targets", Duration::from_millis(3_120))?,
                    self.running(
                        "cargo clippy --all-targets -- -D warnings",
                        Duration::from_secs(64),
                    )?,
                ],
            )],
            caption: Some(format!(
                "tick_chars = {:?} · 80ms · running_template = {:?}",
                self.config.tick_chars, self.config.running_template
            )),
            spinner_frames: Some(self.frames()),
        })
    }

    fn error_block(&self) -> Card {
        let mut lines = vec![self.step(StepStatus::Failed, "cargo test --workspace", Some(4.12))];
        lines.extend(self.error_lines());
        let eb = &self.config.error_block;
        Card {
            component: "ErrorBlock",
            group: "Run output",
            height: 250,
            subtitle: None,
            terms: vec![("failure detail".into(), lines)],
            caption: Some(format!(
                "top = {:?} · mid = {:?} · bottom = {:?} · rail = {:?} · color = {:?}",
                eb.top, eb.mid, eb.bottom, eb.rail, eb.color
            )),
            spinner_frames: None,
        }
    }

    fn summary(&self) -> Card {
        Card {
            component: "Summary",
            group: "Run output",
            height: 200,
            subtitle: None,
            terms: vec![
                ("success".into(), vec![self.summary_line(true)]),
                ("failure".into(), vec![self.summary_line(false)]),
            ],
            caption: Some(format!(
                "summary_prefix = {:?} · summary_color = {:?} (applied whatever the outcome)",
                self.config.summary_prefix, self.config.summary_color
            )),
            spinner_frames: None,
        }
    }

    fn report(&self) -> Card {
        let rows = [
            (ReportStatus::Ok, "Advisories", "None"),
            (ReportStatus::Ok, "License Issues", "None"),
            (ReportStatus::Warning, "Duplicate Crates", "1 warning"),
            (ReportStatus::Error, "Source Issues", "1 error"),
            (ReportStatus::Info, "Unused Dependencies", "Skipped"),
        ];
        let mut report = Report::new("Dependency Check");
        for (status, label, result) in rows {
            report.push(ReportRow::new(status, label, result));
        }
        let rt = &self.config.report;
        let title = classes(&rt.title_color);
        let summary = classes(&self.config.summary_color);
        let rendered = self.theme.render_report(&report, COLUMNS);
        let mut row_iter = rows.iter();
        let last = rendered.len().saturating_sub(1);
        let lines = rendered
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let mut line = Line::plain(text);
                let is_title = i == 1;
                let is_footer = i == last;
                if is_title || is_footer {
                    let (start, class) = if self.boxed() {
                        (
                            chars(self.theme.left_pad_str()).saturating_add(2),
                            if is_title { &title } else { &summary },
                        )
                    } else if is_title {
                        (chars(self.theme.left_pad_str()), &title)
                    } else {
                        (
                            chars(self.theme.left_pad_str())
                                .saturating_add(chars(self.theme.summary_prefix())),
                            &summary,
                        )
                    };
                    let end = if self.boxed() {
                        line.find(" ─", start)
                            .map_or(line.len(), |e| e.saturating_add(1))
                    } else {
                        line.len()
                    };
                    line.paint(start, end.saturating_sub(start), class);
                } else if !line.text().trim().is_empty() {
                    if let Some((status, _, result)) = row_iter.next() {
                        let colour = classes(match status {
                            ReportStatus::Ok => &rt.color_ok,
                            ReportStatus::Warning => &rt.color_warning,
                            ReportStatus::Error => &rt.color_error,
                            _ => &rt.color_info,
                        });
                        let icon = self.theme.report_icon(*status).to_string();
                        if self.boxed() {
                            let offset =
                                chars(self.theme.left_pad_str()).saturating_add(chars("│ █  "));
                            let inner_text: String =
                                line.0.iter().skip(offset).map(|c| c.ch).collect();
                            let inner_text = inner_text.trim_end_matches([' ', '│']).to_string();
                            let mut inner = Line::plain(&inner_text);
                            self.paint_slot(&mut inner, &icon, result, &colour);
                            for (j, cell) in inner.0.into_iter().enumerate() {
                                if let Some(target) = line.0.get_mut(j.saturating_add(offset)) {
                                    *target = cell;
                                }
                            }
                        } else {
                            self.paint_slot(&mut line, &icon, result, &colour);
                        }
                    }
                }
                line
            })
            .collect();
        Card {
            component: "Report",
            group: "Reports",
            height: 300,
            subtitle: Some("Results illustrative"),
            terms: vec![("ops deps --check".into(), lines)],
            caption: None,
            spinner_frames: None,
        }
    }

    fn run_transcript(&self) -> Card {
        let mut lines = self.header();
        lines.push(self.step(
            StepStatus::Succeeded,
            "cargo build --all-targets",
            Some(0.31),
        ));
        lines.push(self.step(
            StepStatus::Succeeded,
            "cargo clippy --all-targets -- -D warnings",
            Some(0.21),
        ));
        lines.push(self.step(StepStatus::Failed, "cargo test --workspace", Some(0.01)));
        lines.extend(self.error_lines());
        lines.push(self.summary_line(false));
        Card {
            component: "RunTranscript",
            group: "Run output",
            height: 400,
            subtitle: Some("A complete run"),
            terms: vec![("ops verify".into(), lines)],
            caption: None,
            spinner_frames: None,
        }
    }

    /// The spinner's animation frames: `tick_chars` minus its final rest frame.
    fn frames(&self) -> String {
        let ticks: Vec<char> = self.config.tick_chars.chars().collect();
        ticks.iter().take(ticks.len().saturating_sub(1)).collect()
    }
}

impl Card {
    fn html(&self) -> String {
        let mut out = String::new();
        let subtitle = self
            .subtitle
            .map(|s| format!(r#" subtitle="{s}""#))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            r#"<!-- @dsCard group="{}" height={}{subtitle} -->"#,
            self.group, self.height
        );
        out.push_str(
            "<!-- Generated by `cargo run -p ops-theme --example design_specimens`; do not edit. -->\n",
        );
        out.push_str("<!doctype html>\n<html><body>\n<div class=\"ops-stack\">\n");
        for (title, lines) in &self.terms {
            let _ = write!(
                out,
                r#"<div class="ops-term"><div class="ops-term__bar">{}</div><div class="ops-term__body">"#,
                escape(title)
            );
            for line in lines {
                let _ = write!(out, r#"<span class="ops-line">{}</span>"#, line.html());
            }
            out.push_str("</div></div>\n");
        }
        if let Some(caption) = &self.caption {
            let _ = writeln!(out, r#"<p class="ops-caption">{}</p>"#, escape(caption));
        }
        out.push_str("</div>\n");
        if let Some(frames) = &self.spinner_frames {
            let js = frames.replace('\\', "\\\\").replace('"', "\\\"");
            let _ = write!(
                out,
                "<script>\n(() => {{\n  const frames = [...\"{js}\"];\n  \
                 const els = document.querySelectorAll(\"[data-spin]\");\n  \
                 if (matchMedia(\"(prefers-reduced-motion: reduce)\").matches) return;\n  \
                 let i = 0;\n  setInterval(() => {{ i = (i + 1) % frames.length; \
                 els.forEach((el, k) => {{ el.textContent = frames[(i + k * 3) % frames.length]; }}); }}, 80);\n\
                 }})();\n</script>\n"
            );
        }
        out.push_str("</body></html>\n");
        out
    }
}

/// The run states the boxed borders are shown in.
#[derive(Clone, Copy)]
enum Outcome {
    /// The plan just started: nothing has finished.
    Started,
    /// Every step succeeded.
    Passed,
    /// The last of the steps failed.
    Failed,
}

/// The live snapshot the boxed borders render, for `ids` in `outcome`.
const fn snapshot(ids: &[String], outcome: Outcome) -> BoxSnapshot<'_> {
    let total = ids.len();
    let (completed, failed, success, elapsed_secs) = match outcome {
        Outcome::Started => (0, 0, true, 0.0),
        Outcome::Passed => (total, 0, true, 0.54),
        Outcome::Failed => (total, 1, false, 0.55),
    };
    BoxSnapshot {
        completed,
        failed,
        skipped: 0,
        total,
        elapsed_secs,
        success,
        columns: COLUMNS,
        command_ids: ids,
    }
}

// ------------------------------------------------------------- template

enum Token {
    Literal(String),
    Placeholder {
        name: String,
        width: usize,
        style: String,
    },
}

/// Split an indicatif template into literal text and `{name[:spec]}`
/// placeholders, reading the alignment width and the `.`-separated style.
fn parse_template(template: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut rest = template;
    while let Some((before, after)) = rest.split_once('{') {
        if !before.is_empty() {
            tokens.push(Token::Literal(before.to_string()));
        }
        let Some((inner, tail)) = after.split_once('}') else {
            tokens.push(Token::Literal(format!("{{{after}")));
            return tokens;
        };
        let (name, spec) = inner.split_once(':').unwrap_or((inner, ""));
        let (align, style) = spec.split_once('.').unwrap_or((spec, ""));
        let width = align
            .trim_start_matches(['<', '>', '^'])
            .parse()
            .unwrap_or(0);
        tokens.push(Token::Placeholder {
            name: name.to_string(),
            width,
            style: style.replace('.', " "),
        });
        rest = tail;
    }
    if !rest.is_empty() {
        tokens.push(Token::Literal(rest.to_string()));
    }
    tokens
}

/// A terminal that records what indicatif draws to it.
#[derive(Clone, Debug, Default)]
struct RecordingTerm(Arc<Mutex<Vec<String>>>);

impl RecordingTerm {
    fn record(&self, s: &str) {
        if let Ok(mut lines) = self.0.lock() {
            lines.push(s.to_string());
        }
    }

    fn last_line(&self) -> Option<String> {
        self.0
            .lock()
            .ok()?
            .iter()
            .rev()
            .map(|s| strip_ansi(s))
            .find(|s| !s.trim().is_empty())
    }
}

impl TermLike for RecordingTerm {
    fn width(&self) -> u16 {
        COLUMNS
    }
    fn move_cursor_up(&self, _: usize) -> io::Result<()> {
        Ok(())
    }
    fn move_cursor_down(&self, _: usize) -> io::Result<()> {
        Ok(())
    }
    fn move_cursor_right(&self, _: usize) -> io::Result<()> {
        Ok(())
    }
    fn move_cursor_left(&self, _: usize) -> io::Result<()> {
        Ok(())
    }
    fn write_line(&self, s: &str) -> io::Result<()> {
        self.record(s);
        Ok(())
    }
    fn write_str(&self, s: &str) -> io::Result<()> {
        self.record(s);
        Ok(())
    }
    fn clear_line(&self) -> io::Result<()> {
        Ok(())
    }
    fn flush(&self) -> io::Result<()> {
        Ok(())
    }
}

// --------------------------------------------------------------- tokens

/// Every colour a theme's specimens use must exist in its `tokens.json`, and
/// `bundle.css` must have a rule that reads that token, or the published page
/// would fall back to the default foreground silently.
fn check_tokens(name: &str, cards: &[Card], tokens: &str, bundle_css: &str) -> Result<()> {
    let mut used: Vec<String> = cards
        .iter()
        .flat_map(|card| card.terms.iter())
        .flat_map(|(_, lines)| lines.iter())
        .flat_map(|line| line.0.iter())
        .map(|cell| cell.classes.clone())
        .filter(|c| !c.is_empty())
        .collect();
    used.sort();
    used.dedup();
    for set in used {
        if let Some(token) = token_for(&set) {
            if !tokens.contains(&format!("\"name\": \"{token}\"")) {
                return Err(format!(
                    "theme '{name}' uses `{set}` but its tokens.json defines no `{token}`"
                )
                .into());
            }
            if !bundle_css.contains(&format!("var(--{token})")) {
                return Err(format!(
                    "theme '{name}' uses `{set}` but bundle.css has no rule reading `{token}`"
                )
                .into());
            }
        }
    }
    Ok(())
}

/// The colour token `bundle.css` reads for a set of `sgr-*` classes.
fn token_for(set: &str) -> Option<String> {
    let codes: Vec<&str> = set
        .split(' ')
        .filter_map(|c| c.strip_prefix("sgr-"))
        .collect();
    let dim = codes.contains(&"2");
    let colour = codes.iter().find_map(|code| {
        Some(match *code {
            "30" => "black",
            "31" => "red",
            "32" => "green",
            "33" => "yellow",
            "34" => "blue",
            "35" => "magenta",
            "36" => "cyan",
            "37" => "white",
            "90" => "bright-black",
            "91" => "bright-red",
            "92" => "bright-green",
            "93" => "bright-yellow",
            "94" => "bright-blue",
            "95" => "bright-magenta",
            "96" => "bright-cyan",
            "97" => "bright-white",
            _ => return None,
        })
    });
    match (colour, dim) {
        (Some(c), true) => Some(format!("ansi-{c}-dim")),
        (Some(c), false) => Some(format!("ansi-{c}")),
        (None, true) => Some("term-dim".to_string()),
        (None, false) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUN_HINT: &str = "run `cargo run -p ops-theme --example design_specimens`";

    fn builtin(name: &str) -> Specimens {
        let config: Config = toml::from_str(default_ops_toml()).unwrap();
        let theme = config.themes.get(name).unwrap().clone();
        Specimens::new(name, theme)
    }

    fn painted(classes: &str) -> Card {
        let mut line = Line::plain("x");
        line.paint(0, 1, classes);
        Card {
            component: "Test",
            group: "Test",
            height: 40,
            subtitle: None,
            terms: vec![(String::new(), vec![line])],
            caption: None,
            spinner_frames: None,
        }
    }

    /// The committed previews are what the generator renders from the
    /// current themes, so a theme or renderer change cannot land without
    /// its specimens.
    #[test]
    fn committed_previews_are_up_to_date() {
        let files = render_all(&default_out_dir()).unwrap();
        assert!(!files.is_empty());
        for (path, expected) in files {
            let committed = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{} is missing ({e}); {RUN_HINT}", path.display()));
            assert!(
                committed == expected,
                "{} is stale; {RUN_HINT}",
                path.display()
            );
        }
    }

    /// A running row fills the budget exactly in every built-in theme, from
    /// a fresh step to an hour-long one.
    #[test]
    fn running_rows_fill_the_column_budget() {
        let config: Config = toml::from_str(default_ops_toml()).unwrap();
        for name in config.themes.keys() {
            let specimens = builtin(name);
            for secs in [0, 3_610, 4_210, 359_940] {
                let line = specimens
                    .running("cargo build", Duration::from_secs(secs))
                    .unwrap();
                assert_eq!(
                    line.columns(),
                    usize::from(COLUMNS),
                    "theme '{name}' at {secs}s: {:?}",
                    line.text()
                );
            }
        }
    }

    // `{spinner}` / `{msg}` / `{elapsed}` are indicatif template placeholders
    // under test, not Rust format arguments (docs/clippy.md layer 3).
    #[allow(clippy::literal_string_with_formatting_args)]
    #[test]
    fn parse_template_reads_placeholders_widths_and_styles() {
        let tokens = parse_template("│ ▓  {spinner:.magenta}{msg:.cyan} {elapsed:>6.yellow.dim} │");
        let shape: Vec<String> = tokens
            .iter()
            .map(|t| match t {
                Token::Literal(text) => format!("lit {text:?}"),
                Token::Placeholder { name, width, style } => format!("{name} {width} {style:?}"),
            })
            .collect();
        assert_eq!(
            shape,
            [
                r#"lit "│ ▓  ""#,
                r#"spinner 0 "magenta""#,
                r#"msg 0 "cyan""#,
                r#"lit " ""#,
                r#"elapsed 6 "yellow dim""#,
                r#"lit " │""#,
            ]
        );
    }

    #[test]
    fn parse_template_keeps_an_unclosed_brace_as_text() {
        let tokens = parse_template("  {msg} {elapsed");
        assert!(matches!(tokens.last(), Some(Token::Literal(t)) if t == "{elapsed"));
    }

    /// Classes come from the renderer's own spec parser, aliases included.
    #[test]
    fn classes_compile_specs_through_the_renderer_parser() {
        assert_eq!(classes("bold green"), "sgr-1 sgr-32");
        assert_eq!(classes("yellow dim"), "sgr-33 sgr-2");
        assert_eq!(classes("bright_black"), "sgr-90");
        assert_eq!(classes(""), "");
        assert_eq!(classes("not-a-colour"), "");
    }

    #[test]
    fn token_for_maps_colour_and_dim_codes_to_tokens() {
        assert_eq!(token_for("sgr-36").as_deref(), Some("ansi-cyan"));
        assert_eq!(
            token_for("sgr-33 sgr-2").as_deref(),
            Some("ansi-yellow-dim")
        );
        assert_eq!(
            token_for("sgr-1 sgr-97").as_deref(),
            Some("ansi-bright-white")
        );
        assert_eq!(token_for("sgr-2").as_deref(), Some("term-dim"));
        assert_eq!(token_for("sgr-1"), None);
    }

    #[test]
    fn check_tokens_rejects_a_colour_the_tokens_do_not_define() {
        let card = painted("sgr-35");
        let css = ".sgr-35 { color: var(--ansi-magenta); }";
        let err = check_tokens("t", &[card], r#"{"name": "ansi-cyan"}"#, css).unwrap_err();
        assert!(
            err.to_string().contains("defines no `ansi-magenta`"),
            "{err}"
        );
    }

    #[test]
    fn check_tokens_rejects_a_colour_bundle_css_cannot_draw() {
        let card = painted("sgr-97 sgr-2");
        let tokens = r#"{"name": "ansi-bright-white-dim"}"#;
        let err = check_tokens("t", &[card], tokens, ".sgr-97 {}").unwrap_err();
        assert!(err.to_string().contains("no rule reading"), "{err}");
    }

    #[test]
    fn check_tokens_accepts_a_defined_and_drawn_colour() {
        let card = painted("sgr-33 sgr-2");
        let tokens = r#"{"name": "ansi-yellow-dim"}"#;
        let css = ".sgr-33.sgr-2 { color: var(--ansi-yellow-dim); }";
        assert!(check_tokens("t", &[card], tokens, css).is_ok());
    }

    /// `bundle.css` draws every colour `token_for` can ask for, plain and dim.
    #[test]
    fn bundle_css_has_a_rule_for_every_colour_and_dim_pair() {
        let css = fs::read_to_string(default_out_dir().join("bundle.css")).unwrap();
        let codes = [
            30, 31, 32, 33, 34, 35, 36, 37, 90, 91, 92, 93, 94, 95, 96, 97,
        ];
        for code in codes {
            for set in [format!("sgr-{code}"), format!("sgr-{code} sgr-2")] {
                let token = token_for(&set).unwrap();
                assert!(
                    css.contains(&format!("var(--{token})")),
                    "bundle.css has no rule for `{set}` (`{token}`)"
                );
            }
        }
    }
}
