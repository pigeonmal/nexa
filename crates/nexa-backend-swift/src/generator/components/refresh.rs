use nexa_codegen::SourceWriter;
use nexa_ir::walk::contains_scrollable;
use nexa_ir::{Action, Module, Node};

use crate::generator::features::Features;
use crate::generator::{components::render_children, controls::render_actions, utils::indent};

pub(crate) fn render_refresh_control(
    _state: &str,
    children: &[Node],
    actions: &[Action],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    if !contains_scrollable(children) {
        out.push_str("ScrollView {\n");
    } else {
        out.push_str("VStack {\n");
    }
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
    out.push_str(".refreshable {\n");
    render_actions(actions, depth + 1, out);
    indent(out, depth);
    out.push('}');
}
