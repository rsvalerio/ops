//! Line-based `pom.xml` parser for the Maven `project_identity` provider.
//!
//! ## Known limits
//!
//! This is a line-oriented extractor, not a real XML parser. It supports the
//! standard, prettily-formatted Maven POM shape and intentionally avoids the
//! complexity (and dependency cost) of `quick-xml`. Specifically:
//!
//! - **XML comments are stripped** (CL-3 / TASK-0846). Both single-line
//!   `<!-- … -->` blocks and multi-line `<!-- …\n…\n -->` blocks are
//!   removed before tag matching, so a commented-out `<artifactId>fake</artifactId>`
//!   does not get captured as the project artifact id.
//! - **No CDATA handling.** `<![CDATA[ ... ]]>` blocks are not unwrapped.
//! - **One element per line for top-level scalars.** Open and close tags must
//!   be on the same line for fields like `<artifactId>` (multi-line element
//!   values are not supported). Single-line `<scm>...</scm>` and
//!   `<licenses>...</licenses>` blocks are special-cased.
//! - **No nested duplicate elements.** Inside a section like `<scm>` the
//!   first matching child wins; deeper nesting (e.g. nested `<url>` inside
//!   another tag) is not tracked.
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
    /// [`try_set_once`] is the single owner of that policy; every field of
    /// this struct is written through it.
    pub(super) artifact_id: Option<String>,
    /// Maven `<name>` — display name, first-write-wins on duplicates (see
    /// [`try_set_once`], which owns the duplicate-resolution policy for the
    /// whole struct). Provider prefers this over `artifact_id` when both are
    /// present.
    pub(super) name: Option<String>,
    pub(super) version: Option<String>,
    pub(super) description: Option<String>,
    pub(super) license: Option<String>,
    pub(super) modules: Vec<String>,
    pub(super) developers: Vec<String>,
    pub(super) scm_url: Option<String>,
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
    /// Container section we deliberately ignore (organization, parent,
    /// issueManagement, ciManagement, distributionManagement). Tracks the
    /// closing tag we're waiting for so a stray `<url>` inside doesn't get
    /// captured as the SCM URL.
    Skip {
        close: &'static str,
    },
}

/// Outcome of matching a top-level line against the section openers.
///
/// PATTERN-1 / TASK-1728: `match_section_open` used to return
/// `Option<PomSection>`, where `None` meant *both* "not a section opener,
/// fall through to `parse_top_level`" and "single-line container, already
/// consumed — do **not** fall through". The dispatcher could not tell them
/// apart and always fell through, so a single-line `<parent>` /
/// `<organization>` / `<licenses>` block leaked its children into the
/// top-level fields. The three states are now distinct variants, so the
/// invariant lives in the type rather than in a comment (CL-3).
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
/// `<name>` etc. must not be captured at top level.
const SKIP_SECTIONS: &[(&str, &str)] = &[
    ("<organization>", "</organization>"),
    ("<parent>", "</parent>"),
    ("<issueManagement>", "</issueManagement>"),
    ("<ciManagement>", "</ciManagement>"),
    ("<distributionManagement>", "</distributionManagement>"),
];

pub(super) fn parse_pom_xml(project_root: &Path) -> Option<PomData> {
    // DUP-1 / TASK-0683: route through the shared manifest_io helper so the
    // NotFound-vs-other-IO classification stays consistent with sibling
    // parsers (go_mod, go_work, package_json, pyproject). Avoids a copy
    // drifting the next time the policy changes (e.g. log severity bump).
    let path = project_root.join("pom.xml");
    let content = ops_about::manifest_io::read_optional_text(&path, "pom.xml")?;

    let mut data = PomData::default();
    let mut started = false;
    let mut opener_pending = false;
    let mut section = PomSection::TopLevel;
    // CL-3 / TASK-0846: track whether we're inside a multi-line `<!-- … -->`
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
            // TASK-0626: support multi-line `<project ... >` openers, which
            // real-world Maven formatters often emit (xmlns/xsi attributes
            // split across lines). Track an "opener pending" state until the
            // closing `>` arrives.
            if opener_pending {
                // PATTERN-1 / TASK-1022: when `>` lands on this line, the
                // opener is closed but the *same* line may carry a real
                // element after it (e.g. `...">`<artifactId>x</artifactId>`).
                // Re-feed the post-`>` remainder through the started-line
                // dispatch so the trailing tag is not silently dropped.
                if let Some((_, after_gt)) = line.split_once('>') {
                    opener_pending = false;
                    started = true;
                    let remainder = after_gt.trim();
                    if remainder.is_empty() {
                        continue;
                    }
                    if dispatch_started_line(remainder, &mut section, &mut data) {
                        break;
                    }
                }
                continue;
            }
            if is_project_open(line) {
                started = true;
            } else if is_project_open_start(line) {
                opener_pending = true;
            }
            continue;
        }
        if dispatch_started_line(line, &mut section, &mut data) {
            break;
        }
    }

    Some(data)
}

/// DUP-1 / TASK-0923: classify a `<project…` line as either an opener
/// or the *start* of a multi-line opener. Returns `(matched, closed)`:
/// - `(true, true)`  — full opener on this line (bare `<project>` or
///   single-line `<project xmlns=...>`).
/// - `(true, false)` — multi-line opener (`<project` + whitespace, no
///   `>` yet on this line).
/// - `(false, _)`    — not a `<project>` opener at all (e.g. `<projectInfo>`).
///
/// Both [`is_project_open`] and [`is_project_open_start`] derive from
/// this so a future Maven shape (e.g. `<project/>`) only has to be
/// taught to one place.
fn classify_project_opener(line: &str) -> (bool, bool) {
    if line == "<project>" {
        return (true, true);
    }
    let Some(rest) = line.strip_prefix("<project") else {
        return (false, false);
    };
    if !rest.starts_with(char::is_whitespace) {
        return (false, false);
    }
    (true, rest.contains('>'))
}

/// Match the `<project>` opener exactly: the bare tag or one carrying
/// attributes (whitespace after `<project`). Rejects unrelated tags whose
/// name merely starts with `project` (e.g. `<projectInfo>`).
fn is_project_open(line: &str) -> bool {
    matches!(classify_project_opener(line), (true, true))
}

/// Match the start of a multi-line `<project ...` opener: `<project` followed
/// by whitespace (attributes) but no closing `>` on this line. Rejects
/// `<projectInfo>` for the same reason as [`is_project_open`].
fn is_project_open_start(line: &str) -> bool {
    matches!(classify_project_opener(line), (true, false))
}

/// PATTERN-1 / TASK-1022: process a single trimmed line in "started" state
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
            if *in_developer {
                if let Some(val) = extract_xml_value(line, "<name>", "</name>") {
                    data.developers.push(val.to_string());
                }
            }
        }
    }
    false
}

fn handle_scm(line: &str, data: &mut PomData) -> bool {
    if line == "</scm>" {
        return true;
    }
    try_set_once(&mut data.scm_url, line, "<url>", "</url>");
    false
}

/// DUP-1 / TASK-0869: write `field` from a `<tag>value</tag>` line iff the
/// field is still empty. Encodes the "first writer wins on duplicates"
/// invariant in a single helper so a future refactor cannot accidentally
/// let a later top-level `<url>` clobber the `<scm><url>` already captured
/// (regression pinned by `parse_pom_scm_takes_precedence_over_url`).
fn try_set_once(field: &mut Option<String>, line: &str, open: &str, close: &str) {
    if field.is_none() {
        if let Some(val) = extract_xml_value(line, open, close) {
            *field = Some(val.to_string());
        }
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
        return SectionOutcome::Entered(PomSection::Developers {
            in_developer: false,
        });
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
    // READ-2 / TASK-0691: a single-line `<licenses>...</licenses>` may carry
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

    // PATTERN-1: a single-line `<developers>...</developers>` used to fall
    // through to `parse_top_level`, whose `<name>` rule then captured the
    // developer's name as the *project* name — and the provider prefers
    // `name` over `artifact_id`, so the project displayed as a person. Handle
    // the collapsed form here and keep the developer, mirroring the multi-line
    // `handle_developers` policy (every `<name>` inside a `<developer>`).
    if line.starts_with("<developers>")
        && line.ends_with("</developers>")
        && line.matches("<developers>").count() == 1
    {
        let mut rest = line;
        while let Some(start) = rest.find("<developer>") {
            let Some(after) = rest.get(start.saturating_add("<developer>".len())..) else {
                break;
            };
            let Some(end) = after.find("</developer>") else {
                break;
            };
            if let Some(entry) = after.get(..end) {
                if let Some(val) = extract_xml_value(entry, "<name>", "</name>") {
                    data.developers.push(val.to_string());
                }
            }
            let Some(next) = after.get(end.saturating_add("</developer>".len())..) else {
                break;
            };
            rest = next;
        }
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
    try_set_once(
        &mut data.description,
        line,
        "<description>",
        "</description>",
    );
    try_set_once(&mut data.name, line, "<name>", "</name>");
    try_set_once(&mut data.scm_url, line, "<url>", "</url>");
}

/// CL-3 / TASK-0846: strip XML comments from `line`, multi-line aware.
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
/// ERR-1 / TASK-0916: decodes the XML predefined entity references
/// (`&amp;` `&lt;` `&gt;` `&quot;` `&apos;`) plus numeric `&#NNN;` /
/// `&#xHH;` so `Foo &amp; Bar` no longer renders as the literal
/// `Foo &amp; Bar` in the About card. Returns `Cow::Borrowed` (no
/// allocation) for entity-free values, `Cow::Owned` only when a
/// decoded substitution was needed.
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
        line.get(val_start..end)
            .map(|val| decode_xml_entities(val.trim()))
    } else {
        None
    }
}

/// ERR-1 / TASK-0916: minimal XML-1.0 predefined-entity + numeric-char-ref
/// decoder. Borrows the input when no `&` is present (the common case).
fn decode_xml_entities(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains('&') {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some((before, after_amp)) = rest.split_once('&') {
        out.push_str(before);
        // The `len() <= 8` guard is the former `find(';')` offset bound: the
        // entity name is everything up to the `;`, so its length is that
        // offset.
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
            other if other.starts_with('#') => other
                .get(1..)
                .and_then(|dec| dec.parse::<u32>().ok())
                .and_then(char::from_u32),
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
            extract_xml_value(
                "<artifactId>camel</artifactId>",
                "<artifactId>",
                "</artifactId>"
            )
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

    /// ERR-1 / TASK-0916: standard XML predefined entities + numeric
    /// references must be decoded so the rendered About card shows the
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

    /// ERR-1 / TASK-0916: an unknown entity is left verbatim (so we
    /// don't silently corrupt content the parser doesn't understand).
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
        assert_eq!(
            extract_xml_value("<name>foo</name>", "<version>", "</version>"),
            None
        );
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
        assert_eq!(
            pom.scm_url,
            Some("https://github.com/user/myapp".to_string())
        );
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

    #[test]
    fn parse_pom_top_level_url_fallback() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <artifactId>mylib</artifactId>\n    <url>https://example.com</url>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.scm_url, Some("https://example.com".to_string()));
    }

    #[test]
    fn parse_pom_scm_takes_precedence_over_url() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r"<project>
    <artifactId>mylib</artifactId>
    <scm>
        <url>https://github.com/user/mylib</url>
    </scm>
    <url>https://example.com</url>
</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(
            pom.scm_url,
            Some("https://github.com/user/mylib".to_string())
        );
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
        assert_eq!(
            pom.scm_url,
            Some("https://github.com/user/myapp".to_string())
        );
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
        // PATTERN-1 / TASK-1728: the consumed container line must not be
        // re-parsed at top level (it carries no `<name>`, but the old
        // fall-through would have run `parse_top_level` on it).
        assert_eq!(pom.name, None);
    }

    /// PATTERN-1 / TASK-1728: a single-line `<parent>` block — the standard
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

    /// PATTERN-1 / TASK-1728: a single-line `<organization>` block must not
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
        // PATTERN-1 / TASK-1728: the license `<name>` must not double as the
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
        // Two `<scm>` openers on one line is malformed. The single-line scm
        // detector now rejects this shape (it would otherwise extract a URL
        // from a line we have not really proven to be one scm element). The
        // top-level `<url>` fallback still picks up the first URL, which is
        // the deterministic outcome we pin here.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <scm><url>https://first.example</url></scm><scm><url>https://second.example</url></scm>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.scm_url, Some("https://first.example".to_string()));
    }

    #[test]
    fn parse_pom_multiline_project_opener() {
        // Real-world formatters often split xmlns/xsi attributes across
        // lines. TASK-0626: parser must treat the opener as continuing until
        // the first `>` and resume normal scanning afterwards.
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

    /// PATTERN-1 / TASK-1022: a multi-line `<project ...>` opener whose
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

    /// CL-3 / TASK-0846: a `<artifactId>` inside an XML comment must NOT
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

    /// CL-3 / TASK-0846: multi-line comment block hides every captured
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

    /// CL-3 / TASK-0846: helper-level edge cases.
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

    /// PATTERN-1: a `<developers>` container collapsed onto one line must not
    /// leak its developer `<name>` into the project `<name>`. The provider
    /// prefers `name` over `artifact_id`, so the regression rendered the
    /// project as whoever was listed first.
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
        assert!(
            pom.name.is_none(),
            "developer name leaked into the project name: {:?}",
            pom.name
        );
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
}
