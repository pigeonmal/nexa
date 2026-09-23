use crate::protocol::{Diagnostic, DiagnosticSeverity, Position, Range};
use nexa_diagnostics::{CompileError, CompileWarning, Span};

/// Converts a Nexa source span into an LSP 0-based range.
pub fn span_to_range(span: &Span) -> Range {
    let start_line = span.line.saturating_sub(1) as u32;
    let start_char = span.column.saturating_sub(1) as u32;
    let length = span.end.saturating_sub(span.start).max(1) as u32;
    Range {
        start: Position {
            line: start_line,
            character: start_char,
        },
        end: Position {
            line: start_line,
            character: start_char + length,
        },
    }
}

/// Converts a Nexa compiler error to an LSP diagnostic.
pub fn compile_error_to_diagnostic(error: &CompileError) -> Diagnostic {
    Diagnostic {
        range: span_to_range(&error.span),
        severity: Some(DiagnosticSeverity::Error),
        message: error.message.clone(),
        source: Some("nexa".to_string()),
    }
}

/// Converts a Nexa compiler warning to an LSP diagnostic.
pub fn compile_warning_to_diagnostic(warning: &CompileWarning) -> Diagnostic {
    Diagnostic {
        range: span_to_range(&warning.span),
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
        diagnostics.push(compile_error_to_diagnostic(&err));
        return diagnostics;
    }

    // 2. Full semantic checking and warnings
    match nexa_compiler::compile_with_warnings(source) {
        Ok(compilation) => {
            for warning in compilation.warnings {
                diagnostics.push(compile_warning_to_diagnostic(&warning));
            }
        }
        Err(err) => {
            diagnostics.push(compile_error_to_diagnostic(&err));
        }
    }

    diagnostics
}
