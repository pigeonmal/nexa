use nexa_ir::walk::contains_scrollable;
use nexa_ir::{Action, Module, Node};

use super::{components::render_children, controls::render_actions, utils::indent};

pub(super) fn render_refresh_control(
    _state: &str,
    children: &[Node],
    actions: &[Action],
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    if !contains_scrollable(children) {
        out.push_str("ScrollView {\n");
    } else {
        out.push_str("VStack {\n");
    }
    render_children(children, module, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
    out.push_str(".refreshable {\n");
    render_actions(actions, depth + 1, out);
    indent(out, depth);
    out.push('}');
}
