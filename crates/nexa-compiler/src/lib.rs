mod compile;
mod optimize;
mod project;
mod semantic;

pub use compile::compile;
pub use compile::{Compilation, compile_with_warnings};
pub use nexa_diagnostics::{CompileError, CompileWarning, Span};
pub use nexa_ir::Module;
pub use project::{compile_file, compile_file_with_warnings};
