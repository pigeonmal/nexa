mod compile;
mod optimize;
mod project;
mod semantic;

pub use compile::compile;
pub use compile::{
    Compilation, Target, compile_for_target, compile_with_warnings,
    compile_with_warnings_for_target,
};
pub use nexa_diagnostics::{CompileError, CompileWarning, Span};
pub use nexa_ir::Module;
pub use project::{
    compile_file, compile_file_for_target, compile_file_with_warnings,
    compile_file_with_warnings_for_target, compile_file_with_warnings_for_targets,
};
