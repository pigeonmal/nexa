use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Expr, Module, Node};

use crate::generator::{
    components::render_children, engine::features::Features, expressions::text_expression,
    utils::indent,
};

pub(crate) fn render_dialog(
    state: &str,
    title: &Expr,
    message: &Expr,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let state_name = state_name(state);
    out.line_at(
        depth,
        format_args!(
            "EmptyView().alert(Text({}), isPresented: ${state_name}) {{",
            text_expression(title)
        ),
    );
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push_str("} message: {\n");
    out.line_at(
        depth + 1,
        format_args!("Text({})", text_expression(message)),
    );
    indent(out, depth);
    out.push('}');
}
