use nexa_diagnostics::{CompileError, CompileWarning};
use nexa_ir::Module;
use nexa_plugin_compiler_api::PluginCompilerExtension;

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
    /// Typed `.nx` test blocks for host-side `nexa test` execution.
    pub tests: crate::testing::TestSuite,
    /// Manifest-resolved plugin declarations in source order. Scaffolding
    /// derives its `PluginPackage` model from these; the IR keeps only
    /// plugin identity (see `nexa_ir::Plugin`).
    pub plugins: Vec<nexa_syntax::ast::PluginDecl>,
}

impl Compilation {
    /// Runs in-process plugin compiler extensions against the typed module.
    ///
    /// This is an opt-in pass so applications without extensions pay no
    /// traversal cost and keep the ordinary compiler API unchanged.
    pub fn validate_with_extensions(
        &self,
        extensions: &[&dyn PluginCompilerExtension],
    ) -> Result<(), Vec<CompileError>> {
        nexa_plugin_compiler_api::inspect_module(&self.module, extensions)
    }
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
    let (module, warnings, tests) = semantic::lower_with_warnings(app, target)?;
    Ok(Compilation {
        module,
        warnings,
        tests,
        plugins,
    })
}
