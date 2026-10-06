//! Backend orchestration and shared code-generation primitives.

pub(crate) mod colors;
pub(crate) mod expressions;
pub(crate) mod features;
pub(crate) mod functions;
pub(crate) mod imports;
pub(crate) mod runtime;
pub(crate) mod state;
pub(crate) mod structs;
pub(crate) mod types;

// Re-exported so the crate root can publish it: `engine` is a private module,
// and a `pub use` cannot name a path through private modules from outside.
pub use types::kotlin_scalar_types;
pub(crate) mod utils;
pub(crate) mod value;
