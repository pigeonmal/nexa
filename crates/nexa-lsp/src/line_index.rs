//! Byte-offset ↔ LSP position translation.
//!
//! LSP `Position.character` values are UTF-16 code units (unless the client
//! negotiates another encoding), while Nexa spans are byte offsets into the
//! source. This index is built once per document version over the source
//! bytes and converts in both directions without ever slicing at a
//! non-character boundary.

use crate::protocol::{Position, Range};
use nexa_diagnostics::Span;

/// Line table for one document version.
#[derive(Clone, Debug, Default)]
pub struct LineIndex {
    /// Byte offset at which each 0-based line starts. Always non-empty with
    /// `line_starts[0] == 0`.
    line_starts: Vec<usize>,
}

impl LineIndex {
    /// Builds the index over `source`. Lines are split on `\n`.
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (index, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Byte range of the `line`-th 0-based line, excluding the line break. A
    /// trailing `\r` (CRLF documents) is excluded so columns match editors.
    fn line_bytes<'a>(&self, source: &'a str, line: usize) -> Option<(usize, &'a str)> {
        let start = *self.line_starts.get(line)?;
        let mut end = source.len();
        if let Some(next) = self.line_starts.get(line + 1) {
            end = next.saturating_sub(1);
        }
        let mut text = source.get(start..end).unwrap_or("");
        if text.ends_with('\r') {
            text = &text[..text.len() - 1];
        }
        Some((start, text))
    }

    /// Converts a byte offset into an LSP position. Offsets past the end of
    /// the document clamp to the end; offsets inside a character snap down to
    /// that character's start.
    pub fn to_position(&self, source: &str, offset: usize) -> Position {
        let mut offset = offset.min(source.len());
        let line = self
            .line_starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts.get(line).copied().unwrap_or(0);
        while offset > line_start && !source.is_char_boundary(offset) {
            offset -= 1;
        }
        let (_, text) = self.line_bytes(source, line).unwrap_or((offset, ""));
        let mut character: u32 = 0;
        for (relative, ch) in text.char_indices() {
            if line_start + relative >= offset {
                break;
            }
            character = character.saturating_add(ch.len_utf16() as u32);
        }
        Position {
            line: line as u32,
            character,
        }
    }

    /// Converts an LSP position (UTF-16 code units) into a byte offset.
    /// Returns `None` when the line does not exist. Columns past the end of
    /// the line clamp to the line end; columns splitting a surrogate pair
    /// snap down to the character start, so the result is always a character
    /// boundary.
    pub fn to_offset(&self, source: &str, position: Position) -> Option<usize> {
        let (line_start, text) = self.line_bytes(source, position.line as usize)?;
        let mut consumed: u32 = 0;
        for (relative, ch) in text.char_indices() {
            let width = ch.len_utf16() as u32;
            if position.character < consumed + width {
                // Exactly at this character's start, or splitting it (for
                // example inside a surrogate pair): resolve to its start so
                // the result is always a character boundary.
                return Some(line_start + relative);
            }
            consumed = consumed.saturating_add(width);
        }
        Some(line_start + text.len())
    }

    /// Converts a byte span into an LSP range. Byte spans are the canonical
    /// diagnostic location; the lexer's scalar columns are not used.
    pub fn span_range(&self, source: &str, span: &Span) -> Range {
        let start = self.to_position(source, span.start);
        let mut end_byte = span.end.max(span.start);
        let start_byte = self.to_offset(source, start).unwrap_or(source.len());
        if end_byte <= start_byte {
            // Zero-width span: cover the next character when possible so the
            // diagnostic is visible instead of collapsing.
            end_byte = source[start_byte..]
                .chars()
                .next()
                .map(|ch| start_byte + ch.len_utf8())
                .unwrap_or(start_byte);
        }
        let end = self.to_position(source, end_byte);
        Range { start, end }
    }
}

/// Converts a byte span into an LSP range for `source`.
pub fn span_to_range(span: &Span, source: &str) -> Range {
    LineIndex::new(source).span_range(source, span)
}

#[cfg(test)]
mod tests {
    use super::LineIndex;
    use crate::protocol::Position;

    #[test]
    fn ascii_positions_round_trip() {
        let source = "app Demo {\n    body {\n        Text(\"x\")\n    }\n}\n";
        let index = LineIndex::new(source);
        assert_eq!(index.line_count(), 6);
        let offset = source.find("Text").expect("probe");
        let position = index.to_position(source, offset);
        assert_eq!((position.line, position.character), (2, 8));
        assert_eq!(index.to_offset(source, position), Some(offset));
    }

    #[test]
    fn non_ascii_columns_use_utf16_code_units() {
        // "é" is 1 UTF-16 unit and 2 bytes; "😀" is 2 units and 4 bytes.
        let source = "state café = \"x\"\nstate 😀count = 1\n";
        let index = LineIndex::new(source);
        // Byte offset of `=` on line 0: "state café " is 12 bytes.
        let line0_eq = source.find('=').expect("probe");
        let position = index.to_position(source, line0_eq);
        assert_eq!((position.line, position.character), (0, 11));
        assert_eq!(index.to_offset(source, position), Some(line0_eq));
        // `count` starts after "state 😀" (10 bytes, 8 UTF-16 units).
        let count = source.find("count").expect("probe");
        let position = index.to_position(source, count);
        assert_eq!((position.line, position.character), (1, 8));
        assert_eq!(index.to_offset(source, position), Some(count));
    }

    #[test]
    fn offsets_never_land_on_non_character_boundaries() {
        let source = "Text(\"héllo\")\n";
        let index = LineIndex::new(source);
        let e_acute = source.find('é').expect("probe");
        // Every byte in the line maps to a safe position that maps back to a
        // character boundary.
        for offset in 0..=source.len() {
            let position = index.to_position(source, offset);
            let back = index.to_offset(source, position).expect("offset");
            assert!(source.is_char_boundary(back), "offset {offset}");
            // Mid-character offsets snap down to the character start.
            if !source.is_char_boundary(offset) {
                assert_eq!(back, e_acute);
            }
        }
    }

    #[test]
    fn surrogate_pair_columns_snap_to_character_start() {
        let source = "😀x\n";
        let index = LineIndex::new(source);
        // Column 1 splits the surrogate pair; both columns 0 and 1 resolve to
        // the character start, column 2 to the byte after the emoji.
        assert_eq!(
            index.to_offset(
                source,
                Position {
                    line: 0,
                    character: 0
                }
            ),
            Some(0)
        );
        assert_eq!(
            index.to_offset(
                source,
                Position {
                    line: 0,
                    character: 1
                }
            ),
            Some(0)
        );
        assert_eq!(
            index.to_offset(
                source,
                Position {
                    line: 0,
                    character: 2
                }
            ),
            Some(4)
        );
    }

    #[test]
    fn out_of_range_positions_clamp_safely() {
        let source = "ab\n";
        let index = LineIndex::new(source);
        assert_eq!(
            index.to_offset(
                source,
                Position {
                    line: 9,
                    character: 0
                }
            ),
            None
        );
        assert_eq!(
            index.to_offset(
                source,
                Position {
                    line: 0,
                    character: 99
                }
            ),
            Some(2)
        );
        assert_eq!(
            index.to_position(source, usize::MAX),
            Position {
                line: 1,
                character: 0
            }
        );
    }

    #[test]
    fn multi_line_spans_produce_multi_line_ranges() {
        let source = "ab\ncdef\ngh\n";
        let index = LineIndex::new(source);
        let span = nexa_diagnostics::Span {
            start: 1,
            end: 9,
            line: 0,
            column: 0,
        };
        let range = index.span_range(source, &span);
        assert_eq!((range.start.line, range.start.character), (0, 1));
        assert_eq!((range.end.line, range.end.character), (2, 1));
    }
}
