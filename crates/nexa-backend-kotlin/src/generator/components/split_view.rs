use nexa_codegen::SourceWriter;
use nexa_ir::Node;

use crate::generator::{
    components::{RenderScope, render_children},
    engine::imports::{ImportContext, ImportSet},
};

pub(crate) fn imports(context: &ImportContext<'_>, imports: &mut ImportSet) {
    if context.features.uses_navigation_split_view {
        for import in [
            "androidx.activity.compose.BackHandler",
            "androidx.compose.foundation.layout.Box",
            "androidx.compose.foundation.layout.BoxWithConstraints",
            "androidx.compose.foundation.layout.Row",
            "androidx.compose.foundation.layout.fillMaxHeight",
            "androidx.compose.foundation.layout.fillMaxSize",
            "androidx.compose.foundation.layout.widthIn",
            "androidx.compose.ui.unit.dp",
        ] {
            imports.add(true, import);
        }
    }
}

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
    out.line_at(depth + 1, format_args!("detailVisible = {state},"));
    out.line_at(
        depth + 1,
        format_args!("onDetailVisibleChange = {{ {state} = it }},"),
    );
    out.line_at(depth + 1, format_args!("sidebar = {{"));
    render_children(sidebar, scope.module, scope.features, depth + 2, out);
    out.push('\n');
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth + 1, format_args!("detail = {{"));
    render_children(detail, scope.module, scope.features, depth + 2, out);
    out.push('\n');
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth, format_args!(")"));
}
