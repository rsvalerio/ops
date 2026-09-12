//! Task-file model: the frontmatter + body shapes `backlog task ...`
//! (backlog.md CLI v1.51.0) reads and writes, parsed and rendered by one
//! hand-rolled line scanner.
//!
//! Why hand-rolled: a census of every task file in this repository (~2073
//! across `tasks/`, `completed/`, `archive/tasks/`) shows a closed set of
//! shapes — bare, single-quoted (with `''` escapes), and folded `>-` scalar
//! titles; 2-space block lists and flow `[]`; free-form `status`; optional
//! `updated_date`/`priority`/`modified_files`; occasional unknown keys
//! (`type`, `parent_task_id`). A parser whose entire input language is
//! visible in this one file keeps unknown keys and key order (the census
//! shows both vary), needs no new dependency, and is testable against golden
//! fixtures — the same posture the extension's SEC-11-hardened scalar codec
//! (moved here) already proved.

/// Filename slug matching the backlog CLI's observed behaviour: runs of
/// characters outside `[A-Za-z0-9._-]` collapse to a single `-`, and
/// leading/trailing `-` are trimmed. Case is preserved.
///
/// Observed CLI samples this pins:
/// - `"Main task"` → `Main-task`
/// - `"REVIEW: Run skill code-review-rust against ops-core"`
///   → `REVIEW-Run-skill-code-review-rust-against-ops-core`
///
/// Non-ASCII letters are outside the alphabet and collapse like any other
/// run: `"naïve-crate"` → `"na-ve-crate"`, and a wholly non-ASCII title slugs
/// to the empty string. Filenames therefore go through [`file_slug`], which
/// substitutes a placeholder rather than emitting `task-0042 - .md`.
#[must_use = "slugify is pure; discarding it means the title was formatted for nothing"]
pub fn slugify(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut in_run = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-' {
            out.push(ch);
            in_run = false;
        } else if !in_run {
            out.push('-');
            in_run = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Slug substituted when a title carries no character the slug alphabet
/// keeps, so a filename always has a non-empty slug component.
const EMPTY_SLUG: &str = "untitled";

/// [`slugify`] for the filename position, where an empty slug would produce
/// the unusable `task-0042 - .md`.
#[must_use = "the caller needs the placeholder guarantee, not the raw slug"]
pub fn file_slug(title: &str) -> String {
    let slug = slugify(title);
    if slug.is_empty() {
        EMPTY_SLUG.to_string()
    } else {
        slug
    }
}

/// YAML scalar for a frontmatter value.
///
/// SEC-11 lineage: a single-quoted scalar cannot encode a control character —
/// a newline in the value would split the scalar across lines and desynchronise
/// the document, so the `---` terminator is read as a second document and the
/// file the crate just wrote no longer parses. Values are validated at the
/// provider boundary, but the encoder does not rely on that: anything the
/// single-quoted form cannot represent is emitted as a double-quoted scalar
/// with escapes instead.
pub fn yaml_scalar(value: &str) -> String {
    if value.chars().any(char::is_control) {
        yaml_double_quoted(value)
    } else {
        yaml_single_quoted(value)
    }
}

/// A bare scalar when the value survives a round trip unquoted (the byte
/// shape the backlog CLI writes for plain values), else the quoted form
/// from [`yaml_scalar`]. The bare form is unsafe when the value is empty,
/// carries a control character, a `:` or `#` (mapping / comment syntax), or
/// outer whitespace the parser would strip. `status` is user-supplied
/// (`task create -s`, `task edit -s`), so it needs the same encoder
/// guarantee as the title without changing the common-case bytes.
fn bare_or_quoted(value: &str) -> String {
    let plain = !value.is_empty()
        && !value.chars().any(char::is_control)
        && !value.contains(':')
        && !value.contains('#')
        && value.trim() == value;
    if plain {
        value.to_string()
    } else {
        yaml_scalar(value)
    }
}

/// YAML single-quoted scalar: wrap in `'` and double any embedded `'`. The
/// backlog CLI quotes titles containing `: `; quoting unconditionally is
/// byte-compatible for our titles. Only safe for control-character-free
/// values — [`yaml_scalar`] owns that decision.
fn yaml_single_quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len().saturating_add(2));
    out.push('\'');
    out.push_str(&value.replace('\'', "''"));
    out.push('\'');
    out
}

/// YAML double-quoted scalar, the only form that can carry a control
/// character: `\\`, `"` and the C0/C1 controls are escaped, everything else
/// (non-ASCII included) is emitted literally.
fn yaml_double_quoted(value: &str) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(value.len().saturating_add(2));
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // `fmt::Write for String` never returns `Err`, so there is
            // nothing to report and nothing to panic on.
            control if control.is_control() => {
                let _ = write!(out, "\\x{:02x}", u32::from(control));
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

// ---------------------------------------------------------------------------
// Parsed document
// ---------------------------------------------------------------------------

/// One parsed task markdown file: YAML-ish frontmatter plus the body, kept
/// verbatim so edits only touch the sections they mean to touch.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskDoc {
    pub frontmatter: Frontmatter,
    pub body: Body,
}

/// Frontmatter values we model. Unknown keys (and known-but-rare ones like
/// `parent_task_id` and `type`) ride in [`Frontmatter::extras`] in file
/// order and are re-emitted on write.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Frontmatter {
    pub id: String,
    pub title: String,
    /// Free-form: the census shows `Done`, `To Do`, `Triage`; config may add
    /// others, so no enum.
    pub status: String,
    /// Always a list on disk (`assignee: []` or a 2-space block list).
    pub assignees: Vec<String>,
    /// Raw unquoted `'YYYY-MM-DD HH:MM'` value (24 oldest files carry `:SS`).
    pub created_date: String,
    /// Absent in 4 pre-history files.
    pub updated_date: Option<String>,
    pub labels: Vec<String>,
    pub dependencies: Vec<String>,
    /// Absent in 243 files; `low`/`medium`/`high`/`critical` bare scalar.
    pub priority: Option<String>,
    pub modified_files: Vec<String>,
    /// Kept as the raw scalar string, rendered verbatim.
    pub ordinal: Option<String>,
    /// Unknown keys in file order, preserved on write.
    pub extras: Vec<(String, FmValue)>,
}

/// A frontmatter value: scalar or list — the only two shapes in the corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FmValue {
    Scalar(String),
    List(Vec<String>),
}

/// Body of a task file: verbatim text after the closing `---`, with parsed
/// views over the marker-delimited sections.
///
/// Unmarked hand-written sections (`## Closure`, `## Scope`, `## Triage
/// Notes` — all present in the corpus) are just text in [`Body::raw`] and
/// pass through every edit byte-identical.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Body {
    /// Verbatim body after the closing `---` delimiter line, including its
    /// leading newline and trailing newline.
    pub raw: String,
}

/// Section marker lines, exactly as the backlog CLI writes them.
const DESC_BEGIN: &str = "<!-- SECTION:DESCRIPTION:BEGIN -->";
const DESC_END: &str = "<!-- SECTION:DESCRIPTION:END -->";
const AC_BEGIN: &str = "<!-- AC:BEGIN -->";
const AC_END: &str = "<!-- AC:END -->";
const DOD_BEGIN: &str = "<!-- DOD:BEGIN -->";
const DOD_END: &str = "<!-- DOD:END -->";
const NOTES_BEGIN: &str = "<!-- SECTION:NOTES:BEGIN -->";
const NOTES_END: &str = "<!-- SECTION:NOTES:END -->";
const PLAN_BEGIN: &str = "<!-- SECTION:PLAN:BEGIN -->";
const PLAN_END: &str = "<!-- SECTION:PLAN:END -->";

/// One checkbox item. `text` carries no `#N ` prefix; the index is
/// regenerated from list position on write.
///
/// Acceptance criteria and definition-of-done items are the same shape on
/// disk (`- [x] #N text` between their own markers), so one type serves
/// both — see [`DodItem`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcItem {
    pub checked: bool,
    pub text: String,
}

/// One definition-of-done checkbox — the same shape as [`AcItem`], named
/// for the section it belongs to at the call site.
pub type DodItem = AcItem;

impl AcItem {
    /// Fresh, unchecked items from their texts — the shape `--ac` and
    /// `--dod` arrive in at create time, and the shape a wholesale section
    /// replacement builds: a replacement always starts unchecked.
    #[must_use]
    pub fn unchecked_all(texts: &[String]) -> Vec<Self> {
        texts
            .iter()
            .map(|text| Self {
                checked: false,
                text: text.clone(),
            })
            .collect()
    }
}

/// Split a document into frontmatter lines and the verbatim body.
///
/// Only the FIRST `---` pair bounds the frontmatter: task-1834 in the real
/// corpus carries YAML-looking text inside a fenced code block, and later
/// `---` lines in the body are inert by construction here.
///
/// # Errors
///
/// The source does not start with a `---` line, or no closing `---` line
/// follows. The error names what was found instead.
pub fn split_frontmatter(src: &str) -> anyhow::Result<(&str, &str)> {
    let mut lines = src.split('\n');
    let Some(first) = lines.next() else {
        anyhow::bail!("empty file: expected `---` frontmatter delimiter");
    };
    if first.trim_end() != "---" {
        anyhow::bail!("first line is {first:?}, expected `---`: not a backlog task file");
    }
    // Absolute byte offset just past the first line's newline — where the
    // frontmatter content starts.
    let start = first.len().saturating_add(1);
    // Absolute byte offset of the line currently being examined.
    let mut offset = start;
    for line in lines {
        if line.trim_end() == "---" {
            let fm = src
                .get(start..offset)
                .unwrap_or("")
                .strip_suffix('\n')
                .unwrap_or("");
            let after_marker = offset.saturating_add(line.len());
            let body = src.get(after_marker..).unwrap_or("");
            let body = body.strip_prefix('\n').unwrap_or(body);
            return Ok((fm, body));
        }
        offset = offset.saturating_add(line.len()).saturating_add(1);
    }
    anyhow::bail!("no closing `---` delimiter after the frontmatter")
}

impl TaskDoc {
    /// Parse a task file from its full text.
    ///
    /// # Errors
    ///
    /// As [`split_frontmatter`], or a frontmatter line that is neither a
    /// `key: value` entry nor part of one; the error names the line number.
    pub fn parse(src: &str) -> anyhow::Result<Self> {
        let (fm_src, body) = split_frontmatter(src)?;
        let frontmatter = Frontmatter::parse(fm_src)?;
        Ok(Self {
            frontmatter,
            body: Body {
                raw: body.to_string(),
            },
        })
    }

    /// Render the whole file: canonical frontmatter between `---` delimiters,
    /// then the body verbatim.
    #[must_use = "rendering is pure; the caller writes the bytes"]
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(self.body.raw.len().saturating_add(512));
        out.push_str("---\n");
        self.frontmatter.render_into(&mut out);
        out.push_str("---\n");
        out.push_str(&self.body.raw);
        out
    }
}

impl Frontmatter {
    /// The scalar value of an extras key (`parent_task_id`, `type`, …),
    /// when that key carries one.
    #[must_use = "querying extras is pure; discarding the value wastes the scan"]
    pub fn extra_scalar(&self, key: &str) -> Option<&str> {
        self.extras.iter().find_map(|(k, v)| {
            if k == key {
                match v {
                    FmValue::Scalar(s) => Some(s.as_str()),
                    FmValue::List(_) => None,
                }
            } else {
                None
            }
        })
    }

    /// Set an extras key to a scalar, replacing any prior value for it or
    /// appending it when absent.
    pub fn set_extra_scalar(&mut self, key: &str, value: &str) {
        match self.extras.iter_mut().find(|(k, _)| k == key) {
            Some((_, v)) => *v = FmValue::Scalar(value.to_string()),
            None => self
                .extras
                .push((key.to_string(), FmValue::Scalar(value.to_string()))),
        }
    }

    /// Remove an extras key entirely.
    pub fn remove_extra(&mut self, key: &str) {
        self.extras.retain(|(k, _)| k != key);
    }

    /// Parse the text between the `---` delimiters.
    ///
    /// # Errors
    ///
    /// A line at column 0 that is not `key: value` (and not the indented
    /// continuation of one), or a value in a shape the census never observed.
    pub fn parse(src: &str) -> anyhow::Result<Self> {
        let lines: Vec<&str> = src.split('\n').collect();
        let mut fm = Self::default();
        let mut i = 0usize;
        while let Some(line) = lines.get(i) {
            if line.trim().is_empty() {
                i = i.saturating_add(1);
                continue;
            }
            let (key, rest) = split_key(line, i)?;
            let (value, consumed) = parse_value(&lines, i, rest)?;
            match key {
                "id" => fm.id = scalar(&value, i, "id")?.to_string(),
                "title" => fm.title = scalar(&value, i, "title")?.to_string(),
                "status" => fm.status = scalar(&value, i, "status")?.to_string(),
                "assignee" => fm.assignees = list(value, "assignee"),
                "created_date" => fm.created_date = scalar(&value, i, "created_date")?.to_string(),
                "updated_date" => {
                    fm.updated_date = Some(scalar(&value, i, "updated_date")?.to_string());
                }
                "labels" => fm.labels = list(value, "labels"),
                "dependencies" => fm.dependencies = list(value, "dependencies"),
                "priority" => fm.priority = Some(scalar(&value, i, "priority")?.to_string()),
                "modified_files" => fm.modified_files = list(value, "modified_files"),
                "ordinal" => fm.ordinal = Some(scalar(&value, i, "ordinal")?.to_string()),
                _ => fm.extras.push((key.to_string(), value)),
            }
            i = i.saturating_add(consumed);
        }
        Ok(fm)
    }

    /// Canonical key order the backlog CLI writes (pinned by golden tests
    /// against real files): known keys first, unknown keys after `ordinal`
    /// in their original file order. `parent_task_id` — unknown to the model
    /// but position-stable in the corpus — is re-emitted right after
    /// `dependencies`, where the CLI puts it.
    fn render_into(&self, out: &mut String) {
        use std::fmt::Write as _;

        let _ = writeln!(out, "id: {}", self.id);
        let _ = writeln!(out, "title: {}", yaml_scalar(&self.title));
        let _ = writeln!(out, "status: {}", bare_or_quoted(&self.status));
        render_list(out, "assignee", &self.assignees);
        let _ = writeln!(out, "created_date: '{}'", self.created_date);
        if let Some(updated) = &self.updated_date {
            let _ = writeln!(out, "updated_date: '{updated}'");
        }
        render_list(out, "labels", &self.labels);
        render_list(out, "dependencies", &self.dependencies);
        let parent = self.extras.iter().find(|(k, _)| k == "parent_task_id");
        if let Some((_, v)) = parent {
            render_extra(out, "parent_task_id", v);
        }
        render_list(out, "modified_files", &self.modified_files);
        if let Some(priority) = &self.priority {
            let _ = writeln!(out, "priority: {priority}");
        }
        if let Some(ordinal) = &self.ordinal {
            let _ = writeln!(out, "ordinal: {ordinal}");
        }
        for (key, value) in &self.extras {
            if key == "parent_task_id" {
                continue;
            }
            render_extra(out, key, value);
        }
    }
}

/// Split a `key: rest` line at column 0.
///
/// # Errors
///
/// The line is not a `key:` entry; the error names the 1-based line number
/// within the frontmatter block.
fn split_key(line: &str, index: usize) -> anyhow::Result<(&str, &str)> {
    let line_no = index.saturating_add(1);
    let Some(colon) = line.find(':') else {
        anyhow::bail!("frontmatter line {line_no}: {line:?} is not `key: value`");
    };
    let key = line.get(..colon).unwrap_or("");
    if key.is_empty()
        || !key
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        anyhow::bail!("frontmatter line {line_no}: {line:?} is not `key: value`");
    }
    let rest = line.get(colon.saturating_add(1)..).unwrap_or("");
    Ok((key, rest))
}

/// Parse the value starting at `lines[i]` (whose post-colon text is `rest`),
/// returning the value and how many lines it consumed (including the key
/// line itself).
///
/// # Errors
///
/// A shape the census never observed (e.g. a flow list with items).
fn parse_value(lines: &[&str], i: usize, rest: &str) -> anyhow::Result<(FmValue, usize)> {
    let rest = rest.strip_prefix(' ').unwrap_or(rest);
    if rest.is_empty() {
        // Block list follows, or the key simply had no value (treated as an
        // empty list — the tolerant reading; the corpus never exercises it).
        let mut items = Vec::new();
        let mut j = i.saturating_add(1);
        while let Some(candidate) = lines.get(j) {
            if let Some(item) = block_list_item(candidate) {
                items.push(item.to_string());
                j = j.saturating_add(1);
            } else {
                break;
            }
        }
        return Ok((FmValue::List(items), j.saturating_sub(i)));
    }
    if rest == "[]" {
        return Ok((FmValue::List(Vec::new()), 1));
    }
    if let Some(quoted) = rest.strip_prefix('\'') {
        return Ok((FmValue::Scalar(decode_single_quoted(quoted, i)?), 1));
    }
    if let Some(quoted) = rest.strip_prefix('"') {
        return Ok((FmValue::Scalar(decode_double_quoted(quoted, i)?), 1));
    }
    if rest == ">-" || rest == ">" {
        // Folded: following lines indented by 2, dedented, joined with a
        // single space (YAML folded semantics for our no-blank-line corpus).
        let mut parts = Vec::new();
        let mut j = i.saturating_add(1);
        while let Some(candidate) = lines.get(j) {
            if let Some(dedented) = candidate.strip_prefix("  ") {
                if !dedented.trim().is_empty() {
                    parts.push(dedented.trim_end().to_string());
                }
                j = j.saturating_add(1);
            } else {
                break;
            }
        }
        return Ok((FmValue::Scalar(parts.join(" ")), j.saturating_sub(i)));
    }
    // Bare scalar.
    Ok((FmValue::Scalar(rest.trim().to_string()), 1))
}

/// A `  - item` block-list line (2-space indent, dash, space).
fn block_list_item(line: &str) -> Option<&str> {
    let after_indent = line.strip_prefix("  ")?;
    let item = after_indent.strip_prefix("- ")?;
    Some(item.trim_end())
}

/// Decode a single-quoted scalar whose opening quote was already stripped.
/// An inner quote is a doubled `''`, so stripping one trailing `'` and
/// un-doubling is the whole decode.
///
/// # Errors
///
/// No closing quote on the line — multi-line quoted scalars are not in the
/// corpus and are refused rather than guessed at.
fn decode_single_quoted(quoted: &str, i: usize) -> anyhow::Result<String> {
    let body = quoted.strip_suffix('\'').ok_or_else(|| {
        anyhow::anyhow!(
            "frontmatter line {}: unterminated single-quoted scalar",
            i.saturating_add(1)
        )
    })?;
    Ok(body.replace("''", "'"))
}

/// Decode a double-quoted scalar whose opening quote was already stripped.
///
/// # Errors
///
/// An unsupported escape, or the closing quote never arrives.
fn decode_double_quoted(quoted: &str, i: usize) -> anyhow::Result<String> {
    let line_no = i.saturating_add(1);
    let Some(end) = quoted.rfind('"') else {
        anyhow::bail!("frontmatter line {line_no}: unterminated double-quoted scalar");
    };
    let body = quoted.get(..end).unwrap_or("");
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(escaped) => {
                anyhow::bail!("frontmatter line {line_no}: unsupported escape `\\{escaped}`");
            }
            None => anyhow::bail!("frontmatter line {line_no}: trailing backslash"),
        }
    }
    Ok(out)
}

/// Extract a scalar value, or name the key that carried a list instead.
fn scalar<'a>(value: &'a FmValue, i: usize, key: &str) -> anyhow::Result<&'a str> {
    match value {
        FmValue::Scalar(s) => Ok(s),
        FmValue::List(_) => anyhow::bail!(
            "frontmatter line {}: `{key}` carries a list where a scalar was expected",
            i.saturating_add(1)
        ),
    }
}

/// Extract a list value; a scalar degenerates to a one-element list (the
/// tolerant reading — the corpus never mixes the two for one key).
fn list(value: FmValue, _key: &str) -> Vec<String> {
    match value {
        FmValue::List(items) => items,
        FmValue::Scalar(s) => vec![s],
    }
}

/// Render one `key: value` frontmatter line pair for an extras entry.
fn render_extra(out: &mut String, key: &str, value: &FmValue) {
    use std::fmt::Write as _;

    match value {
        FmValue::Scalar(s) => {
            let _ = writeln!(out, "{key}: {}", yaml_scalar(s));
        }
        FmValue::List(items) => render_list(out, key, items),
    }
}

/// `key: []` when empty, else `key:` followed by 2-space block items.
fn render_list(out: &mut String, key: &str, items: &[String]) {
    use std::fmt::Write as _;

    if items.is_empty() {
        let _ = writeln!(out, "{key}: []");
        return;
    }
    let _ = writeln!(out, "{key}:");
    for item in items {
        let _ = writeln!(out, "  - {item}");
    }
}

// ---------------------------------------------------------------------------
// Body sections
// ---------------------------------------------------------------------------

impl Body {
    /// Content between two marker lines, newline-edges trimmed. `None` when
    /// either marker is absent (31 corpus files carry no body at all).
    #[must_use = "querying the body is pure"]
    pub fn section(&self, begin: &str, end: &str) -> Option<&str> {
        let begin_line = format!("{begin}\n");
        let end_line = format!("\n{end}");
        let start = self
            .raw
            .find(&begin_line)
            .and_then(|p| p.checked_add(begin_line.len()))?;
        let stop = self
            .raw
            .get(start..)?
            .find(&end_line)
            .and_then(|p| p.checked_add(start))?;
        Some(self.raw.get(start..stop)?.trim_matches('\n'))
    }

    /// Replace the content between two existing marker lines; `false` when
    /// either marker is absent (caller decides whether to append a fresh
    /// section instead).
    fn replace_between(&mut self, begin: &str, end: &str, content: &str) -> bool {
        let Some(new_raw) = self.replace_between_opt(begin, end, content) else {
            return false;
        };
        self.raw = new_raw;
        true
    }

    fn replace_between_opt(&self, begin: &str, end: &str, content: &str) -> Option<String> {
        let begin_line = format!("{begin}\n");
        let end_line = format!("\n{end}");
        let start = self
            .raw
            .find(&begin_line)
            .and_then(|p| p.checked_add(begin_line.len()))?;
        let stop = self
            .raw
            .get(start..)?
            .find(&end_line)
            .and_then(|p| p.checked_add(start))?;
        let mut out = String::with_capacity(self.raw.len());
        out.push_str(self.raw.get(..start)?);
        if !content.is_empty() {
            out.push_str(content);
            out.push('\n');
        }
        out.push_str(self.raw.get(stop..)?);
        Some(out)
    }

    /// The task description between SECTION:DESCRIPTION markers.
    #[must_use = "querying the body is pure"]
    pub fn description(&self) -> Option<&str> {
        self.section(DESC_BEGIN, DESC_END)
    }

    /// Implementation notes between SECTION:NOTES markers.
    #[must_use = "querying the body is pure"]
    pub fn notes(&self) -> Option<&str> {
        self.section(NOTES_BEGIN, NOTES_END)
    }

    /// Implementation plan between SECTION:PLAN markers.
    #[must_use = "querying the body is pure"]
    pub fn plan(&self) -> Option<&str> {
        self.section(PLAN_BEGIN, PLAN_END)
    }

    /// Acceptance-criteria checkboxes between AC markers, `#N ` prefixes
    /// stripped.
    #[must_use = "querying the body is pure"]
    pub fn ac_items(&self) -> Vec<AcItem> {
        self.checkbox_items(AC_BEGIN, AC_END)
    }

    /// Definition-of-done checkboxes between DOD markers, `#N ` prefixes
    /// stripped. Same line shape as the acceptance criteria above.
    #[must_use = "querying the body is pure"]
    pub fn dod_items(&self) -> Vec<DodItem> {
        self.checkbox_items(DOD_BEGIN, DOD_END)
    }

    /// Parse `- [x] #N text` checkbox lines between one marker pair.
    fn checkbox_items(&self, begin: &str, end: &str) -> Vec<AcItem> {
        let Some(block) = self.section(begin, end) else {
            return Vec::new();
        };
        block
            .split('\n')
            .filter_map(|line| {
                let checked = if let Some(rest) = line.strip_prefix("- [x] ") {
                    rest
                } else {
                    line.strip_prefix("- [ ] ")?
                };
                // Census: 100% of checkbox lines carry `#N `; strip it when
                // present so the number is owned by list position on write.
                let text = strip_ac_index(checked);
                Some(AcItem {
                    checked: line.starts_with("- [x] "),
                    text: text.to_string(),
                })
            })
            .collect()
    }

    /// Write the description section: replace between existing markers, or
    /// append a fresh `## Description` section when absent.
    pub fn set_description(&mut self, content: &str) {
        if !self.replace_between(DESC_BEGIN, DESC_END, content) {
            self.push_section("## Description", true, (DESC_BEGIN, DESC_END), content);
        }
    }

    /// Write the acceptance-criteria section from items, regenerating `#N`.
    pub fn set_ac(&mut self, items: &[AcItem]) {
        self.set_checkboxes(items, (AC_BEGIN, AC_END), "## Acceptance Criteria");
    }

    /// Write the definition-of-done section from items, regenerating `#N`.
    ///
    /// A body with no definition-of-done section yet gains one at the end,
    /// the same way [`Body::set_ac`] appends acceptance criteria — the CLI
    /// writes it right after the criteria at create time, and both parse.
    pub fn set_dod(&mut self, items: &[DodItem]) {
        self.set_checkboxes(items, (DOD_BEGIN, DOD_END), "## Definition of Done");
    }

    /// Render checkbox items between one marker pair, regenerating `#N` from
    /// list position; append the section under `header` when it is absent.
    fn set_checkboxes(&mut self, items: &[AcItem], markers: (&str, &str), header: &str) {
        use std::fmt::Write as _;

        let mut block = String::new();
        for (idx, item) in items.iter().enumerate() {
            let mark = if item.checked { "x" } else { " " };
            let _ = writeln!(
                &mut block,
                "- [{mark}] #{} {}",
                idx.saturating_add(1),
                item.text
            );
        }
        let block = block.trim_matches('\n');
        let (begin, end) = markers;
        if !self.replace_between(begin, end, block) {
            self.push_section(header, false, markers, block);
        }
    }

    /// Set the checked state of the 1-based criterion `index`.
    ///
    /// # Errors
    ///
    /// No criterion with that index.
    pub fn set_ac_checked(&mut self, index: usize, checked: bool) -> anyhow::Result<()> {
        let mut items = self.ac_items();
        check_item_at(&mut items, index, checked, "acceptance criterion")?;
        self.set_ac(&items);
        Ok(())
    }

    /// Set the checked state of the 1-based definition-of-done item `index`.
    ///
    /// # Errors
    ///
    /// No definition-of-done item with that index.
    pub fn set_dod_checked(&mut self, index: usize, checked: bool) -> anyhow::Result<()> {
        let mut items = self.dod_items();
        check_item_at(&mut items, index, checked, "definition-of-done item")?;
        self.set_dod(&items);
        Ok(())
    }

    /// Append implementation notes: inside existing SECTION:NOTES markers
    /// (separated from what is there by a blank line), or as a fresh
    /// `## Implementation Notes` section at the end of the body.
    pub fn append_notes(&mut self, text: &str) {
        let merged = match self.notes() {
            Some(existing) if !existing.is_empty() => format!("{existing}\n\n{text}"),
            _ => text.to_string(),
        };
        if self.replace_between(NOTES_BEGIN, NOTES_END, &merged) {
            return;
        }
        self.push_section(
            "## Implementation Notes",
            true,
            (NOTES_BEGIN, NOTES_END),
            text,
        );
    }

    /// Set the implementation plan section.
    pub fn set_plan(&mut self, content: &str) {
        if !self.replace_between(PLAN_BEGIN, PLAN_END, content) {
            self.push_section(
                "## Implementation Plan",
                true,
                (PLAN_BEGIN, PLAN_END),
                content,
            );
        }
    }

    /// Append a fresh section to the body. `blank_after_header` matches the
    /// CLI's per-section shapes: Description/Notes/Plan carry a blank line
    /// between header and marker, Acceptance Criteria does not.
    fn push_section(
        &mut self,
        header: &str,
        blank_after_header: bool,
        markers: (&str, &str),
        content: &str,
    ) {
        use std::fmt::Write as _;

        let (begin, end) = markers;
        // Sections are separated by one blank line; the body always ends
        // with a newline.
        let mut trimmed = self.raw.trim_end_matches('\n').to_string();
        if !trimmed.is_empty() {
            trimmed.push('\n');
        }
        let section = format!(
            "\n{header}{}{begin}\n{content}\n{end}\n",
            // Header line terminator, plus a blank line for the sections the
            // CLI separates from their marker (Description/Notes/Plan).
            if blank_after_header { "\n\n" } else { "\n" }
        );
        let _ = write!(&mut trimmed, "{section}");
        self.raw = trimmed;
    }
}

/// Set the checked state of the 1-based `index` in a checkbox list.
///
/// # Errors
///
/// No item with that index — the message names `what` ("acceptance
/// criterion", "definition-of-done item") so the CLI error points at the
/// section the caller meant.
fn check_item_at(
    items: &mut [AcItem],
    index: usize,
    checked: bool,
    what: &str,
) -> anyhow::Result<()> {
    let Some(item) = index
        .checked_sub(1)
        .and_then(|zero_based| items.get_mut(zero_based))
    else {
        anyhow::bail!("no {what} #{index}");
    };
    item.checked = checked;
    Ok(())
}

/// Strip a leading `#N ` from an acceptance-criterion line, when present.
fn strip_ac_index(text: &str) -> &str {
    let Some(rest) = text.strip_prefix('#') else {
        return text;
    };
    let digits_len = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits_len == 0 {
        return text;
    }
    rest.get(digits_len..)
        .and_then(|tail| tail.strip_prefix(' '))
        .unwrap_or(text)
}

/// Write a task file body from scratch, in the CLI's create-time shape.
#[must_use = "building the body is pure; the caller writes the file"]
pub fn render_body(
    description: &str,
    ac: &[AcItem],
    dod: &[DodItem],
    plan: Option<&str>,
    notes: Option<&str>,
) -> String {
    let mut body = Body::default();
    body.set_description(description);
    body.set_ac(ac);
    // The CLI writes Definition of Done between the criteria and the plan.
    if !dod.is_empty() {
        body.set_dod(dod);
    }
    if let Some(plan) = plan {
        body.set_plan(plan);
    }
    if let Some(notes) = notes {
        body.append_notes(notes);
    }
    body.raw
}

// ---------------------------------------------------------------------------
// Tests: one golden per census shape
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
---
id: TASK-2069
title: >-
  DUP-3: six more hand-rolled tracing-capture scaffolds outside the TASK-2058
  enumeration
status: Done
assignee: []
created_date: '2026-08-29 18:21'
updated_date: '2026-08-31 17:38'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - crates/runner/src/command/tests/parallel.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/foo/src/lib.rs:42`

**What**: something
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 first criterion
- [ ] #2 second criterion
<!-- AC:END -->
";

    /// `status` is user-supplied, so a YAML-hostile status must come back
    /// out quoted: the file this crate writes has to re-parse (the same
    /// SEC-11 round-trip rule the module header states for titles), while
    /// a plain status keeps its bare CLI byte shape.
    #[test]
    fn hostile_status_round_trips_and_plain_status_stays_bare() {
        let mut doc = TaskDoc::parse(SAMPLE).expect("must parse");

        doc.frontmatter.status = "In Progress".to_string();
        assert!(
            doc.render().contains("\nstatus: In Progress\n"),
            "plain statuses keep the bare CLI byte shape"
        );

        for hostile in ["needs: quoting", "hash # inside", " padded ", ""] {
            doc.frontmatter.status = hostile.to_string();
            let rendered = doc.render();
            let reparsed = TaskDoc::parse(&rendered)
                .unwrap_or_else(|e| panic!("status {hostile:?} must round-trip: {e:#}"));
            assert_eq!(
                reparsed.frontmatter.status, hostile,
                "the rendered status must survive its own writer: {rendered}"
            );
        }
    }

    #[test]
    fn parse_reads_folded_title_and_sections() {
        let doc = TaskDoc::parse(SAMPLE).expect("must parse");
        assert_eq!(doc.frontmatter.id, "TASK-2069");
        assert_eq!(
            doc.frontmatter.title,
            "DUP-3: six more hand-rolled tracing-capture scaffolds outside the \
             TASK-2058 enumeration"
        );
        assert_eq!(doc.frontmatter.status, "Done");
        assert!(doc.frontmatter.assignees.is_empty());
        assert_eq!(doc.frontmatter.created_date, "2026-08-29 18:21");
        assert_eq!(
            doc.frontmatter.modified_files,
            vec!["crates/runner/src/command/tests/parallel.rs"]
        );
        assert_eq!(doc.frontmatter.priority.as_deref(), Some("low"));
        assert_eq!(
            doc.body.description(),
            Some("**File**: `crates/foo/src/lib.rs:42`\n\n**What**: something")
        );
        let ac = doc.body.ac_items();
        assert_eq!(ac.len(), 2);
        assert!(ac[0].checked);
        assert_eq!(ac[0].text, "first criterion");
        assert!(!ac[1].checked);
        assert_eq!(ac[1].text, "second criterion");
    }

    #[test]
    fn parse_render_round_trip_is_stable() {
        let doc = TaskDoc::parse(SAMPLE).expect("must parse");
        let once = doc.render();
        let twice = TaskDoc::parse(&once).expect("re-parse").render();
        assert_eq!(once, twice, "render must be a fixed point");
    }

    #[test]
    fn single_quoted_title_with_escaped_quotes() {
        // Census: 7 quoted titles contain YAML-doubled quotes.
        let src = SAMPLE.replacen(
            "title: >-\n  DUP-3: six more hand-rolled tracing-capture scaffolds outside the TASK-2058\n  enumeration\n",
            "title: 'include a, b and it''s fine'\n",
            1,
        );
        let doc = TaskDoc::parse(&src).expect("must parse");
        assert_eq!(doc.frontmatter.title, "include a, b and it's fine");
    }

    #[test]
    fn bare_scalar_title_and_missing_optional_keys() {
        // Census: ~239 old files have bare titles; 4 lack updated_date; 243
        // lack priority.
        let src = "---\nid: TASK-0001\ntitle: Old style plain title\nstatus: Done\nassignee: []\ncreated_date: '2026-04-10 07:15:00'\nlabels: []\ndependencies: []\n---\n";
        let doc = TaskDoc::parse(src).expect("must parse");
        assert_eq!(doc.frontmatter.title, "Old style plain title");
        assert_eq!(doc.frontmatter.created_date, "2026-04-10 07:15:00");
        assert_eq!(doc.frontmatter.updated_date, None);
        assert_eq!(doc.frontmatter.priority, None);
        assert!(doc.body.raw.is_empty());
    }

    #[test]
    fn unknown_keys_survive_round_trip_in_order() {
        let src = "---\nid: TASK-2039\ntitle: 'x'\nstatus: To Do\nassignee: []\ncreated_date: '2026-08-01 10:00'\nlabels: []\ndependencies: []\ntype: enhancement\nparent_task_id: TASK-2000\npriority: high\n---\nbody\n";
        let doc = TaskDoc::parse(src).expect("must parse");
        assert_eq!(
            doc.frontmatter.extras,
            vec![
                (
                    "type".to_string(),
                    FmValue::Scalar("enhancement".to_string())
                ),
                (
                    "parent_task_id".to_string(),
                    FmValue::Scalar("TASK-2000".to_string())
                ),
            ]
        );
        let rendered = doc.render();
        let reparsed = TaskDoc::parse(&rendered).expect("re-parse");
        // `parent_task_id` is deliberately repositioned next to
        // `dependencies` on write, so compare extras as a sorted key set —
        // nothing may be dropped, values must survive.
        let mut before = doc.frontmatter.extras;
        let mut after = reparsed.frontmatter.extras;
        before.sort_by(|a, b| a.0.cmp(&b.0));
        after.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(after, before);
        // parent_task_id renders right after dependencies, before priority.
        let deps = rendered.find("dependencies: []").expect("deps");
        let parent = rendered
            .find("parent_task_id: 'TASK-2000'")
            .expect("parent");
        let priority = rendered.find("priority: high").expect("priority");
        assert!(deps < parent && parent < priority);
    }

    #[test]
    fn fenced_yaml_in_body_is_inert() {
        // task-1834: YAML-looking text inside a fenced code block must not
        // confuse the frontmatter split.
        let src = "---\nid: TASK-1834\ntitle: 't'\nstatus: To Do\nassignee: []\ncreated_date: '2026-06-01 09:00'\nlabels: []\ndependencies: []\n---\n\n## Description\n\n<!-- SECTION:DESCRIPTION:BEGIN -->\n```\nstatus: To Do\ntitle: not frontmatter\n---\n```\n<!-- SECTION:DESCRIPTION:END -->\n";
        let doc = TaskDoc::parse(src).expect("must parse");
        assert_eq!(doc.frontmatter.status, "To Do");
        assert_eq!(
            doc.body.description(),
            Some("```\nstatus: To Do\ntitle: not frontmatter\n---\n```")
        );
    }

    #[test]
    fn first_delimiter_pair_bounds_frontmatter() {
        let (fm, body) =
            split_frontmatter("---\nid: TASK-0001\n---\nrest\n---\nmore\n").expect("split");
        assert_eq!(fm, "id: TASK-0001");
        assert_eq!(body, "rest\n---\nmore\n");
    }

    #[test]
    fn missing_delimiters_are_errors() {
        assert!(TaskDoc::parse("no delimiters at all\n").is_err());
        assert!(TaskDoc::parse("---\nid: TASK-0001\n").is_err());
        assert!(TaskDoc::parse("").is_err());
    }

    #[test]
    fn unmarked_sections_pass_through_edits_byte_identical() {
        let src = "---\nid: TASK-0002\ntitle: 't'\nstatus: To Do\nassignee: []\ncreated_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\n---\n\n## Closure\n\nHand-written, no markers.\n";
        let mut doc = TaskDoc::parse(src).expect("must parse");
        doc.body.append_notes("a note");
        let rendered = doc.render();
        assert!(
            rendered.contains("## Closure\n\nHand-written, no markers.\n"),
            "unmarked section must survive verbatim, got: {rendered}"
        );
        assert!(rendered.contains("## Implementation Notes"));
    }

    #[test]
    fn append_notes_inserts_before_end_marker_with_blank_separation() {
        let mut doc = TaskDoc::parse(SAMPLE).expect("must parse");
        doc.body.append_notes("Overlaps: TASK-0119");
        doc.body.append_notes("Branch: code-review/TASK-2069");
        let notes = doc.body.notes().expect("notes section");
        assert_eq!(
            notes, "Overlaps: TASK-0119\n\nBranch: code-review/TASK-2069",
            "appends separate with a blank line, got: {notes}"
        );
        // The AC section after the inserted notes is still parseable.
        assert_eq!(doc.body.ac_items().len(), 2);
    }

    #[test]
    fn set_ac_regenerates_indices_and_states() {
        let mut doc = TaskDoc::parse(SAMPLE).expect("must parse");
        doc.body.set_ac_checked(2, true).expect("check #2");
        let items = doc.body.ac_items();
        assert!(items[0].checked && items[1].checked);
        doc.body.set_ac_checked(1, false).expect("uncheck #1");
        let items = doc.body.ac_items();
        assert!(!items[0].checked && items[1].checked);
        assert!(doc.body.set_ac_checked(3, true).is_err());
        let rendered = doc.render();
        assert!(rendered.contains("- [ ] #1 first criterion"));
        assert!(rendered.contains("- [x] #2 second criterion"));
    }

    #[test]
    fn render_body_from_scratch_matches_cli_create_shape() {
        let body = render_body(
            "**File**: `crates/foo/src/lib.rs:42`",
            &[AcItem {
                checked: false,
                text: "criterion one".to_string(),
            }],
            &[],
            None,
            None,
        );
        let expected = "\
\n## Description\n\n<!-- SECTION:DESCRIPTION:BEGIN -->\n**File**: `crates/foo/src/lib.rs:42`\n<!-- SECTION:DESCRIPTION:END -->\n\n## Acceptance Criteria\n<!-- AC:BEGIN -->\n- [ ] #1 criterion one\n<!-- AC:END -->\n";
        assert_eq!(body, expected);
    }

    #[test]
    fn render_body_places_dod_between_criteria_and_plan() {
        let body = render_body(
            "desc",
            &[AcItem {
                checked: false,
                text: "criterion one".to_string(),
            }],
            &[DodItem {
                checked: false,
                text: "gate one".to_string(),
            }],
            Some("the plan"),
            None,
        );
        // The section order the backlog CLI writes at create time.
        let offsets: Vec<usize> = [
            body.find("## Description").expect("description"),
            body.find("## Acceptance Criteria").expect("ac"),
            body.find("## Definition of Done").expect("dod"),
            body.find("## Implementation Plan").expect("plan"),
        ]
        .to_vec();
        assert!(offsets.windows(2).all(|w| w[0] < w[1]));
        assert!(body.contains("<!-- DOD:BEGIN -->\n- [ ] #1 gate one\n<!-- DOD:END -->"));
    }

    #[test]
    fn dod_items_parse_and_check_independently_of_ac() {
        // A body carrying both sections: checking a DoD item must not touch
        // the criteria, and the `#N` numbering is per section.
        let mut doc = TaskDoc::parse(
            "---\nid: TASK-0001\ntitle: 'x'\nstatus: Triage\n---\n\n## Acceptance Criteria\n<!-- AC:BEGIN -->\n- [ ] #1 criterion\n<!-- AC:END -->\n\n## Definition of Done\n<!-- DOD:BEGIN -->\n- [ ] #1 gate one\n- [ ] #2 gate two\n<!-- DOD:END -->\n",
        )
        .expect("must parse");
        assert_eq!(doc.body.dod_items().len(), 2);
        doc.body.set_dod_checked(2, true).expect("check #2");
        let rendered = doc.render();
        assert!(rendered.contains("- [ ] #1 gate one"));
        assert!(rendered.contains("- [x] #2 gate two"));
        assert!(rendered.contains("- [ ] #1 criterion"));
        assert!(doc.body.ac_items().iter().all(|item| !item.checked));
        let err = doc
            .body
            .set_dod_checked(9, true)
            .expect_err("index out of range");
        assert!(err.to_string().contains("no definition-of-done item #9"));
    }

    #[test]
    fn dod_survives_an_edit_that_does_not_mention_it() {
        // ops must not eat a DoD section written by the npm CLI.
        let source = "---\nid: TASK-0001\ntitle: 'x'\nstatus: Triage\n---\n\n## Definition of Done\n<!-- DOD:BEGIN -->\n- [x] #1 gate one\n<!-- DOD:END -->\n";
        let mut doc = TaskDoc::parse(source).expect("must parse");
        doc.body.set_description("new description");
        let rendered = doc.render();
        assert!(rendered.contains("- [x] #1 gate one"));
        assert_eq!(doc.body.dod_items().len(), 1);
    }

    #[test]
    fn render_frontmatter_uses_canonical_order() {
        let doc = TaskDoc::parse(SAMPLE).expect("must parse");
        let rendered = doc.render();
        let offsets: Vec<usize> = [
            rendered.find("id: ").expect("id"),
            rendered.find("title: ").expect("title"),
            rendered.find("status: ").expect("status"),
            rendered.find("assignee: ").expect("assignee"),
            rendered.find("created_date: ").expect("created"),
            rendered.find("updated_date: ").expect("updated"),
            rendered.find("\nlabels:").expect("labels"),
            rendered.find("\ndependencies:").expect("deps"),
            rendered.find("\nmodified_files:").expect("modified"),
            rendered.find("\npriority: ").expect("priority"),
        ]
        .to_vec();
        let mut sorted = offsets.clone();
        sorted.sort_unstable();
        assert_eq!(offsets, sorted, "keys must render in canonical order");
    }

    #[test]
    fn extra_mutators_set_clear_and_replace() {
        let mut doc = TaskDoc::parse(SAMPLE).expect("must parse");
        doc.frontmatter
            .set_extra_scalar("parent_task_id", "TASK-2000");
        assert_eq!(
            doc.frontmatter.extra_scalar("parent_task_id"),
            Some("TASK-2000")
        );
        doc.frontmatter
            .set_extra_scalar("parent_task_id", "TASK-3000");
        assert_eq!(
            doc.frontmatter.extra_scalar("parent_task_id"),
            Some("TASK-3000"),
            "a second set must replace, not append"
        );
        doc.frontmatter.remove_extra("parent_task_id");
        assert_eq!(doc.frontmatter.extra_scalar("parent_task_id"), None);
        let rendered = doc.render();
        assert!(!rendered.contains("parent_task_id"));
    }

    #[test]
    fn slugify_and_yaml_codec_pins() {
        assert_eq!(slugify("Main task"), "Main-task");
        assert_eq!(slugify("a  b"), "a-b");
        assert_eq!(slugify("?!leading"), "leading");
        assert_eq!(slugify("trailing??"), "trailing");
        assert_eq!(slugify("keep.dots_and-dashes"), "keep.dots_and-dashes");
        assert_eq!(slugify("naïve-crate"), "na-ve-crate");
        assert_eq!(slugify("日本語"), "");
        assert_eq!(file_slug("日本語"), "untitled");
        assert_eq!(yaml_scalar("plain"), "'plain'");
        assert_eq!(yaml_scalar("it's"), "'it''s'");
        assert_eq!(yaml_scalar("ops\ncore"), "\"ops\\ncore\"");
        assert_eq!(yaml_scalar("esc\u{1b}[31m"), "\"esc\\x1b[31m\"");
        assert_eq!(yaml_scalar("tab\tsep\r"), "\"tab\\tsep\\r\"");
    }
}
