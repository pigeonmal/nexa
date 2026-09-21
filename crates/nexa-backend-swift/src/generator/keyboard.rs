use nexa_ir::{Module, Node};

use super::{components::render_children, utils::indent};

pub(super) fn render_keyboard_aware(
    children: &[Node],
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str("ScrollView(.vertical) {\n");
    render_children(children, module, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push_str("}.scrollDismissesKeyboard(.interactively)");
}
