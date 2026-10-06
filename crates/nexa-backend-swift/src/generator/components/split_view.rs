use nexa_codegen::SourceWriter;
use nexa_ir::Node;

use crate::generator::{
    components::{RenderScope, render_children},
    utils::indent,
};

pub(crate) fn render(
    detail_visible: &str,
    sidebar: &[Node],
    detail: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    out.push_str("Group {\n");
    indent(out, depth + 1);
    out.push_str("if #available(iOS 17.0, *) {\n");
    indent(out, depth + 2);
    out.push_str("NavigationSplitView(preferredCompactColumn: Binding(\n");
    indent(out, depth + 3);
    out.push_str(&format!(
        "get: {{ {} ? .detail : .sidebar }},\n",
        nexa_codegen::names::state_name(detail_visible)
    ));
    indent(out, depth + 3);
    out.push_str(&format!(
        "set: {{ {} = ($0 == .detail) }}\n",
        nexa_codegen::names::state_name(detail_visible)
    ));
    indent(out, depth + 2);
    out.push_str(") {\n");
    render_children(sidebar, scope.module, scope.features, depth + 3, out);
    out.push('\n');
    indent(out, depth + 2);
    out.push_str("} detail: {\n");
    render_children(detail, scope.module, scope.features, depth + 3, out);
    out.push('\n');
    indent(out, depth + 2);
    out.push_str("}\n");
    indent(out, depth + 1);
    out.push_str("} else if #available(iOS 16.0, *) {\n");
    indent(out, depth + 2);
    out.push_str("NavigationSplitView {\n");
    render_children(sidebar, scope.module, scope.features, depth + 3, out);
    out.push('\n');
    indent(out, depth + 2);
    out.push_str("} detail: {\n");
    render_children(detail, scope.module, scope.features, depth + 3, out);
    out.push('\n');
    indent(out, depth + 2);
    out.push_str("}\n");
    indent(out, depth + 1);
    out.push_str("} else {\n");
    indent(out, depth + 2);
    out.push_str("HStack(spacing: 0) {\n");
    render_children(sidebar, scope.module, scope.features, depth + 3, out);
    out.push('\n');
    indent(out, depth + 3);
    out.push_str("Divider()\n");
    render_children(detail, scope.module, scope.features, depth + 3, out);
    out.push('\n');
    indent(out, depth + 2);
    out.push_str("}\n");
    indent(out, depth + 1);
    out.push_str("}\n");
    indent(out, depth);
    out.push('}');
}
