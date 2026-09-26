use nexa_syntax::catalog;

use crate::line_index::LineIndex;
use crate::protocol::{CompletionItem, CompletionItemKind, Position};

/// Generates code completions for the given document and cursor position.
///
/// Every label comes from the shared language catalog in `nexa-syntax`, so
/// only names accepted by the parser are ever offered. The cursor context
/// decides the set: after `.` only the trailing dot-modifiers the parser
/// accepts are offered; otherwise components, keywords, and builtin types
/// are offered, filtered by the identifier prefix under the cursor.
pub fn get_completions(source: &str, position: Position) -> Vec<CompletionItem> {
    let index = LineIndex::new(source);
    let (prefix, after_dot) = completion_context(source, &index, position);
    if after_dot {
        return catalog::DOT_MODIFIERS
            .iter()
            .filter(|entry| entry.name.starts_with(prefix.as_str()))
            .map(|entry| CompletionItem {
                label: entry.name.to_string(),
                kind: Some(CompletionItemKind::Method),
                detail: Some(format!(".{}(...)", entry.name)),
                documentation: Some(entry.summary.to_string()),
                insert_text: Some(entry.snippet.to_string()),
            })
            .collect();
    }

    let mut items = Vec::new();
    for entry in catalog::COMPONENTS {
        if !entry.name.starts_with(prefix.as_str()) {
            continue;
        }
        items.push(CompletionItem {
            label: entry.name.to_string(),
            kind: Some(CompletionItemKind::Constructor),
            detail: Some(format!("Nexa UI Component: {}", entry.name)),
            documentation: Some(entry.summary.to_string()),
            insert_text: Some(entry.snippet.to_string()),
        });
    }
    for entry in catalog::KEYWORDS {
        if !entry.name.starts_with(prefix.as_str()) {
            continue;
        }
        items.push(CompletionItem {
            label: entry.name.to_string(),
            kind: Some(CompletionItemKind::Keyword),
            detail: Some("Nexa Keyword".to_string()),
            documentation: Some(entry.summary.to_string()),
            insert_text: None,
        });
    }
    for entry in catalog::TYPES {
        if !entry.name.starts_with(prefix.as_str()) {
            continue;
        }
        items.push(CompletionItem {
            label: entry.name.to_string(),
            kind: Some(CompletionItemKind::Class),
            detail: Some(format!("Nexa Type: {}", entry.name)),
            documentation: Some(entry.summary.to_string()),
            insert_text: None,
        });
    }
    items
}

/// Returns the identifier prefix immediately before the cursor and whether
/// the cursor sits in member position (the prefix follows a `.`).
fn completion_context(source: &str, index: &LineIndex, position: Position) -> (String, bool) {
    let Some(cursor) = index.to_offset(source, position) else {
        return (String::new(), false);
    };
    let before = &source[..cursor.min(source.len())];
    // Walk by characters so `prefix_start` is always a character boundary,
    // even when non-ASCII text precedes the cursor.
    let mut prefix_start = 0;
    for (index, ch) in before.char_indices() {
        if !is_prefix_char(ch) {
            prefix_start = index + ch.len_utf8();
        }
    }
    let prefix = before[prefix_start..].to_string();
    let after_dot = before[..prefix_start].ends_with('.');
    (prefix, after_dot)
}

fn is_prefix_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::get_completions;
    use crate::protocol::Position;

    fn labels_at(source: &str, line: u32, character: u32) -> Vec<String> {
        get_completions(source, Position { line, character })
            .into_iter()
            .map(|item| item.label)
            .collect()
    }

    #[test]
    fn unprefixed_cursor_offers_the_full_catalog() {
        let labels = labels_at("", 0, 0);
        assert!(labels.contains(&"Column".to_string()));
        assert!(labels.contains(&"TextInput".to_string()));
        assert!(labels.contains(&"FastList".to_string()));
        assert!(labels.contains(&"state".to_string()));
        assert!(labels.contains(&"Result".to_string()));
    }

    #[test]
    fn unsupported_parser_names_are_never_offered() {
        let labels = labels_at("", 0, 0);
        for rejected in [
            "TextField",
            "FastSectionedList",
            "Spacer",
            "Divider",
            "VStack",
            "HStack",
            "ZStack",
        ] {
            assert!(
                !labels.contains(&rejected.to_string()),
                "{rejected} must not be completed"
            );
        }
    }

    #[test]
    fn prefix_filters_offered_items() {
        let source = "app P {\n    body {\n        Tex\n    }\n}\n";
        let labels = labels_at(source, 2, 11);
        assert!(labels.contains(&"Text".to_string()));
        assert!(labels.contains(&"TextInput".to_string()));
        assert!(!labels.contains(&"Column".to_string()));
        assert!(!labels.contains(&"Button".to_string()));
    }

    #[test]
    fn dot_context_offers_only_trailing_modifiers() {
        let source = "app P {\n    body {\n        FastList(count: 3) { index in Text(\"x\") }.on\n    }\n}\n";
        let offset = source.find(".on").expect("probe") + 3;
        let line_start = source[..offset]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        // "on" is 2 UTF-16 units on an ASCII line, so the byte offset doubles
        // as the character position here.
        let labels = labels_at(source, 2, (offset - line_start) as u32);
        assert!(labels.contains(&"onEndReached".to_string()));
        assert!(labels.contains(&"onScroll".to_string()));
        assert!(!labels.contains(&"Text".to_string()));
        assert!(!labels.contains(&"Column".to_string()));
    }

    #[test]
    fn non_ascii_text_before_cursor_keeps_prefix_correct() {
        // The emoji occupies 2 UTF-16 units on line 0; the cursor on line 1
        // sits after "Tex" (8 spaces + 3 identifier characters).
        let source = "state label = \"😀\"\n        Tex\n";
        let labels = labels_at(source, 1, 11);
        assert!(labels.contains(&"Text".to_string()));
        assert!(!labels.contains(&"Column".to_string()));
    }

    #[test]
    fn out_of_range_cursor_falls_back_to_full_catalog() {
        let labels = labels_at("app P {}", 99, 0);
        assert!(labels.contains(&"Column".to_string()));
    }
}
