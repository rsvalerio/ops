//! Line-based `pom.xml` parser for the Maven `project_identity` provider.
//!
//! ## Known limits
//!
//! This is a line-oriented extractor, not a real XML parser. It supports the
//! standard, prettily-formatted Maven POM shape and intentionally avoids the
//! complexity (and dependency cost) of `quick-xml`. Specifically:
//!
//! - **No XML comment handling.** Tags inside `<!-- ... -->` are matched as
//!   regular content if they appear on a non-commented line shape.
//! - **No CDATA handling.** `<![CDATA[ ... ]]>` blocks are not unwrapped.
//! - **One element per line.** Open and close tags must be on the same line
//!   (e.g. `<artifactId>foo</artifactId>`); multi-line element values are
//!   not supported.
//! - **No nested duplicate elements.** Inside a section like `<scm>` the
//!   first matching child wins; deeper nesting (e.g. nested `<url>` inside
//!   another tag) is not tracked.
//!
//! Replacing this with `quick-xml` is a single-module swap; callers depend
//! only on `parse_pom_xml` and `PomData`.

use std::path::Path;

pub(super) struct PomData {
    pub(super) artifact_id: Option<String>,
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
    Developers { in_developer: bool },
    Scm,
    Licenses,
}

pub(super) fn parse_pom_xml(project_root: &Path) -> Result<PomData, std::io::Error> {
    let path = project_root.join("pom.xml");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!(path = %path.display(), error = %e, "failed to read pom.xml");
            return Err(e);
        }
    };

    let mut data = PomData {
        artifact_id: None,
        version: None,
        description: None,
        license: None,
        modules: Vec::new(),
        developers: Vec::new(),
        scm_url: None,
    };

    let mut started = false;
    let mut section = PomSection::TopLevel;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("<project") {
            started = true;
            continue;
        }
        if line == "</project>" {
            break;
        }
        if !started {
            continue;
        }

        if matches!(section, PomSection::TopLevel) {
            if let Some(new_section) = match_section_open(line) {
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

    Ok(data)
}

/// Dispatch a line to the active section's handler. Returns `true` when the
/// section's closing tag was seen and the parser should return to `TopLevel`.
fn handle_section_line(section: &mut PomSection, line: &str, data: &mut PomData) -> bool {
    match section {
        PomSection::Modules => handle_modules(line, data),
        PomSection::Developers { in_developer } => handle_developers(line, in_developer, data),
        PomSection::Scm => handle_scm(line, data),
        PomSection::Licenses => handle_licenses(line, data),
        PomSection::TopLevel => unreachable!(),
    }
}

fn handle_modules(line: &str, data: &mut PomData) -> bool {
    if line == "</modules>" {
        return true;
    }
    if let Some(val) = extract_xml_value(line, "module") {
        data.modules.push(val);
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
                if let Some(val) = extract_xml_value(line, "name") {
                    data.developers.push(val);
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
    if data.scm_url.is_none() {
        data.scm_url = extract_xml_value(line, "url");
    }
    false
}

fn handle_licenses(line: &str, data: &mut PomData) -> bool {
    if line == "</licenses>" {
        return true;
    }
    if data.license.is_none() {
        data.license = extract_xml_value(line, "name");
    }
    false
}

/// Match opening tags for POM sections.
fn match_section_open(line: &str) -> Option<PomSection> {
    if line == "<modules>" {
        Some(PomSection::Modules)
    } else if line == "<developers>" {
        Some(PomSection::Developers {
            in_developer: false,
        })
    } else if line == "<scm>" || line.starts_with("<scm>") {
        Some(PomSection::Scm)
    } else if line == "<licenses>" || line.starts_with("<licenses>") {
        Some(PomSection::Licenses)
    } else {
        None
    }
}

/// Parse top-level simple elements (artifactId, version, description, name, url).
fn parse_top_level(line: &str, data: &mut PomData) {
    if data.artifact_id.is_none() {
        if let Some(val) = extract_xml_value(line, "artifactId") {
            data.artifact_id = Some(val);
        }
    }
    if data.version.is_none() {
        data.version = extract_xml_value(line, "version");
    }
    if data.description.is_none() {
        data.description = extract_xml_value(line, "description");
    }
    if let Some(val) = extract_xml_value(line, "name") {
        data.artifact_id = Some(val);
    }
    if data.scm_url.is_none() {
        data.scm_url = extract_xml_value(line, "url");
    }
}

/// Extract value from `<tag>value</tag>` on a single line.
fn extract_xml_value(line: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    if let Some(start) = line.find(&open) {
        if let Some(end) = line.find(&close) {
            let val_start = start + open.len();
            if val_start < end {
                return Some(line[val_start..end].trim().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_xml_value_basic() {
        assert_eq!(
            extract_xml_value("<artifactId>camel</artifactId>", "artifactId"),
            Some("camel".to_string())
        );
    }

    #[test]
    fn extract_xml_value_with_whitespace() {
        assert_eq!(
            extract_xml_value("    <version>1.0</version>  ", "version"),
            Some("1.0".to_string())
        );
    }

    #[test]
    fn extract_xml_value_no_match() {
        assert_eq!(extract_xml_value("<name>foo</name>", "version"), None);
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
        assert_eq!(pom.artifact_id, Some("My App".to_string()));
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
        assert!(parse_pom_xml(dir.path()).is_err());
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
}
