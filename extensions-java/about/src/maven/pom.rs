//! Line-based `pom.xml` parser for the Maven `project_identity` provider.
//!
//! ## Known limits
//!
//! This is a line-oriented extractor, not a real XML parser. It supports the
//! standard, prettily-formatted Maven POM shape and intentionally avoids the
//! complexity (and dependency cost) of `quick-xml`. Specifically:
//!
//! - **XML comments are stripped.** Both single-line
//!   `<!-- … -->` blocks and multi-line `<!-- …\n…\n -->` blocks are
//!   removed before tag matching, so a commented-out `<artifactId>fake</artifactId>`
//!   does not get captured as the project artifact id.
//! - **No CDATA handling.** `<![CDATA[ ... ]]>` blocks are not unwrapped.
//! - **One element per line for top-level scalars.** Open and close tags must
//!   be on the same line for fields like `<artifactId>` (multi-line element
//!   values are not supported). Single-line `<scm>...</scm>` and
//!   `<licenses>...</licenses>` blocks are special-cased. Elements glued to
//!   the `<project>` opener's `>` (including a whole compact one-line POM)
//!   are split into one element per piece and parsed like the
//!   pretty-printed form, so a container on that line is tracked as a
//!   section and its children never fill project scalars.
//! - **No nested duplicate elements.** Inside a section like `<scm>` the
//!   first matching child wins; deeper nesting (e.g. nested `<url>` inside
//!   another tag) is not tracked.
//! - **Only direct children of `<project>` are project scalars.** The
//!   containers in `SKIP_SECTIONS` (`<parent>`, `<dependencies>`, `<build>`,
//!   `<profiles>`, …) are skipped wholesale, so a `<version>`, `<url>`,
//!   `<name>`, `<description>` or `<modules>` nested inside one never fills a
//!   project field. A container not listed there is not skipped. A project
//!   that inherits its `<version>` from `<parent>` reports no version.
//! - **No attribute-bearing tag matching.** Elements with attributes
//!   (`<artifactId xml:lang="en">…</artifactId>`) or namespace prefixes are
//!   not recognised; the canonical bare-tag form is required.
//!
//! Replacing this with `quick-xml` is a single-module swap; callers depend
//! only on `parse_pom_xml` and `PomData`.

use std::path::Path;

#[derive(Default)]
#[non_exhaustive]
pub(super) struct PomData {
    /// Maven `<artifactId>` — coordinate, first-write-wins on duplicates.
    /// [`try_set_once`] is the single owner of that policy for the scalar
    /// `Option` fields below. The `Vec` fields (`modules`, `developers`) are
    /// collections, not single-valued: they append every entry they see.
    pub(super) artifact_id: Option<String>,
    /// Maven `<name>` — display name, first-write-wins on duplicates (see
    /// [`try_set_once`], which owns duplicate resolution for the scalar
    /// fields). Provider prefers this over `artifact_id` when both are
    /// present.
    pub(super) name: Option<String>,
    pub(super) version: Option<String>,
    pub(super) description: Option<String>,
    pub(super) license: Option<String>,
    pub(super) modules: Vec<String>,
    pub(super) developers: Vec<String>,
    /// `<scm><url>` — the source-control repository URL. The provider maps
    /// this to `ParsedManifest::repository`.
    pub(super) scm_url: Option<String>,
    /// The **top-level** `<url>` — in the Maven POM schema the *project
    /// homepage*, a distinct element from `<scm><url>`. The provider maps this
    /// to `ParsedManifest::homepage`, and it is held in its own field so
    /// neither URL can clobber the other whatever order they appear in.
    pub(super) project_url: Option<String>,
}

/// Tracks which POM section we're currently inside.
#[derive(PartialEq)]
enum PomSection {
    TopLevel,
    Modules,
    Developers {
        in_developer: bool,
    },
    Scm,
    Licenses {
        in_license: bool,
    },
    /// Container section we deliberately ignore (every entry of
    /// [`SKIP_SECTIONS`]). Tracks the closing tag we're waiting for so a
    /// `<url>`, `<version>`, `<name>` or `<modules>` inside it is not
    /// captured as a project-level field.
    Skip {
        close: &'static str,
    },
}

impl PomSection {
    /// Construct name for the end-of-input diagnostic, or `None` at top level
    /// (where nothing can be unterminated).
    fn unterminated_name(&self) -> Option<&'static str> {
        match self {
            Self::TopLevel => None,
            Self::Modules => Some("modules"),
            Self::Developers { .. } => Some("developers"),
            Self::Scm => Some("scm"),
            Self::Licenses { .. } => Some("licenses"),
            // `close` is `"</parent>"`-shaped; the construct name is the
            // bare tag between the angle brackets.
            Self::Skip { close } => Some(close.trim_matches(['<', '>', '/'])),
        }
    }
}

/// Outcome of matching a top-level line against the section openers.
///
/// The three states are distinct variants so the dispatcher can tell "not a
/// section opener, fall through to `parse_top_level`" from "single-line
/// container, already consumed — do **not** fall through". Collapsing the
/// latter two would let a single-line `<parent>` / `<organization>` /
/// `<licenses>` block leak its children into the top-level fields.
enum SectionOutcome {
    /// A multi-line section opened; the parser moves into it.
    Entered(PomSection),
    /// A single-line container was handled in place. The line must not be
    /// re-parsed at top level.
    Consumed,
    /// Not a section opener at all — the caller parses it at top level.
    NotASection,
}

/// Top-level container sections to skip wholesale: their inner `<url>`,
/// `<name>`, `<version>`, `<description>` and `<modules>` belong to the
/// container (a dependency, plugin, repository, profile, …), not to the
/// project, and must not be captured at top level.
///
/// None of these containers nests an element of its own name in the POM
/// schema, so waiting for the bare closing tag is enough to find the end.
const SKIP_SECTIONS: &[(&str, &str)] = &[
    ("<organization>", "</organization>"),
    ("<parent>", "</parent>"),
    ("<issueManagement>", "</issueManagement>"),
    ("<ciManagement>", "</ciManagement>"),
    ("<distributionManagement>", "</distributionManagement>"),
    ("<dependencies>", "</dependencies>"),
    ("<dependencyManagement>", "</dependencyManagement>"),
    ("<build>", "</build>"),
    ("<reporting>", "</reporting>"),
    ("<repositories>", "</repositories>"),
    ("<pluginRepositories>", "</pluginRepositories>"),
    ("<profiles>", "</profiles>"),
    ("<properties>", "</properties>"),
    ("<contributors>", "</contributors>"),
    ("<mailingLists>", "</mailingLists>"),
    ("<prerequisites>", "</prerequisites>"),
];

/// Parse `<project_root>/pom.xml`, contained within `project_root`.
pub(super) fn parse_pom_xml(project_root: &Path) -> Option<PomData> {
    parse_pom_xml_within(project_root, project_root)
}

/// Parse `<dir>/pom.xml`, reading it only if it resolves inside
/// `workspace_root`.
///
/// A child module's pom is contained against the workspace root, not its own
/// directory: a module directory that is a symlink out of the workspace
/// canonicalizes outside the root and is not read. The read goes through the
/// shared `manifest_io` helper, so the NotFound-vs-other-IO classification
/// matches the sibling parsers (`go_mod`, `go_work`, `package_json`,
/// `pyproject`).
pub(super) fn parse_pom_xml_within(dir: &Path, workspace_root: &Path) -> Option<PomData> {
    let path = dir.join("pom.xml");
    let content = ops_about::manifest_io::read_optional_text(&path, workspace_root, "pom.xml")?;
    Some(parse_pom_content(&content))
}

fn parse_pom_content(content: &str) -> PomData {
    let mut data = PomData::default();
    let mut started = false;
    let mut opener_pending = false;
    let mut section = PomSection::TopLevel;
    // Track whether we're inside a multi-line `<!-- … -->`
    // block. Lines (or partial lines) inside the block are stripped before
    // any tag matching so a commented-out `<artifactId>` cannot be captured.
    let mut in_comment = false;

    for raw_line in content.lines() {
        let cleaned = strip_xml_comments(raw_line, &mut in_comment);
        let line = cleaned.trim();
        if line.is_empty() {
            continue;
        }

        if !started {
            if let Some(remainder) = accept_project_opener(line, &mut started, &mut opener_pending)
                && dispatch_opener_remainder(remainder, &mut section, &mut data)
            {
                break;
            }
            continue;
        }
        if dispatch_started_line(line, &mut section, &mut data) {
            break;
        }
    }

    warn_unterminated(in_comment, opener_pending, &section);

    data
}

/// Advance the `<project` opener state machine by one not-yet-started line.
///
/// Supports multi-line `<project ... >` openers, which real-world Maven
/// formatters often emit (xmlns/xsi attributes split across lines): an
/// "opener pending" state is tracked until the closing `>` arrives. Returns
/// `Some(remainder)` once the document has started, where `remainder` is the
/// trimmed text after the opener's closing `>` on this same line (possibly
/// empty). Every opener shape (bare `<project>`, single-line with attributes,
/// and the closing line of a multi-line opener) hands back its remainder, so
/// an element glued to the `>` is dispatched instead of dropped. Returns
/// `None` while the document has not started.
fn accept_project_opener<'a>(
    line: &'a str,
    started: &mut bool,
    opener_pending: &mut bool,
) -> Option<&'a str> {
    if *opener_pending {
        let (_, after_gt) = line.split_once('>')?;
        *opener_pending = false;
        *started = true;
        return Some(after_gt.trim());
    }
    match classify_project_opener(line) {
        ProjectOpener::Complete(remainder) => {
            *started = true;
            Some(remainder)
        }
        ProjectOpener::Pending => {
            *opener_pending = true;
            None
        }
        ProjectOpener::NotOpener => None,
    }
}

/// Dispatch the text left on the opener's line after its closing `>`.
///
/// A compact POM puts the whole document on one line
/// (`<project><parent><version>9</version></parent><artifactId>x</artifactId></project>`).
/// The remainder is split by [`split_glued_elements`] into the pieces a
/// pretty-printed POM would put on separate lines, and each piece is fed to
/// the same dispatcher as a started line. Container openers and closers
/// therefore drive the section state machine exactly as they do in the
/// multi-line form, so a container's children never fill project scalars.
/// Returns `true` when the document ended on this line and the outer loop
/// should break.
fn dispatch_opener_remainder(
    remainder: &str,
    section: &mut PomSection,
    data: &mut PomData,
) -> bool {
    split_glued_elements(remainder).any(|piece| dispatch_started_line(piece, section, data))
}

/// Split a run of glued XML elements into the pieces a pretty-printer would
/// put one per line: a leaf element with its text (`<version>9</version>`),
/// a bare opening tag of a container (`<parent>`), or a closing tag
/// (`</parent>`). Whitespace between pieces is dropped, and text outside any
/// tag is kept as its own piece so it can never be mistaken for a tag.
fn split_glued_elements(line: &str) -> impl Iterator<Item = &str> {
    let mut rest = line.trim_start();
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let end = glued_piece_end(rest);
        let (piece, tail) = rest.split_at(end);
        rest = tail.trim_start();
        Some(piece.trim_end())
    })
}

/// Byte length of the first piece of `rest` (non-empty, no leading
/// whitespace); see [`split_glued_elements`] for the piece shapes.
fn glued_piece_end(rest: &str) -> usize {
    if !rest.starts_with('<') {
        return rest.find('<').unwrap_or(rest.len());
    }
    let Some(gt) = rest.find('>') else {
        return rest.len();
    };
    let tag_end = gt.saturating_add(1);
    let tag = rest.get(..tag_end).unwrap_or(rest);
    if tag.starts_with("</") || tag.ends_with("/>") {
        return tag_end;
    }
    let after = rest.get(tag_end..).unwrap_or("");
    // A leaf: text (possibly empty) followed directly by a closing tag.
    match after.find('<') {
        Some(lt) if after.get(lt..).is_some_and(|t| t.starts_with("</")) => {
            after.get(lt..).and_then(|t| t.find('>')).map_or(rest.len(), |close_gt| {
                tag_end.saturating_add(lt).saturating_add(close_gt).saturating_add(1)
            })
        }
        _ => tag_end,
    }
}

/// Report every construct still open at end of input.
///
/// A truncated or hand-mangled pom.xml must not degrade to "empty POM" with
/// no diagnostic, which is the same failure mode the crate-level
/// malformed-vs-missing policy exists to prevent. The fields parsed before
/// the construct are kept, and everything after it is explicitly dropped
/// rather than silently swallowed.
fn warn_unterminated(in_comment: bool, opener_pending: bool, section: &PomSection) {
    if in_comment {
        tracing::warn!(
            manifest = "pom.xml",
            construct = "comment",
            "pom.xml: unterminated XML comment at end of file; every line after the `<!--` was dropped"
        );
    }
    if opener_pending {
        tracing::warn!(
            manifest = "pom.xml",
            construct = "project",
            "pom.xml: unterminated <project ...> opener at end of file; no fields were parsed"
        );
    }
    if let Some(section_name) = section.unterminated_name() {
        tracing::warn!(
            manifest = "pom.xml",
            construct = section_name,
            "pom.xml: unterminated section at end of file; top-level fields after it were dropped"
        );
    }
}

/// How a not-yet-started line relates to the `<project>` opener.
#[derive(Debug, PartialEq)]
enum ProjectOpener<'a> {
    /// The whole opener is on this line (bare `<project>` or one carrying
    /// attributes). Holds the trimmed text after its closing `>`, which may
    /// be empty or carry elements glued to the opener.
    Complete(&'a str),
    /// `<project` followed by attributes with no closing `>` on this line:
    /// the opener continues on a later line.
    Pending,
    /// Not a `<project>` opener at all (e.g. `<projectInfo>` or `<project/>`).
    NotOpener,
}

/// Classify a not-yet-started line against the `<project>` opener.
///
/// The tag name must end right after `project`: either the opener closes
/// immediately (`<project>…`) or whitespace introduces attributes. That
/// rejects unrelated tags whose name merely starts with `project`
/// (`<projectInfo>`).
fn classify_project_opener(line: &str) -> ProjectOpener<'_> {
    let Some(rest) = line.strip_prefix("<project") else {
        return ProjectOpener::NotOpener;
    };
    if let Some(after_gt) = rest.strip_prefix('>') {
        return ProjectOpener::Complete(after_gt.trim());
    }
    if !rest.starts_with(char::is_whitespace) {
        return ProjectOpener::NotOpener;
    }
    match rest.split_once('>') {
        Some((_, after_gt)) => ProjectOpener::Complete(after_gt.trim()),
        None => ProjectOpener::Pending,
    }
}

/// Process a single trimmed line in "started" state
/// (i.e. inside `<project>`). Returns `true` when `</project>` was seen and
/// the outer loop should break. Extracted so the multi-line opener path can
/// re-feed any post-`>` remainder through the same dispatch.
fn dispatch_started_line(line: &str, section: &mut PomSection, data: &mut PomData) -> bool {
    if line == "</project>" {
        return true;
    }
    if matches!(section, PomSection::TopLevel) {
        match match_section_open(line, data) {
            SectionOutcome::Entered(new_section) => *section = new_section,
            SectionOutcome::Consumed => {}
            SectionOutcome::NotASection => parse_top_level(line, data),
        }
        return false;
    }
    if handle_section_line(section, line, data) {
        *section = PomSection::TopLevel;
    }
    false
}

/// Dispatch a line to the active section's handler. Returns `true` when the
/// section's closing tag was seen and the parser should return to `TopLevel`.
fn handle_section_line(section: &mut PomSection, line: &str, data: &mut PomData) -> bool {
    match section {
        PomSection::Modules => handle_modules(line, data),
        PomSection::Developers { in_developer } => handle_developers(line, in_developer, data),
        PomSection::Scm => handle_scm(line, data),
        PomSection::Licenses { in_license } => handle_licenses(line, in_license, data),
        PomSection::Skip { close } => line == *close,
        // `dispatch_started_line` returns before calling this for
        // `TopLevel`; `false` ("no closing tag on this line") keeps the
        // dispatcher total instead of panicking if that guard ever moves.
        PomSection::TopLevel => false,
    }
}

fn handle_modules(line: &str, data: &mut PomData) -> bool {
    if line == "</modules>" {
        return true;
    }
    if let Some(val) = extract_xml_value(line, "<module>", "</module>") {
        data.modules.push(val.to_string());
    }
    false
}

fn handle_developers(line: &str, in_developer: &mut bool, data: &mut PomData) -> bool {
    match line {
        "</developers>" => return true,
        "<developer>" => *in_developer = true,
        "</developer>" => *in_developer = false,
        _ => {
            if *in_developer && let Some(val) = extract_xml_value(line, "<name>", "</name>") {
                data.developers.push(val.to_string());
            }
        }
    }
    false
}

/// Walk every `<developer>` entry in a collapsed single-line
/// `<developers>...</developers>` container, collecting each entry's
/// `<name>` into `data.developers`.
///
/// Mirrors the multi-line [`handle_developers`] policy: every `<name>`
/// inside a `<developer>` is kept, and text outside an entry contributes
/// nothing. Scanning stops at the first malformed (unclosed) entry.
fn extract_collapsed_developers(line: &str, data: &mut PomData) {
    let mut rest = line;
    while let Some(start) = rest.find("<developer>") {
        let Some(after) = rest.get(start.saturating_add("<developer>".len())..) else {
            break;
        };
        let Some(end) = after.find("</developer>") else {
            break;
        };
        if let Some(entry) = after.get(..end)
            && let Some(val) = extract_xml_value(entry, "<name>", "</name>")
        {
            data.developers.push(val.to_string());
        }
        let Some(next) = after.get(end.saturating_add("</developer>".len())..) else {
            break;
        };
        rest = next;
    }
}

fn handle_scm(line: &str, data: &mut PomData) -> bool {
    if line == "</scm>" {
        return true;
    }
    try_set_once(&mut data.scm_url, line, "<url>", "</url>");
    false
}

/// Write `field` from a `<tag>value</tag>` line iff the field is still empty.
///
/// This is the single home of the "first writer wins on duplicates" rule, so a
/// refactor cannot change duplicate resolution one call site at a time. The
/// top-level `<url>` and `<scm><url>` write *distinct* fields (`project_url`
/// vs `scm_url`), so neither can clobber the other whatever the source order.
fn try_set_once(field: &mut Option<String>, line: &str, open: &str, close: &str) {
    if field.is_none()
        && let Some(val) = extract_xml_value(line, open, close)
    {
        *field = Some(val.to_string());
    }
}

fn handle_licenses(line: &str, in_license: &mut bool, data: &mut PomData) -> bool {
    match line {
        "</licenses>" => return true,
        "<license>" => *in_license = true,
        "</license>" => *in_license = false,
        _ => {
            if *in_license {
                try_set_once(&mut data.license, line, "<name>", "</name>");
            }
        }
    }
    false
}

/// Match opening tags for POM sections. Single-line `<scm>...</scm>`,
/// `<licenses>...</licenses>` and `<developers>...</developers>` blocks are
/// extracted in place and reported as [`SectionOutcome::Consumed`], leaving
/// the caller in `TopLevel` **without** re-parsing the line at top level.
///
/// Consuming the collapsed `<developers>` form is load-bearing, not tidiness:
/// falling through would hand a line containing `<name>` to `parse_top_level`.
fn match_section_open(line: &str, data: &mut PomData) -> SectionOutcome {
    if line == "<modules>" {
        return SectionOutcome::Entered(PomSection::Modules);
    }
    if line == "<developers>" {
        return SectionOutcome::Entered(PomSection::Developers { in_developer: false });
    }
    if line == "<scm>" {
        return SectionOutcome::Entered(PomSection::Scm);
    }
    if line == "<licenses>" {
        return SectionOutcome::Entered(PomSection::Licenses { in_license: false });
    }

    // Single-line forms: `<scm><url>...</url></scm>` or
    // `<licenses><license><name>...</name></license></licenses>`.
    // Reject malformed inputs with duplicated openers (e.g. `<scm>...<scm>`)
    // to keep the partial-input handler honest.
    if line.starts_with("<scm>") && line.ends_with("</scm>") && line.matches("<scm>").count() == 1 {
        try_set_once(&mut data.scm_url, line, "<url>", "</url>");
        return SectionOutcome::Consumed;
    }
    // A single-line `<licenses>...</licenses>` may carry
    // multiple `<license>` children. Unlike the `<scm>` shortcut above (which
    // rejects pathological lines with duplicate `<scm>` openers), this branch
    // intentionally accepts the multi-license shape and keeps the **first**
    // `<name>` it finds — matching the multi-line `handle_licenses` policy
    // ("first license wins"). The asymmetry with `<scm>` is deliberate: SCM
    // is a single-valued element, while `<licenses>` is a list container.
    if line.starts_with("<licenses>")
        && line.ends_with("</licenses>")
        && line.matches("<licenses>").count() == 1
    {
        try_set_once(&mut data.license, line, "<name>", "</name>");
        return SectionOutcome::Consumed;
    }

    // A single-line `<developers>...</developers>` is handled here rather
    // than falling through to `parse_top_level`, whose `<name>` rule would
    // capture the developer's name as the *project* name (the provider
    // prefers `name` over `artifact_id`, so the project would display as a
    // person). The collapsed form keeps every `<name>` inside a `<developer>`,
    // mirroring the multi-line `handle_developers` policy.
    if line.starts_with("<developers>")
        && line.ends_with("</developers>")
        && line.matches("<developers>").count() == 1
    {
        extract_collapsed_developers(line, data);
        return SectionOutcome::Consumed;
    }

    for (open, close) in SKIP_SECTIONS {
        if line == *open {
            return SectionOutcome::Entered(PomSection::Skip { close });
        }
        // Single-line container — ignore entirely.
        if line.starts_with(*open) && line.ends_with(*close) {
            return SectionOutcome::Consumed;
        }
    }

    SectionOutcome::NotASection
}

/// Parse top-level simple elements (artifactId, version, description, name, url).
fn parse_top_level(line: &str, data: &mut PomData) {
    try_set_once(&mut data.artifact_id, line, "<artifactId>", "</artifactId>");
    try_set_once(&mut data.version, line, "<version>", "</version>");
    try_set_once(&mut data.description, line, "<description>", "</description>");
    try_set_once(&mut data.name, line, "<name>", "</name>");
    try_set_once(&mut data.project_url, line, "<url>", "</url>");
}

/// Strip XML comments from `line`, multi-line aware.
///
/// `in_comment` carries the open-comment state across lines. The returned
/// String is `line` with every `<!-- … -->` region removed (replaced by a
/// single space so adjacent tokens don't get glued together). Per-line
/// processing handles all four cases:
///
/// - already-inside, no `-->` here       → discard the whole line
/// - already-inside, `-->` on this line  → discard up to and including
///   `-->`, then continue scanning the rest for further comments
/// - not inside, `<!--` opens but no `-->` on this line → keep prefix,
///   set `in_comment = true`, discard suffix from `<!--`
/// - not inside, complete `<!-- … -->`   → splice the comment out
fn strip_xml_comments(line: &str, in_comment: &mut bool) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    loop {
        if *in_comment {
            match rest.split_once("-->") {
                Some((_, after)) => {
                    rest = after;
                    *in_comment = false;
                }
                None => return out,
            }
        }
        if let Some((before, after)) = rest.split_once("<!--") {
            out.push_str(before);
            // Insert a separator so e.g. `foo<!--x-->bar` doesn't
            // collapse to `foobar` if a future caller relies on
            // word-boundary parsing. Tag matching uses literal
            // `<tag>` patterns so a stray space is harmless.
            out.push(' ');
            rest = after;
            *in_comment = true;
        } else {
            out.push_str(rest);
            return out;
        }
    }
}

/// Extract value from `<tag>value</tag>` on a single line. Open/close
/// markers are passed pre-built to avoid per-line allocation.
///
/// The XML predefined entity references (`&amp;` `&lt;` `&gt;` `&quot;`
/// `&apos;`) plus numeric `&#NNN;` / `&#xHH;` are decoded, so `Foo &amp; Bar`
/// renders as `Foo & Bar` in the About card. Returns `Cow::Borrowed` (no
/// allocation) for entity-free values, `Cow::Owned` only when a substitution
/// was needed.
fn extract_xml_value<'a>(
    line: &'a str,
    open: &str,
    close: &str,
) -> Option<std::borrow::Cow<'a, str>> {
    let start = line.find(open)?;
    let end = line.find(close)?;
    // `start` is the offset of a match of `open` inside `line`, so
    // `start + open.len()` is at most `line.len()`.
    let val_start = start.saturating_add(open.len());
    if val_start < end {
        // `start`/`end` come from `find`, and `open` matched at `start`, so
        // `val_start` is a char boundary too — `get` never returns `None`
        // here; treating it as "no value" is the safe degradation.
        line.get(val_start..end).map(|val| decode_xml_entities(val.trim()))
    } else {
        None
    }
}

/// Minimal XML-1.0 predefined-entity and numeric-character-reference decoder.
/// Borrows the input when no `&` is present (the common case).
fn decode_xml_entities(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains('&') {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some((before, after_amp)) = rest.split_once('&') {
        out.push_str(before);
        // The `len() <= 8` guard bounds how far ahead a `;` may sit: the
        // entity name is everything up to the `;`, and no recognised entity
        // name is longer than that.
        let Some((entity, tail)) = after_amp.split_once(';').filter(|(e, _)| e.len() <= 8) else {
            // Stray `&` or runaway entity: leave verbatim and continue.
            out.push('&');
            rest = after_amp;
            continue;
        };
        let decoded: Option<char> = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            other if other.starts_with("#x") || other.starts_with("#X") => other
                .get(2..)
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .and_then(char::from_u32),
            other if other.starts_with('#') => {
                other.get(1..).and_then(|dec| dec.parse::<u32>().ok()).and_then(char::from_u32)
            }
            _ => None,
        };
        if let Some(c) = decoded {
            out.push(c);
        } else {
            // Unknown entity: keep verbatim (`&entity;`).
            out.push('&');
            out.push_str(entity);
            out.push(';');
        }
        rest = tail;
    }
    out.push_str(rest);
    std::borrow::Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_xml_value_basic() {
        assert_eq!(
            extract_xml_value("<artifactId>camel</artifactId>", "<artifactId>", "</artifactId>")
                .as_deref(),
            Some("camel")
        );
    }

    #[test]
    fn extract_xml_value_with_whitespace() {
        assert_eq!(
            extract_xml_value("    <version>1.0</version>  ", "<version>", "</version>").as_deref(),
            Some("1.0")
        );
    }

    /// Standard XML predefined entities and numeric references are decoded so
    /// the rendered About card shows the
    /// human-readable text rather than the raw `&amp;` / `&#39;` source.
    #[test]
    fn extract_xml_value_decodes_predefined_entities() {
        assert_eq!(
            extract_xml_value(
                "<description>Foo &amp; Bar &lt;v2&gt; &quot;ok&quot;</description>",
                "<description>",
                "</description>"
            )
            .as_deref(),
            Some(r#"Foo & Bar <v2> "ok""#)
        );
        assert_eq!(
            extract_xml_value(
                "<description>It&apos;s &#39;a&#39; test &#x26;</description>",
                "<description>",
                "</description>"
            )
            .as_deref(),
            Some("It's 'a' test &")
        );
    }

    /// An unknown entity is left verbatim, so content the decoder does not
    /// understand is never silently corrupted.
    #[test]
    fn extract_xml_value_passes_through_unknown_entities() {
        assert_eq!(
            extract_xml_value(
                "<description>weird &custom; thing</description>",
                "<description>",
                "</description>"
            )
            .as_deref(),
            Some("weird &custom; thing")
        );
    }

    #[test]
    fn extract_xml_value_no_match() {
        assert_eq!(extract_xml_value("<name>foo</name>", "<version>", "</version>"), None);
    }

    #[test]
    fn parse_pom_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <artifactId>myapp</artifactId>
    <version>2.0.0</version>
    <name>My App</name>
    <description>A cool app</description>
    <modules>
        <module>core</module>
        <module>web</module>
    </modules>
    <developers>
        <developer>
            <name>Alice</name>
        </developer>
    </developers>
    <scm>
        <url>https://github.com/user/myapp</url>
    </scm>
    <licenses>
        <license>
            <name>Apache-2.0</name>
        </license>
    </licenses>
</project>"#,
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.artifact_id, Some("myapp".to_string()));
        assert_eq!(pom.name, Some("My App".to_string()));
        assert_eq!(pom.version, Some("2.0.0".to_string()));
        assert_eq!(pom.description, Some("A cool app".to_string()));
        assert_eq!(pom.modules, vec!["core", "web"]);
        assert_eq!(pom.developers, vec!["Alice"]);
        assert_eq!(pom.scm_url, Some("https://github.com/user/myapp".to_string()));
        assert_eq!(pom.license, Some("Apache-2.0".to_string()));
    }

    #[test]
    fn parse_pom_single_line_licenses_with_multiple_children_keeps_first() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n<licenses><license><name>Apache-2.0</name></license><license><name>MIT</name></license></licenses>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.license, Some("Apache-2.0".to_string()));
    }

    #[test]
    fn parse_pom_artifact_id_without_name_kept() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <artifactId>foo</artifactId>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.artifact_id, Some("foo".to_string()));
        assert!(pom.name.is_none());
    }

    #[test]
    fn parse_pom_minimal() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <artifactId>simple</artifactId>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.artifact_id, Some("simple".to_string()));
        assert!(pom.version.is_none());
        assert!(pom.modules.is_empty());
    }

    #[test]
    fn parse_pom_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(parse_pom_xml(dir.path()).is_none());
    }

    /// A top-level `<url>` is the project homepage, not the SCM
    /// repository. It must land in `project_url` and leave `scm_url` empty
    /// for the provider's git-remote repository fallback.
    #[test]
    fn parse_pom_top_level_url_is_the_project_homepage() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <artifactId>mylib</artifactId>\n    <url>https://example.com</url>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.project_url, Some("https://example.com".to_string()));
        assert_eq!(pom.scm_url, None);
    }

    /// `<url>` and `<scm><url>` are distinct POM elements and must
    /// be captured independently, in either source order — the homepage from
    /// the top-level `<url>`, the repository from `<scm><url>`.
    #[test]
    fn parse_pom_url_and_scm_url_are_captured_independently_in_either_order() {
        for (label, pom_xml) in [
            (
                "scm first, url second",
                r"<project>
    <artifactId>mylib</artifactId>
    <scm>
        <url>https://github.com/user/mylib</url>
    </scm>
    <url>https://example.com</url>
</project>",
            ),
            (
                "url first, scm second",
                r"<project>
    <artifactId>mylib</artifactId>
    <url>https://example.com</url>
    <scm>
        <url>https://github.com/user/mylib</url>
    </scm>
</project>",
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("pom.xml"), pom_xml).unwrap();

            let pom = parse_pom_xml(dir.path()).unwrap();
            assert_eq!(
                pom.project_url,
                Some("https://example.com".to_string()),
                "homepage source must come from the top-level <url> ({label})"
            );
            assert_eq!(
                pom.scm_url,
                Some("https://github.com/user/mylib".to_string()),
                "repository source must come from <scm><url> ({label})"
            );
        }
    }

    #[test]
    fn parse_pom_multiple_developers() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r"<project>
    <artifactId>multi</artifactId>
    <developers>
        <developer>
            <name>Alice</name>
        </developer>
        <developer>
            <name>Bob</name>
        </developer>
    </developers>
</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.developers, vec!["Alice", "Bob"]);
    }

    #[test]
    fn parse_pom_organization_url_not_captured_as_scm() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r"<project>
    <organization>
        <name>Acme</name>
        <url>https://acme.example</url>
    </organization>
    <scm>
        <url>https://github.com/user/myapp</url>
    </scm>
</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.scm_url, Some("https://github.com/user/myapp".to_string()));
    }

    #[test]
    fn parse_pom_single_line_scm() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <scm><url>https://example.com</url></scm>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.scm_url, Some("https://example.com".to_string()));
        // The consumed container line must not be re-parsed at top level:
        // `SectionOutcome::Consumed` stops the fall-through to
        // `parse_top_level`.
        assert_eq!(pom.name, None);
    }

    /// A single-line `<parent>` block — the standard
    /// shape in Maven multi-module children — must not leak the parent's
    /// coordinates into the child's own `artifactId` / `version`.
    #[test]
    fn parse_pom_single_line_parent_does_not_leak_coordinates() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r"<project>
    <parent><artifactId>parent-pom</artifactId><version>9.9.9</version></parent>
    <artifactId>child</artifactId>
    <version>1.0.0</version>
</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.artifact_id, Some("child".to_string()));
        assert_eq!(pom.version, Some("1.0.0".to_string()));
    }

    /// A single-line `<organization>` block must not
    /// contribute its `<name>` / `<url>` to the project name or SCM URL.
    #[test]
    fn parse_pom_single_line_organization_ignored_entirely() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r"<project>
    <organization><name>Acme</name><url>https://acme.example</url></organization>
    <artifactId>real</artifactId>
</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.name, None);
        assert_eq!(pom.scm_url, None);
        assert_eq!(pom.artifact_id, Some("real".to_string()));
    }

    #[test]
    fn parse_pom_single_line_licenses() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <licenses><license><name>MIT</name></license></licenses>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.license, Some("MIT".to_string()));
        // The license `<name>` must not double as the
        // project name via a top-level re-parse of the consumed line.
        assert_eq!(pom.name, None);
    }

    #[test]
    fn parse_pom_stray_name_in_licenses_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r"<project>
    <licenses>
        <name>stray</name>
        <license>
            <name>Apache-2.0</name>
        </license>
    </licenses>
</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.license, Some("Apache-2.0".to_string()));
    }

    #[test]
    fn parse_pom_leading_project_info_does_not_open() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<projectInfo>noise</projectInfo>\n<project>\n    <artifactId>real</artifactId>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.artifact_id, Some("real".to_string()));
    }

    #[test]
    fn parse_pom_duplicate_scm_opener_deterministic() {
        // Two `<scm>` openers on one line is malformed, and the single-line
        // scm detector rejects that shape rather than extracting a URL from a
        // line it cannot prove is one scm element. The line still carries a
        // `<url>` the top-level scanner reads into `project_url`, so the
        // outcome stays deterministic: the first URL becomes the homepage
        // source and `scm_url` stays empty.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <scm><url>https://first.example</url></scm><scm><url>https://second.example</url></scm>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.project_url, Some("https://first.example".to_string()));
        assert_eq!(pom.scm_url, None);
    }

    #[test]
    fn parse_pom_multiline_project_opener() {
        // Real-world formatters often split xmlns/xsi attributes across
        // lines, so the parser treats the opener as continuing until the
        // first `>` and resumes normal scanning afterwards.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
    <artifactId>multiline</artifactId>
</project>"#,
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.artifact_id, Some("multiline".to_string()));
    }

    /// A multi-line `<project ...>` opener whose
    /// closing `>` is followed *on the same line* by a real element must
    /// not drop that trailing element. Real-world Maven formatters emit
    /// this shape when the xmlns block is wrapped but the next tag is
    /// glued to the closing `>`.
    #[test]
    fn parse_pom_multiline_project_opener_trailing_element_on_same_line() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project xmlns=\"http://maven.apache.org/POM/4.0.0\"\n         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n         xsi:schemaLocation=\"http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd\"><artifactId>x</artifactId>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.artifact_id, Some("x".to_string()));
    }

    /// A single-line opener carrying attributes, with an element glued to its
    /// closing `>`, dispatches that element like the multi-line opener does.
    #[test]
    fn parse_pom_single_line_attributed_opener_keeps_glued_element() {
        let pom = parse_pom_str(
            "<project xmlns=\"http://maven.apache.org/POM/4.0.0\"><artifactId>x</artifactId>\n<version>2.0</version>\n</project>",
        );
        assert_eq!(pom.artifact_id.as_deref(), Some("x"));
        assert_eq!(pom.version.as_deref(), Some("2.0"));
    }

    /// A bare `<project>` with an element glued to it dispatches that element.
    #[test]
    fn parse_pom_bare_opener_keeps_glued_element() {
        let pom =
            parse_pom_str("<project><artifactId>x</artifactId>\n<name>Glued</name>\n</project>");
        assert_eq!(pom.artifact_id.as_deref(), Some("x"));
        assert_eq!(pom.name.as_deref(), Some("Glued"));
    }

    /// A compact POM with the whole document on one line yields its scalars.
    #[test]
    fn parse_pom_compact_one_line_pom_yields_scalars() {
        let pom = parse_pom_str(
            "<project><artifactId>x</artifactId><version>1.0</version><description>d</description></project>",
        );
        assert_eq!(pom.artifact_id.as_deref(), Some("x"));
        assert_eq!(pom.version.as_deref(), Some("1.0"));
        assert_eq!(pom.description.as_deref(), Some("d"));
    }

    /// A trailing `</project>` on the opener line ends the document: a stray
    /// element after it is not captured.
    #[test]
    fn parse_pom_compact_one_line_pom_stops_at_project_close() {
        let pom =
            parse_pom_str("<project><artifactId>x</artifactId></project>\n<version>9</version>");
        assert_eq!(pom.artifact_id.as_deref(), Some("x"));
        assert_eq!(pom.version, None);
    }

    /// A `<parent>` glued to a compact opener keeps its `<version>` out of
    /// the project version.
    #[test]
    fn parse_pom_compact_one_line_parent_version_not_captured() {
        let pom = parse_pom_str(
            "<project><parent><groupId>g</groupId><artifactId>p</artifactId><version>9</version></parent><artifactId>x</artifactId></project>",
        );
        assert_eq!(pom.artifact_id.as_deref(), Some("x"));
        assert_eq!(pom.version, None);
    }

    /// An `<scm><url>` glued to a compact opener is the repository, never
    /// the homepage.
    #[test]
    fn parse_pom_compact_one_line_scm_url_is_not_the_homepage() {
        let pom = parse_pom_str(
            "<project><scm><url>https://scm.example</url></scm><artifactId>x</artifactId></project>",
        );
        assert_eq!(pom.scm_url.as_deref(), Some("https://scm.example"));
        assert_eq!(pom.project_url, None);
        assert_eq!(pom.artifact_id.as_deref(), Some("x"));
    }

    /// A developer `<name>` glued to a compact opener is a developer, not
    /// the project name.
    #[test]
    fn parse_pom_compact_one_line_developer_name_not_the_project_name() {
        let pom = parse_pom_str(
            "<project><developers><developer><name>Ada</name></developer></developers><artifactId>x</artifactId></project>",
        );
        assert_eq!(pom.developers, vec!["Ada".to_string()]);
        assert_eq!(pom.name, None);
    }

    /// A skip-section container left open on the opener line keeps
    /// swallowing children on later lines until its closing tag.
    #[test]
    fn parse_pom_container_opened_on_opener_line_spans_lines() {
        let pom = parse_pom_str(
            "<project><dependencies><dependency><version>2</version>\n</dependency>\n</dependencies>\n<version>1</version>\n</project>",
        );
        assert_eq!(pom.version.as_deref(), Some("1"));
    }

    /// The glued-element splitter yields the pretty-printed pieces.
    #[test]
    fn split_glued_elements_yields_pretty_printed_pieces() {
        let pieces: Vec<&str> =
            split_glued_elements("<parent> <version>9</version></parent><x/>txt<a></a></project>")
                .collect();
        assert_eq!(
            pieces,
            [
                "<parent>",
                "<version>9</version>",
                "</parent>",
                "<x/>",
                "txt",
                "<a></a>",
                "</project>"
            ]
        );
        assert_eq!(split_glued_elements("").count(), 0);
        assert_eq!(split_glued_elements("<unterminated").collect::<Vec<_>>(), ["<unterminated"]);
    }

    /// `<project/>` and `<projectInfo>` are not the opener.
    #[test]
    fn classify_project_opener_rejects_lookalike_tags() {
        assert_eq!(classify_project_opener("<project/>"), ProjectOpener::NotOpener);
        assert_eq!(classify_project_opener("<projectInfo>"), ProjectOpener::NotOpener);
        assert_eq!(classify_project_opener("<project"), ProjectOpener::NotOpener);
        assert_eq!(classify_project_opener("<project xmlns=\"a\""), ProjectOpener::Pending);
        assert_eq!(classify_project_opener("<project>"), ProjectOpener::Complete(""));
    }

    /// A `<artifactId>` inside an XML comment must NOT
    /// be captured. The release/SNAPSHOT swap pattern is common in real
    /// repos.
    #[test]
    fn parse_pom_commented_artifact_id_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r"<project>
    <!-- <artifactId>fake-snapshot</artifactId> -->
    <artifactId>real-release</artifactId>
</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.artifact_id, Some("real-release".to_string()));
    }

    /// A multi-line comment block hides every captured
    /// element it contains, including `<scm><url>` blocks.
    #[test]
    fn parse_pom_multiline_comment_hides_inner_tags() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r"<project>
    <!--
    <artifactId>commented-out</artifactId>
    <scm>
        <url>https://example.com/old</url>
    </scm>
    -->
    <artifactId>kept</artifactId>
    <scm>
        <url>https://example.com/new</url>
    </scm>
</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.artifact_id, Some("kept".to_string()));
        assert_eq!(pom.scm_url, Some("https://example.com/new".to_string()));
    }

    /// Helper-level comment-stripping edge cases.
    #[test]
    fn strip_xml_comments_handles_inline_and_multiline() {
        let mut state = false;
        // Single-line comment is spliced out (one separator space inserted).
        assert_eq!(strip_xml_comments("a<!--c-->b", &mut state), "a b");
        assert!(!state);

        // Multi-line: opener leaves state=true and discards trailing.
        let mut state = false;
        let prefix = strip_xml_comments("keep <!-- start", &mut state);
        assert_eq!(prefix.trim(), "keep");
        assert!(state);
        let middle = strip_xml_comments("inside comment", &mut state);
        assert_eq!(middle, "");
        assert!(state);
        let close = strip_xml_comments("more --> tail", &mut state);
        assert_eq!(close.trim(), "tail");
        assert!(!state);
    }

    #[test]
    fn parse_pom_project_with_attributes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project xmlns=\"http://maven.apache.org/POM/4.0.0\">\n    <artifactId>attr</artifactId>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.artifact_id, Some("attr".to_string()));
    }

    /// A `<developers>` container collapsed onto one line must not leak its
    /// developer `<name>` into the project `<name>`. The provider prefers
    /// `name` over `artifact_id`, so a leak here would render the project as
    /// whoever is listed first.
    #[test]
    fn single_line_developers_does_not_become_the_project_name() {
        let mut data = PomData::default();
        let outcome = match_section_open(
            "<developers><developer><name>Jane Doe</name></developer></developers>",
            &mut data,
        );

        assert!(matches!(outcome, SectionOutcome::Consumed));
        assert_eq!(data.name, None, "developer name must not set project name");
        assert_eq!(data.developers, vec!["Jane Doe".to_string()]);
    }

    /// End-to-end through the real parser: the project keeps its own
    /// `artifactId` and stays nameless, and the developer lands in
    /// `developers` rather than in `name`.
    #[test]
    fn parse_pom_single_line_developers_does_not_hijack_the_project_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n<artifactId>camel</artifactId>\n<developers><developer><name>Jane Doe</name></developer></developers>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.artifact_id, Some("camel".to_string()));
        assert!(pom.name.is_none(), "developer name leaked into the project name: {:?}", pom.name);
        assert_eq!(pom.developers, vec!["Jane Doe".to_string()]);
    }

    /// The collapsed form carries every `<developer>`, matching the multi-line
    /// `handle_developers` policy rather than keeping only the first.
    #[test]
    fn single_line_developers_keeps_every_developer() {
        let mut data = PomData::default();
        let outcome = match_section_open(
            "<developers><developer><name>Jane</name></developer>\
<developer><name>Ada</name></developer></developers>",
            &mut data,
        );

        assert!(matches!(outcome, SectionOutcome::Consumed));
        assert_eq!(data.name, None);
        assert_eq!(data.developers, vec!["Jane".to_string(), "Ada".to_string()]);
    }

    /// An unterminated `<!--` comment must be
    /// reported, not silently absorbed as "empty POM". Fields parsed before
    /// the comment survive; the swallowed remainder is explicitly dropped.
    #[test]
    fn pom_with_unterminated_comment_warns_and_keeps_prefix_fields() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n<artifactId>kept</artifactId>\n<!-- swallowed\n<version>1.0</version>\n</project>\n</project>",
        )
        .unwrap();

        let (pom, warn_count) = ops_core::test_utils::count_warnings(|| parse_pom_xml(dir.path()));

        let pom = pom.unwrap();
        assert_eq!(pom.artifact_id, Some("kept".to_string()));
        // The `<version>` after the unterminated `<!--` was dropped, with a
        // diagnostic — not silently parsed and not silently ignored.
        assert_eq!(pom.version, None);
        assert_eq!(warn_count, 1);
    }

    /// A pom.xml that ends inside a multi-line
    /// `<project ...` opener must be reported rather than returning a
    /// silently empty `PomData`.
    #[test]
    fn pom_ending_inside_multiline_project_opener_warns() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project xmlns=\"http://maven.apache.org/POM/4.0.0\"\n         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n",
        )
        .unwrap();

        let (pom, warn_count) = ops_core::test_utils::count_warnings(|| parse_pom_xml(dir.path()));

        let pom = pom.unwrap();
        assert!(pom.artifact_id.is_none());
        assert_eq!(warn_count, 1);
    }

    /// A `<parent>` section that never closes
    /// parks the parser in `Skip` for the rest of the file, so a following
    /// top-level `<artifactId>` is absorbed. The diagnostic reports the
    /// unterminated construct, and the fields after it are explicitly
    /// dropped rather than silently swallowed.
    #[test]
    fn pom_with_unclosed_parent_section_warns_and_reports_dropped_fields() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n<artifactId>before</artifactId>\n<parent>\n<groupId>com.example</groupId>\n<artifactId>child</artifactId>\n<version>1.0</version>\n",
        )
        .unwrap();

        let (pom, warn_count) = ops_core::test_utils::count_warnings(|| parse_pom_xml(dir.path()));

        let pom = pom.unwrap();
        // Parsed before the section opened: kept.
        assert_eq!(pom.artifact_id, Some("before".to_string()));
        // Parked inside `<parent>` at EOF: dropped, with exactly one warn.
        assert_eq!(pom.version, None);
        assert_eq!(warn_count, 1);
    }

    /// The same holds for a tracked section (`<scm>`), and the rendered
    /// diagnostic names both `pom.xml` and the unterminated construct.
    #[test]
    fn pom_with_unclosed_scm_section_warns_naming_manifest_and_construct() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n<artifactId>x</artifactId>\n<scm>\n<url>https://example.com/scm</url>\n<name>swallowed</name>\n",
        )
        .unwrap();

        let rendered = ops_core::test_utils::capture_warn(|| {
            parse_pom_xml(dir.path());
        });

        assert!(rendered.contains("pom.xml"), "warn should name the manifest: {rendered}");
        assert!(
            rendered.contains("scm"),
            "warn should name the unterminated construct: {rendered}"
        );
        assert!(rendered.contains("unterminated"), "warn should say what is wrong: {rendered}");
    }

    fn parse_pom_str(content: &str) -> PomData {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pom.xml"), content).unwrap();
        parse_pom_xml(dir.path()).unwrap()
    }

    /// A child POM that inherits its version from `<parent>` reports no
    /// version: the `<version>` of a dependency, a managed dependency or a
    /// build plugin is not the project's.
    #[test]
    fn parse_pom_inherited_version_is_not_taken_from_dependencies_or_plugins() {
        let pom = parse_pom_str(
            r"<project>
    <parent>
        <groupId>com.example</groupId>
        <artifactId>parent-pom</artifactId>
        <version>9.9.9</version>
    </parent>
    <artifactId>child</artifactId>
    <dependencyManagement>
        <dependencies>
            <dependency>
                <artifactId>managed</artifactId>
                <version>1.1.1</version>
            </dependency>
        </dependencies>
    </dependencyManagement>
    <dependencies>
        <dependency>
            <artifactId>junit</artifactId>
            <version>4.13.2</version>
        </dependency>
    </dependencies>
    <build>
        <plugins>
            <plugin>
                <artifactId>maven-compiler-plugin</artifactId>
                <version>3.11.0</version>
                <dependencies>
                    <dependency>
                        <artifactId>plugin-dep</artifactId>
                        <version>2.2.2</version>
                    </dependency>
                </dependencies>
            </plugin>
        </plugins>
    </build>
</project>",
        );

        assert_eq!(pom.artifact_id, Some("child".to_string()));
        assert_eq!(pom.version, None);
    }

    /// Fields declared after a skipped container are still captured, and the
    /// container's own children do not pre-empt them (first-writer-wins).
    #[test]
    fn parse_pom_fields_after_skipped_containers_are_captured() {
        let pom = parse_pom_str(
            r"<project>
    <dependencies>
        <dependency>
            <artifactId>junit</artifactId>
            <version>4.13.2</version>
        </dependency>
    </dependencies>
    <build>
        <plugins>
            <plugin>
                <artifactId>some-plugin</artifactId>
                <name>Plugin Name</name>
                <description>Plugin description</description>
            </plugin>
        </plugins>
    </build>
    <artifactId>app</artifactId>
    <version>1.0.0</version>
    <name>App</name>
    <description>The app</description>
</project>",
        );

        assert_eq!(pom.artifact_id, Some("app".to_string()));
        assert_eq!(pom.version, Some("1.0.0".to_string()));
        assert_eq!(pom.name, Some("App".to_string()));
        assert_eq!(pom.description, Some("The app".to_string()));
    }

    /// A `<repository><url>` (or plugin repository URL) is where artifacts
    /// are fetched from, never the project homepage.
    #[test]
    fn parse_pom_repository_url_is_not_the_project_homepage() {
        let pom = parse_pom_str(
            r"<project>
    <artifactId>app</artifactId>
    <repositories>
        <repository>
            <id>central</id>
            <name>Central</name>
            <url>https://repo.example/maven2</url>
        </repository>
    </repositories>
    <pluginRepositories>
        <pluginRepository>
            <id>plugins</id>
            <url>https://plugins.example/maven2</url>
        </pluginRepository>
    </pluginRepositories>
</project>",
        );

        assert_eq!(pom.project_url, None);
        assert_eq!(pom.name, None);
    }

    /// `<modules>` inside `<profiles><profile>` only builds when the profile
    /// is active, so it is not counted; neither are the profile's own
    /// dependency versions or repository URLs.
    #[test]
    fn parse_pom_profile_modules_are_not_counted() {
        let pom = parse_pom_str(
            r"<project>
    <artifactId>app</artifactId>
    <modules>
        <module>core</module>
    </modules>
    <profiles>
        <profile>
            <id>extras</id>
            <modules>
                <module>extra-a</module>
                <module>extra-b</module>
            </modules>
            <dependencies>
                <dependency>
                    <artifactId>profile-dep</artifactId>
                    <version>7.7.7</version>
                </dependency>
            </dependencies>
            <repositories>
                <repository>
                    <url>https://profile.example/repo</url>
                </repository>
            </repositories>
            <build>
                <plugins>
                    <plugin>
                        <version>8.8.8</version>
                    </plugin>
                </plugins>
            </build>
        </profile>
    </profiles>
</project>",
        );

        assert_eq!(pom.modules, vec!["core"]);
        assert_eq!(pom.version, None);
        assert_eq!(pom.project_url, None);
    }

    /// `<properties>` holds arbitrary user-named elements, including ones
    /// spelled like project scalars.
    #[test]
    fn parse_pom_properties_do_not_fill_project_fields() {
        let pom = parse_pom_str(
            r"<project>
    <artifactId>app</artifactId>
    <properties>
        <version>${revision}</version>
        <url>https://property.example</url>
    </properties>
</project>",
        );

        assert_eq!(pom.version, None);
        assert_eq!(pom.project_url, None);
    }
}
