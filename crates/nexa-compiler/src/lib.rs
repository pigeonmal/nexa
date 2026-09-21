mod compile;
mod semantic;

pub use compile::compile;
pub use nexa_diagnostics::{CompileError, Span};
pub use nexa_ir::Module;
