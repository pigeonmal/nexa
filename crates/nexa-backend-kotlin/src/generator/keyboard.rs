use nexa_ir::Node;

use super::{components::render_children, features::Features, utils::indent};

pub(super) fn render_keyboard_aware(
    children: &[Node],
    module: &nexa_ir::Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str(
        "Column(modifier = Modifier.imePadding().verticalScroll(rememberScrollState())) {\n",
    );
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
