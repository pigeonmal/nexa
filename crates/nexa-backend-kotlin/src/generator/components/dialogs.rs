use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Expr, Node};

use crate::generator::{
    components::render_children,
    engine::{features::Features, imports::ImportSet},
    expressions::expression,
    utils::indent,
};

use super::RenderScope;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_dialog,
        "androidx.compose.material3.AlertDialog",
    );
    imports.add(
        features.uses_dialog,
        "androidx.compose.foundation.layout.Column",
    );
}

pub(crate) fn render_dialog(
    state: &str,
    title: &Expr,
    message: &Expr,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let state_name = state_name(state);
    let has_message = !matches!(message, Expr::String(value) if value.is_empty());
    let inputs = children
        .iter()
        .filter(|child| matches!(child, Node::TextInput { .. }))
        .cloned()
        .collect::<Vec<_>>();
    let actions = children
        .iter()
        .filter(|child| matches!(child, Node::Button { .. }))
        .cloned()
        .collect::<Vec<_>>();
    out.line_at(depth, format_args!("if ({state_name}) {{"));
    indent(out, depth + 1);
    out.push_str("AlertDialog(\n");
    out.line_at(
        depth + 2,
        format_args!("onDismissRequest = {{ {state_name} = false }},"),
    );
    out.line_at(
        depth + 2,
        format_args!("title = {{ Text({}) }},", expression(title)),
    );
    if inputs.is_empty() {
        out.line_at(
            depth + 2,
            format_args!("text = {{ Text({}) }},", expression(message)),
        );
    } else {
        out.line_at(depth + 2, format_args!("text = {{ Column {{"));
        if has_message {
            out.line_at(depth + 3, format_args!("Text({})", expression(message)));
        }
        render_children(&inputs, scope.module, scope.features, depth + 3, out);
        out.line_at(depth + 2, format_args!("}} }},"));
    }
    indent(out, depth + 2);
    out.push_str("confirmButton = {\n");
    render_children(&actions, scope.module, scope.features, depth + 3, out);
    out.push('\n');
    indent(out, depth + 2);
    out.push_str("},\n");
    indent(out, depth + 1);
    out.push_str(")\n");
    indent(out, depth);
    out.push('}');
}

pub(crate) fn render_confirmation_dialog(
    state: &str,
    title: &Expr,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let state = state_name(state);
    out.line_at(depth, format_args!("if ({state}) {{"));
    out.line_at(depth + 1, format_args!("AlertDialog("));
    out.line_at(
        depth + 2,
        format_args!("onDismissRequest = {{ {state} = false }},"),
    );
    out.line_at(
        depth + 2,
        format_args!("title = {{ Text({}) }},", expression(title)),
    );
    out.line_at(depth + 2, format_args!("confirmButton = {{"));
    out.line_at(depth + 3, format_args!("Column {{"));
    render_children(children, scope.module, scope.features, depth + 4, out);
    out.push('\n');
    indent(out, depth + 3);
    out.push_str("}\n");
    out.line_at(depth + 2, format_args!("}},"));
    out.line_at(depth + 1, format_args!(")"));
    out.line_at(depth, format_args!("}}"));
}
