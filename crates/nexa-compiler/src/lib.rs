mod compile;
mod optimize;
pub use optimize::OptimizationReport;
mod project;
mod semantic;
pub use semantic::core_api;
pub mod testing;

pub use compile::compile;
pub use compile::{
    Compilation, Target, compile_for_target, compile_with_warnings,
    compile_with_warnings_for_target,
};
pub use nexa_diagnostics::{CompileError, CompileWarning, Span, WarningCode};

/// Indexed declarations the incremental DevRuntime compiler proved unchanged
/// from its previous successful compilation.
#[derive(Clone, Debug, Default)]
pub struct DevChangeHint {
    pub unchanged_function_indices: Vec<usize>,
    pub unchanged_screen_indices: Vec<usize>,
}
use nexa_ir::adaptive_layout::{ScrollOwnerDescriptor, ScrollOwnerResolution};
pub use nexa_ir::{Module, adaptive_layout::ScrollOwnerId};
use nexa_ir::{Node, adaptive_layout::LayoutEdge};
pub use nexa_plugin_compiler_api::{ExtensionContext, NativeCall, PluginCompilerExtension};
pub use project::{
    IncrementalProjectCompiler, ProjectCompileStats,
    compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots, compile_file,
    compile_file_for_target, compile_file_with_optimization_report_for_targets_and_plugin_roots,
    compile_file_with_warnings, compile_file_with_warnings_for_target,
    compile_file_with_warnings_for_targets,
    compile_file_with_warnings_for_targets_and_plugin_roots,
};

/// Resolve the scroll owner an edge-attached view can affect. No owner is a
/// valid result because edge placement remains meaningful without scrolling.
/// Ambiguous scopes require an explicit typed `ScrollRef` selection.
pub fn resolve_scroll_owner(
    nodes: &[Node],
    edge: LayoutEdge,
    explicit: Option<ScrollOwnerId>,
    span: Span,
) -> Result<Option<ScrollOwnerDescriptor>, CompileError> {
    match nexa_ir::walk::resolve_scroll_owner(nodes, edge, explicit) {
        ScrollOwnerResolution::None => Ok(None),
        ScrollOwnerResolution::Unique(owner) => Ok(Some(owner)),
        ScrollOwnerResolution::RouteHost => Ok(None),
        ScrollOwnerResolution::Ambiguous(owners) => Err(CompileError::new(
            span,
            format!(
                "edge-attached content has {} compatible scroll owners; provide a typed ScrollRef to select one",
                owners.len()
            ),
        )),
        ScrollOwnerResolution::InvalidReference(id) => Err(CompileError::new(
            span,
            format!(
                "ScrollRef {} does not identify a scroll owner compatible with the selected edge",
                id.0
            ),
        )),
    }
}

/// Render the compiler-owned built-in API signatures for generated reference
/// documentation.
pub fn render_core_api_reference() -> String {
    semantic::render_core_api_reference()
}
