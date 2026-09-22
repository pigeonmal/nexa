use nexa_ir::{Expr, Module, Node};

use super::{
    components::render_children,
    expressions::expression,
    utils::{indent, swift_string},
};

pub(super) fn render_link(
    url: &Expr,
    children: &[Node],
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    if let Expr::String(value) = url {
        indent(out, depth);
        out.push_str(&format!(
            "Link(destination: URL(string: {})!) {{\n",
            swift_string(value)
        ));
        render_children(children, module, depth + 1, out);
        out.push('\n');
        indent(out, depth);
        out.push('}');
        return;
    }

    indent(out, depth);
    out.push_str(&format!(
        "if let nexaLinkURL = URL(string: {}) {{\n",
        expression(url)
    ));
    indent(out, depth + 1);
    out.push_str("Link(destination: nexaLinkURL) {\n");
    render_children(children, module, depth + 2, out);
    out.push('\n');
    indent(out, depth + 1);
    out.push('}');
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
