#![allow(
    clippy::large_enum_variant,
    clippy::collapsible_if,
    clippy::type_complexity,
    clippy::while_let_on_iterator
)]

pub mod ast;
pub mod catalog;
mod format;
mod lexer;
mod parser;

use nexa_diagnostics::CompileError;

pub use lexer::{Kind as TokenKind, Token};

/// Tokenize Nexa source while preserving byte spans for editor integrations.
/// Comments and whitespace are intentionally omitted, as they are for the
/// parser; callers that need trivia should retain it from the source text.
pub fn tokenize(source: &str) -> Result<Vec<Token>, CompileError> {
    lexer::lex(source)
}

pub use format::{format_source, format_source_with_options};

pub fn parse(source: &str) -> Result<ast::App, CompileError> {
    parser::parse(lexer::lex(source)?)
}

pub fn parse_program(source: &str) -> Result<ast::Program, CompileError> {
    parser::parse_program(lexer::lex(source)?)
}

pub fn parse_config(source: &str) -> Result<ast::Config, CompileError> {
    parser::parse_config(lexer::lex(source)?)
}
