use nexa_codegen::SourceWriter;
use nexa_ir::Node;

use crate::generator::{
    components::{RenderScope, render_children},
    engine::imports::{ImportContext, ImportSet},
    utils::indent,
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
    indent(out, depth);
    out.push_str("BoxWithConstraints(Modifier.fillMaxSize()) {\n");
    indent(out, depth + 1);
    out.push_str("if (maxWidth >= 840.dp) {\n");
    indent(out, depth + 2);
    out.push_str("Row(Modifier.fillMaxSize()) {\n");
    indent(out, depth + 3);
    out.push_str("Box(Modifier.widthIn(min = 260.dp, max = 360.dp).fillMaxHeight()) {\n");
    render_children(sidebar, scope.module, scope.features, depth + 4, out);
    out.push('\n');
    indent(out, depth + 3);
    out.push_str("}\n");
    indent(out, depth + 3);
    out.push_str("Box(Modifier.weight(1f).fillMaxHeight()) {\n");
    render_children(detail, scope.module, scope.features, depth + 4, out);
    out.push('\n');
    indent(out, depth + 3);
    out.push_str("}\n");
    indent(out, depth + 2);
    out.push_str("}\n");
    indent(out, depth + 1);
    out.push_str("} else {\n");
    indent(out, depth + 2);
    out.push_str(&format!(
        "BackHandler(enabled = {state}) {{ {state} = false }}\n"
    ));
    indent(out, depth + 2);
    out.push_str(&format!("if ({state}) {{\n"));
    render_children(detail, scope.module, scope.features, depth + 3, out);
    out.push('\n');
    indent(out, depth + 2);
    out.push_str("} else {\n");
    render_children(sidebar, scope.module, scope.features, depth + 3, out);
    out.push('\n');
    indent(out, depth + 2);
    out.push_str("}\n");
    indent(out, depth + 1);
    out.push_str("}\n");
    indent(out, depth);
    out.push('}');
}
