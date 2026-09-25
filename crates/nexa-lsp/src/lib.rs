pub mod completions;
pub mod diagnostics;
pub mod hover;
pub mod line_index;
pub mod protocol;
pub mod server;
pub mod symbols;

pub use completions::get_completions;
pub use diagnostics::check_source;
pub use hover::get_hover;
pub use line_index::{span_to_range, LineIndex};
pub use protocol::*;
pub use server::LspServer;
pub use symbols::get_document_symbols;
