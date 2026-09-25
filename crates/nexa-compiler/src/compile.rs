use nexa_diagnostics::{CompileError, CompileWarning};
use nexa_ir::Module;

use crate::semantic;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Swift,
    Kotlin,
    All,
}

pub struct Compilation {
    pub module: Module,
    pub warnings: Vec<CompileWarning>,
    /// Manifest-resolved plugin declarations in source order. Scaffolding
    /// derives its `PluginPackage` model from these; the IR keeps only
    /// plugin identity (see `nexa_ir::Plugin`).
    pub plugins: Vec<nexa_syntax::ast::PluginDecl>,
}

/// Runs lexing, parsing, semantic analysis, and lowering to the common IR.
pub fn compile(source: &str) -> Result<Module, CompileError> {
    Ok(compile_with_target(source, Target::All)?.module)
}

/// Runs compilation and returns non-fatal diagnostics alongside the typed IR.
pub fn compile_with_warnings(source: &str) -> Result<Compilation, CompileError> {
    compile_with_target(source, Target::All)
}

pub fn compile_for_target(source: &str, target: Target) -> Result<Module, CompileError> {
    Ok(compile_with_target(source, target)?.module)
}

pub fn compile_with_warnings_for_target(
    source: &str,
    target: Target,
) -> Result<Compilation, CompileError> {
    compile_with_target(source, target)
}

fn compile_with_target(source: &str, target: Target) -> Result<Compilation, CompileError> {
    let app = nexa_syntax::parse(source)?;
    let plugins = app.plugins.clone();
    let (module, warnings) = semantic::lower_with_warnings(app, target)?;
    Ok(Compilation {
        module,
        warnings,
        plugins,
    })
}
