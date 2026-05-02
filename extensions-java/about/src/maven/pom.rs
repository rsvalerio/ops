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
    /// Maven `<artifactId>` — coordinate, last-write-wins on duplicates.
    pub(super) artifact_id: Option<String>,
    /// Maven `<name>` — display name, last-write-wins on duplicates.
    /// Provider prefers this over `artifact_id` when both are present.
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
                if line.contains('>') {
                    opener_pending = false;
                    started = true;
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
        if line == "</project>" {
            break;
        }

        if matches!(section, PomSection::TopLevel) {
            if let Some(new_section) = match_section_open(line, &mut data) {
                section = new_section;
            } else {
                parse_top_level(line, &mut data);
            }
            continue;
        }

        if handle_section_line(&mut section, line, &mut data) {
            section = PomSection::TopLevel;
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

/// Dispatch a line to the active section's handler. Returns `true` when the
/// section's closing tag was seen and the parser should return to `TopLevel`.
fn handle_section_line(section: &mut PomSection, line: &str, data: &mut PomData) -> bool {
    match section {
        PomSection::Modules => handle_modules(line, data),
        PomSection::Developers { in_developer } => handle_developers(line, in_developer, data),
        PomSection::Scm => handle_scm(line, data),
        PomSection::Licenses { in_license } => handle_licenses(line, in_license, data),
        PomSection::Skip { close } => line == *close,
        PomSection::TopLevel => unreachable!(),
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

/// Match opening tags for POM sections. Single-line `<scm>...</scm>` and
/// `<licenses>...</licenses>` blocks are extracted in place and leave the
/// caller in `TopLevel`.
fn match_section_open(line: &str, data: &mut PomData) -> Option<PomSection> {
    if line == "<modules>" {
        return Some(PomSection::Modules);
    }
    if line == "<developers>" {
        return Some(PomSection::Developers {
            in_developer: false,
        });
    }
    if line == "<scm>" {
        return Some(PomSection::Scm);
    }
    if line == "<licenses>" {
        return Some(PomSection::Licenses { in_license: false });
    }

    // Single-line forms: `<scm><url>...</url></scm>` or
    // `<licenses><license><name>...</name></license></licenses>`.
    // Reject malformed inputs with duplicated openers (e.g. `<scm>...<scm>`)
    // to keep the partial-input handler honest.
    if line.starts_with("<scm>") && line.ends_with("</scm>") && line.matches("<scm>").count() == 1 {
        try_set_once(&mut data.scm_url, line, "<url>", "</url>");
        return None;
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
        return None;
    }

    for (open, close) in SKIP_SECTIONS {
        if line == *open {
            return Some(PomSection::Skip { close });
        }
        // Single-line container — ignore entirely.
        if line.starts_with(*open) && line.ends_with(*close) {
            return None;
        }
    }

    None
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
            match rest.find("-->") {
                Some(end) => {
                    rest = &rest[end + 3..];
                    *in_comment = false;
                }
                None => return out,
            }
        }
        match rest.find("<!--") {
            Some(start) => {
                out.push_str(&rest[..start]);
                // Insert a separator so e.g. `foo<!--x-->bar` doesn't
                // collapse to `foobar` if a future caller relies on
                // word-boundary parsing. Tag matching uses literal
                // `<tag>` patterns so a stray space is harmless.
                out.push(' ');
                rest = &rest[start + 4..];
                *in_comment = true;
            }
            None => {
                out.push_str(rest);
                return out;
            }
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
    let val_start = start + open.len();
    if val_start < end {
        Some(decode_xml_entities(line[val_start..end].trim()))
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
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after_amp = &rest[amp + 1..];
        let semi = match after_amp.find(';') {
            Some(i) if i <= 8 => i,
            _ => {
                // Stray `&` or runaway entity: leave verbatim and continue.
                out.push('&');
                rest = after_amp;
                continue;
            }
        };
        let entity = &after_amp[..semi];
        let decoded: Option<char> = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            other if other.starts_with("#x") || other.starts_with("#X") => {
                u32::from_str_radix(&other[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
            }
            other if other.starts_with('#') => {
                other[1..].parse::<u32>().ok().and_then(char::from_u32)
            }
            _ => None,
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &after_amp[semi + 1..];
            }
            None => {
                // Unknown entity: keep verbatim (`&entity;`) and advance.
                out.push('&');
                out.push_str(entity);
                out.push(';');
                rest = &after_amp[semi + 1..];
            }
        }
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
            r#"<project>
    <artifactId>mylib</artifactId>
    <scm>
        <url>https://github.com/user/mylib</url>
    </scm>
    <url>https://example.com</url>
</project>"#,
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
            r#"<project>
    <artifactId>multi</artifactId>
    <developers>
        <developer>
            <name>Alice</name>
        </developer>
        <developer>
            <name>Bob</name>
        </developer>
    </developers>
</project>"#,
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
            r#"<project>
    <organization>
        <name>Acme</name>
        <url>https://acme.example</url>
    </organization>
    <scm>
        <url>https://github.com/user/myapp</url>
    </scm>
</project>"#,
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
    }

    #[test]
    fn parse_pom_stray_name_in_licenses_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<project>
    <licenses>
        <name>stray</name>
        <license>
            <name>Apache-2.0</name>
        </license>
    </licenses>
</project>"#,
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

    /// CL-3 / TASK-0846: a `<artifactId>` inside an XML comment must NOT
    /// be captured. The release/SNAPSHOT swap pattern is common in real
    /// repos.
    #[test]
    fn parse_pom_commented_artifact_id_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<project>
    <!-- <artifactId>fake-snapshot</artifactId> -->
    <artifactId>real-release</artifactId>
</project>"#,
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
            r#"<project>
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
</project>"#,
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
}
