//! Reachability selection for named sections of generated native source.
//!
//! Large native templates can be shared by several backends or by AOT and
//! DevRuntime, while an AOT app only needs the helpers reachable from its
//! generated call sites. A section begins with `// nexa-codegen: <symbol>`;
//! references between declared symbols pull their sections into the output.

const SECTION_MARKER: &str = "// nexa-codegen: ";

#[derive(Debug)]
struct Section {
    name: String,
    source: String,
}

/// Renders the source sections reachable from identifiers in `references`.
///
/// References are expanded transitively between section names, and output
/// order remains the template's source order for deterministic codegen.
pub fn reachable(template: &str, references: &str) -> String {
    let sections = parse_sections(template);
    let mut included = vec![false; sections.len()];
    for (index, section) in sections.iter().enumerate() {
        included[index] = contains_identifier(references, &section.name);
    }

    loop {
        let mut changed = false;
        for (index, section) in sections.iter().enumerate() {
            if !included[index] {
                continue;
            }
            for (candidate, dependency) in sections.iter().enumerate() {
                if !included[candidate] && contains_identifier(&section.source, &dependency.name) {
                    included[candidate] = true;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    render_sections(&sections, &included)
}

/// Renders every named source section, removing the section markers.
pub fn all(template: &str) -> String {
    let sections = parse_sections(template);
    render_sections(&sections, &vec![true; sections.len()])
}

fn parse_sections(template: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current: Option<Section> = None;
    for line in template.lines() {
        if let Some(name) = line.strip_prefix(SECTION_MARKER) {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            current = Some(Section {
                name: name.trim().to_owned(),
                source: String::new(),
            });
        } else if let Some(section) = current.as_mut() {
            section.source.push_str(line);
            section.source.push('\n');
        }
    }
    if let Some(section) = current {
        sections.push(section);
    }
    sections
}

fn render_sections(sections: &[Section], included: &[bool]) -> String {
    let mut output = String::new();
    for (section, include) in sections.iter().zip(included) {
        if *include {
            output.push_str(&section.source);
        }
    }
    output
}

fn contains_identifier(source: &str, identifier: &str) -> bool {
    source.match_indices(identifier).any(|(start, _)| {
        let end = start + identifier.len();
        let before_is_identifier = source[..start]
            .bytes()
            .next_back()
            .is_some_and(is_identifier_byte);
        let after_is_identifier = source
            .as_bytes()
            .get(end)
            .is_some_and(|byte| is_identifier_byte(*byte));
        !before_is_identifier && !after_is_identifier
    })
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::{all, reachable};

    const TEMPLATE: &str = r#"// nexa-codegen: Root
private fun Root() { Leaf() }
// nexa-codegen: Leaf
private fun Leaf() { val value = 1 }
// nexa-codegen: Unused
private fun Unused() { val value = 2 }
"#;

    #[test]
    fn reachable_sections_include_transitive_dependencies_in_source_order() {
        let source = reachable(TEMPLATE, "Root()");

        assert!(source.contains("private fun Root()"));
        assert!(source.contains("private fun Leaf()"));
        assert!(!source.contains("private fun Unused()"));
        assert!(!source.contains("nexa-codegen:"));
    }

    #[test]
    fn all_sections_preserve_the_complete_template_without_markers() {
        let source = all(TEMPLATE);

        assert!(source.contains("private fun Root()"));
        assert!(source.contains("private fun Leaf()"));
        assert!(source.contains("private fun Unused()"));
        assert!(!source.contains("nexa-codegen:"));
    }

    #[test]
    fn identifiers_do_not_match_substrings() {
        assert!(!super::contains_identifier("Rooted()", "Root"));
        assert!(!super::contains_identifier("prefixRoot()", "Root"));
        assert!(!super::contains_identifier("Root_suffix()", "Root"));
        assert!(super::contains_identifier("Root()", "Root"));
    }
}
