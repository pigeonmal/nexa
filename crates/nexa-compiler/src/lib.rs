mod compile;
mod project;
mod semantic;

pub use compile::compile;
pub use nexa_diagnostics::{CompileError, Span};
pub use nexa_ir::Module;
pub use project::compile_file;
