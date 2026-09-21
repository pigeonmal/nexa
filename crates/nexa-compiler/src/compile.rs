use nexa_diagnostics::{CompileError, CompileWarning};
use nexa_ir::Module;

use crate::semantic;

pub struct Compilation {
    pub module: Module,
    pub warnings: Vec<CompileWarning>,
}

/// Runs lexing, parsing, semantic analysis, and lowering to the common IR.
pub fn compile(source: &str) -> Result<Module, CompileError> {
    Ok(compile_with_warnings(source)?.module)
}

/// Runs compilation and returns non-fatal diagnostics alongside the typed IR.
pub fn compile_with_warnings(source: &str) -> Result<Compilation, CompileError> {
    let app = nexa_syntax::parse(source)?;
    let (module, warnings) = semantic::lower_with_warnings(app)?;
    Ok(Compilation { module, warnings })
}
