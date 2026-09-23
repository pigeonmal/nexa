#![allow(
    clippy::large_enum_variant,
    clippy::collapsible_if,
    clippy::type_complexity,
    clippy::while_let_on_iterator
)]

pub mod ast;
mod lexer;
mod parser;

use nexa_diagnostics::CompileError;

pub fn parse(source: &str) -> Result<ast::App, CompileError> {
    parser::parse(lexer::lex(source)?)
}

pub fn parse_program(source: &str) -> Result<ast::Program, CompileError> {
    parser::parse_program(lexer::lex(source)?)
}

pub fn parse_config(source: &str) -> Result<ast::Config, CompileError> {
    parser::parse_config(lexer::lex(source)?)
}
