//! Reachability selection for named sections of generated native source.
//!
//! Large native templates can be shared by several backends or by AOT and
//! DevRuntime, while an AOT app only needs the helpers reachable from its
//! generated call sites. A section begins with `// nexa-codegen: <symbol>`;
//! references between declared symbols pull their sections into the output.

use std::collections::HashMap;

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
    let mut symbols: HashMap<&str, Vec<usize>> = HashMap::with_capacity(sections.len());
    let mut non_identifiers = Vec::new();
    for (index, section) in sections.iter().enumerate() {
        if !section.name.is_empty() && section.name.bytes().all(is_identifier_byte) {
            symbols.entry(&section.name).or_default().push(index);
        } else {
            // Preserve the conservative behavior for externally supplied
            // section labels which are not a single native identifier.
            non_identifiers.push((index, section.name.as_str()));
        }
    }
    let mut pending = Vec::with_capacity(sections.len());
    enqueue_references(
        references,
        &symbols,
        &non_identifiers,
        &mut included,
        &mut pending,
    );
    while let Some(index) = pending.pop() {
        enqueue_references(
            &sections[index].source,
            &symbols,
            &non_identifiers,
            &mut included,
            &mut pending,
        );
    }

    render_sections(&sections, &included)
}

/// Scan each reached section once instead of rescanning every section for each
/// possible dependency at every fixed-point iteration. Tokenization remains
/// intentionally conservative: names in native comments and strings count too.
fn enqueue_references(
    source: &str,
    symbols: &HashMap<&str, Vec<usize>>,
    non_identifiers: &[(usize, &str)],
    included: &mut [bool],
    pending: &mut Vec<usize>,
) {
    for token in source.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_') {
        if let Some(indices) = symbols.get(token) {
            for &index in indices {
                if !included[index] {
                    included[index] = true;
                    pending.push(index);
                }
            }
        }
    }
    for &(index, name) in non_identifiers {
        if !included[index] && contains_identifier(source, name) {
            included[index] = true;
            pending.push(index);
        }
    }
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
    let capacity = sections
        .iter()
        .zip(included)
        .filter_map(|(section, include)| include.then_some(section.source.len()))
        .sum();
    let mut output = String::with_capacity(capacity);
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

    #[test]
    fn reachable_graph_preserves_cycles_duplicates_and_non_identifier_labels() {
        let template = "// nexa-codegen: A\nA calls B and A::helper\n\
            // nexa-codegen: B\nB calls A\n\
            // nexa-codegen: A\nA also calls C\n\
            // nexa-codegen: C\nC is a leaf\n\
            // nexa-codegen: A::helper\nhelper body\n\
            // nexa-codegen: Ax\nunreachable body\n";
        let result = reachable(template, "éA()");
        assert_eq!(
            result,
            "A calls B and A::helper\nB calls A\nA also calls C\nC is a leaf\nhelper body\n"
        );
        assert!(reachable(template, "prefixA() A_suffix()").is_empty());
    }

    #[test]
    fn graph_traversal_matches_conservative_fixed_point_selection() {
        let mut template = String::new();
        for index in 0..80 {
            template.push_str(&format!("// nexa-codegen: Symbol{index}\n"));
            template.push_str(&format!(
                "Symbol{index} calls Symbol{} and Symbol{}; Symbol{}_suffix\n",
                (index * 7 + 3) % 80,
                (index + 79) % 80,
                (index + 11) % 80
            ));
        }
        let sections = super::parse_sections(&template);
        for root in [
            "Symbol0()",
            "Symbol17() Symbol38()",
            "Symbol79_suffix()",
            "",
        ] {
            let mut included: Vec<_> = sections
                .iter()
                .map(|section| super::contains_identifier(root, &section.name))
                .collect();
            loop {
                let before = included.clone();
                for (index, section) in sections.iter().enumerate() {
                    if included[index] {
                        for (candidate, dependency) in sections.iter().enumerate() {
                            included[candidate] |=
                                super::contains_identifier(&section.source, &dependency.name);
                        }
                    }
                }
                if included == before {
                    break;
                }
            }
            assert_eq!(
                reachable(&template, root),
                super::render_sections(&sections, &included)
            );
        }
    }
}
