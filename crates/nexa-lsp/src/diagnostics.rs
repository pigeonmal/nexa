use crate::line_index::LineIndex;
use crate::protocol::{Diagnostic, DiagnosticSeverity, Range};
use nexa_diagnostics::{CompileError, CompileWarning, Span};

/// Converts a Nexa source span into an LSP 0-based range for `source`.
///
/// Byte offsets are the canonical location: the span is resolved through a
/// [`LineIndex`] into UTF-16 columns, so multi-line spans and non-ASCII text
/// map correctly. The lexer's scalar line/column are display hints only and
/// are not used here.
pub fn span_to_range(span: &Span, source: &str) -> Range {
    LineIndex::new(source).span_range(source, span)
}

/// Converts a Nexa compiler error to an LSP diagnostic for `source`.
pub fn compile_error_to_diagnostic(error: &CompileError, source: &str) -> Diagnostic {
    Diagnostic {
        range: span_to_range(&error.span, source),
        severity: Some(DiagnosticSeverity::Error),
        message: error.message.clone(),
        source: Some("nexa".to_string()),
    }
}

/// Converts a Nexa compiler warning to an LSP diagnostic for `source`.
pub fn compile_warning_to_diagnostic(warning: &CompileWarning, source: &str) -> Diagnostic {
    Diagnostic {
        range: span_to_range(&warning.span, source),
        severity: Some(DiagnosticSeverity::Warning),
        message: warning.message.clone(),
        source: Some("nexa".to_string()),
    }
}

/// Runs full syntax and semantic analysis on the provided source and returns all diagnostics.
pub fn check_source(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // 1. Syntax analysis
    if let Err(err) = nexa_syntax::parse_program(source) {
        diagnostics.push(compile_error_to_diagnostic(&err, source));
        return diagnostics;
    }

    // 2. Full semantic checking and warnings
    match nexa_compiler::compile_with_warnings(source) {
        Ok(compilation) => {
            for warning in compilation.warnings {
                diagnostics.push(compile_warning_to_diagnostic(&warning, source));
            }
        }
        Err(err) => {
            diagnostics.push(compile_error_to_diagnostic(&err, source));
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::{check_source, span_to_range};
    use nexa_diagnostics::Span;

    #[test]
    fn ranges_use_utf16_columns_for_non_ascii_lines() {
        // Line 0 is `state café = 0`: `café` starts at byte 6, UTF-16 column 6.
        let source = "state café = 0\n";
        let error_at = source.find("café").expect("probe");
        let range = span_to_range(
            &Span {
                start: error_at,
                end: error_at + "café".len(),
                line: 1,
                column: 7,
            },
            source,
        );
        assert_eq!((range.start.line, range.start.character), (0, 6));
        assert_eq!((range.end.line, range.end.character), (0, 10));
    }

    #[test]
    fn ranges_span_multiple_lines_from_byte_offsets() {
        let source = "app P {\n    body {\n        Text(\"x\")\n    }\n}\n";
        let start = source.find("body").expect("probe");
        let end = source.find('}').expect("probe") + 1;
        let range = span_to_range(
            &Span {
                start,
                end,
                line: 2,
                column: 5,
            },
            source,
        );
        assert_eq!((range.start.line, range.start.character), (1, 4));
        // The first `}` closes the Text-adjacent block on line 3.
        assert_eq!((range.end.line, range.end.character), (3, 5));
    }

    #[test]
    fn zero_width_spans_cover_one_character() {
        let source = "ab\n";
        let range = span_to_range(
            &Span {
                start: 1,
                end: 1,
                line: 1,
                column: 2,
            },
            source,
        );
        assert_eq!((range.start.line, range.start.character), (0, 1));
        assert_eq!((range.end.line, range.end.character), (0, 2));
    }

    #[test]
    fn diagnostics_point_at_non_ascii_errors() {
        // `ü` cannot start an identifier, so the lexer reports a zero-width
        // span on it. The range must cover exactly that character in UTF-16
        // columns instead of collapsing or overshooting by byte length.
        let source = "app P {\n    body {\n        ünknown()\n    }\n}\n";
        let diagnostics = check_source(source);
        assert!(!diagnostics.is_empty());
        assert!(diagnostics[0].message.contains('ü'));
        let range = diagnostics[0].range;
        assert_eq!((range.start.line, range.start.character), (2, 8));
        assert_eq!((range.end.line, range.end.character), (2, 9));
    }
}
