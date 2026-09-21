use nexa_ir::{Module, Node};

use super::{
    components::render_children,
    utils::{indent, swift_string},
};

pub(super) fn render_link(
    url: &str,
    children: &[Node],
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str(&format!(
        "Link(destination: URL(string: {})!) {{\n",
        swift_string(url)
    ));
    render_children(children, module, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
