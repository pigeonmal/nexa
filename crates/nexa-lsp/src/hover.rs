use nexa_syntax::catalog;

use crate::line_index::LineIndex;
use crate::protocol::{Hover, MarkupContent, Position, Range};

/// Returns hover documentation for the token under the cursor.
///
/// The cursor column is interpreted as UTF-16 code units per the LSP
/// contract, converted to a byte offset through [`LineIndex`] so non-ASCII
/// text selects the correct token and never slices at a non-character
/// boundary. Documentation comes from the shared language catalog, so hover
/// can only describe names the parser accepts.
pub fn get_hover(source: &str, position: Position) -> Option<Hover> {
    let index = LineIndex::new(source);
    let (word, range) = word_at_position(source, &index, position)?;
    let value = hover_documentation(&word)?;
    Some(Hover {
        contents: MarkupContent {
            kind: "markdown".to_string(),
            value,
        },
        range: Some(range),
    })
}

/// Extracts the identifier under the cursor and its LSP range.
fn word_at_position(
    source: &str,
    index: &LineIndex,
    position: Position,
) -> Option<(String, Range)> {
    let cursor = index.to_offset(source, position)?;
    let line_start = source[..cursor.min(source.len())]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let line_end = source[line_start..]
        .find('\n')
        .map(|index| line_start + index)
        .unwrap_or(source.len());
    let line = &source[line_start..line_end];
    let relative = cursor - line_start;
    if relative > line.len() {
        return None;
    }

    let mut start = relative;
    while start > 0 {
        let ch = line[..start].chars().next_back()?;
        if !is_ident_char(ch) {
            break;
        }
        start -= ch.len_utf8();
    }
    let mut end = relative;
    // A cursor between characters belongs to the token on either side; when
    // it splits a word character pair the forward scan finds the token.
    while end < line.len() {
        let Some(ch) = line[end..].chars().next() else {
            break;
        };
        if !is_ident_char(ch) {
            break;
        }
        end += ch.len_utf8();
    }
    if start == end {
        return None;
    }
    let mut word = line[start..end].to_string();
    // An optional-chaining `?` suffix is not part of the documented name.
    while word.ends_with('?') {
        word.pop();
        end -= 1;
    }
    if word.is_empty() {
        return None;
    }
    let range = Range {
        start: index.to_position(source, line_start + start),
        end: index.to_position(source, line_start + end),
    };
    Some((word, range))
}

fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn hover_documentation(token: &str) -> Option<String> {
    if let Some(entry) = catalog::component(token) {
        let reference = catalog::component_schema(token)
            .map(|schema| format!("\n\nReference: {}", schema.doc))
            .unwrap_or_default();
        return Some(format!(
            "### `{}`\n\n{}\n\n```nx\n{}\n```{reference}",
            entry.name, entry.summary, entry.snippet
        ));
    }
    if let Some(entry) = catalog::dot_modifier(token) {
        return Some(format!(
            "### `.{}`\n\n{}\n\n```nx\n{}\n```",
            entry.name, entry.summary, entry.snippet
        ));
    }
    if let Some(entry) = catalog::keyword(token) {
        return Some(format!("### `{}` keyword\n\n{}", entry.name, entry.summary));
    }
    if let Some(entry) = catalog::builtin_type(token) {
        return Some(format!("### `{}`\n\n{}", entry.name, entry.summary));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::get_hover;
    use crate::protocol::Position;

    fn hover_at(source: &str, line: u32, character: u32) -> Option<String> {
        get_hover(source, Position { line, character }).map(|hover| hover.contents.value)
    }

    #[test]
    fn hover_documents_catalog_components() {
        let source = "app Demo {\n    body {\n        FastList(count: 3) { index in Text(\"x\") }\n    }\n}\n";
        let doc = hover_at(source, 2, 10).expect("hover over FastList");
        assert!(doc.contains("FastList"));
    }

    #[test]
    fn hover_rejects_unsupported_names() {
        let source = "app Demo {\n    body {\n        TextField()\n    }\n}\n";
        assert!(hover_at(source, 2, 10).is_none());
    }

    #[test]
    fn hover_documents_dot_modifiers_and_doc_reference() {
        let source =
            "app Demo {\n    body {\n        Pressable() { Text(\"x\") }.onPress { }\n    }\n}\n";
        let doc = hover_at(source, 2, 38).expect("hover over onPress");
        assert!(doc.contains("`.onPress`"));
        let component = hover_at(source, 2, 10).expect("hover over Pressable");
        assert!(component.contains("Reference: components.md#pressable"));
    }

    #[test]
    fn hover_counts_columns_in_utf16_code_units() {
        // "é" is 2 bytes but 1 UTF-16 unit: `Text` starts at byte 9 but
        // column 8. Byte-based math would land on the space and miss it.
        let source = "// café Text\n";
        let doc = hover_at(source, 0, 8).expect("hover over Text");
        assert!(doc.contains("### `Text`"));
    }

    #[test]
    fn hover_selects_token_after_emoji_on_earlier_text() {
        // Line 0 holds an emoji (2 UTF-16 units, 4 bytes). Line 1 is ASCII;
        // hovering "Text" must resolve through the line table, not byte math.
        let source = "state icon = \"😀\"\n        Text(\"hi\")\n";
        let doc = hover_at(source, 1, 10).expect("hover over Text");
        assert!(doc.contains("### `Text`"));
    }

    #[test]
    fn hover_returns_the_token_range_in_utf16() {
        // `Text` follows non-ASCII text on the same line: it starts at byte
        // 14 but UTF-16 column 13 and is 4 units wide.
        let source = "state café = Text\n";
        let hover = get_hover(
            source,
            Position {
                line: 0,
                character: 14,
            },
        )
        .expect("hover");
        let range = hover.range.expect("range");
        assert_eq!((range.start.line, range.start.character), (0, 13));
        assert_eq!((range.end.line, range.end.character), (0, 17));
    }

    #[test]
    fn hover_mid_token_and_boundaries_do_not_panic() {
        let source = "héllo wörld\n";
        for character in 0..12 {
            let _ = hover_at(source, 0, character);
        }
        assert!(hover_at(source, 9, 0).is_none());
    }
}
