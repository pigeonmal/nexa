use nexa_codegen::SourceWriter;
use nexa_ir::{Expr, Module, Node};

use crate::generator::{
    components::render_children,
    expressions::expression,
    utils::{indent, swift_string},
};

use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.uses_link, "Foundation");
}

pub(crate) fn render_link(
    url: &Expr,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    if let Expr::String(value) = url {
        out.line_at(
            depth,
            format_args!(
                "Link(destination: URL(string: {})!) {{",
                swift_string(value)
            ),
        );
        render_children(children, module, features, depth + 1, out);
        out.push('\n');
        indent(out, depth);
        out.push('}');
        return;
    }

    out.line_at(
        depth,
        format_args!("if let nexaLinkURL = URL(string: {}) {{", expression(url)),
    );
    indent(out, depth + 1);
    out.push_str("Link(destination: nexaLinkURL) {\n");
    render_children(children, module, features, depth + 2, out);
    out.push('\n');
    indent(out, depth + 1);
    out.push('}');
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
