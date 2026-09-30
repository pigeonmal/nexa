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
    out.line_at(
        depth + 2,
        format_args!("text = {{ Text({}) }},", expression(message)),
    );
    indent(out, depth + 2);
    out.push_str("confirmButton = {\n");
    render_children(children, scope.module, scope.features, depth + 3, out);
    out.push('\n');
    indent(out, depth + 2);
    out.push_str("},\n");
    indent(out, depth + 1);
    out.push_str(")\n");
    indent(out, depth);
    out.push('}');
}
