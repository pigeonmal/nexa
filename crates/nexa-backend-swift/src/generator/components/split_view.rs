use nexa_codegen::SourceWriter;
use nexa_ir::Node;

use crate::generator::components::{RenderScope, render_children};

pub(crate) fn render(
    detail_visible: &str,
    sidebar: &[Node],
    detail: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let state = nexa_codegen::names::state_name(detail_visible);
    out.line_at(depth, format_args!("NexaNavigationSplitViewPrimitive("));
    out.line_at(depth + 1, format_args!("detailVisible: ${state},"));
    out.line_at(depth + 1, format_args!("sidebar: {{"));
    render_children(sidebar, scope.module, scope.features, depth + 2, out);
    out.push('\n');
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth + 1, format_args!("detail: {{"));
    render_children(detail, scope.module, scope.features, depth + 2, out);
    out.push('\n');
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth, format_args!(")"));
}
