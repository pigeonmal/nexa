use nexa_diagnostics::CompileError;
use nexa_ir::Module;

use crate::semantic;

/// Runs lexing, parsing, semantic analysis, and lowering to the common IR.
pub fn compile(source: &str) -> Result<Module, CompileError> {
    let app = nexa_syntax::parse(source)?;
    semantic::lower(app)
}
