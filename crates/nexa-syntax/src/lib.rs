pub mod ast;
mod lexer;
mod parser;

use nexa_diagnostics::CompileError;

pub fn parse(source: &str) -> Result<ast::App, CompileError> {
    parser::parse(lexer::lex(source)?)
}
