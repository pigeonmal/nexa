use nexa_codegen::names::state_name;
use nexa_ir::{Module, Node};

use crate::generator::features::Features;
use crate::generator::{components::render_children, utils::indent};

pub(crate) fn render_bottom_sheet(
    state: &str,
    partial: bool,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str(&format!(
        "EmptyView().sheet(isPresented: ${}) {{\n",
        state_name(state)
    ));
    render_children(children, module, features, depth + 1, out);
    if partial {
        out.push('\n');
        indent(out, depth + 1);
        out.push_str(".presentationDetents([.medium, .large])");
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
