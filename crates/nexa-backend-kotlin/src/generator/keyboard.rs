use nexa_ir::{KeyboardDismissMode, Node};

use super::{components::render_children, features::Features, utils::indent};

pub(super) fn render_keyboard_aware(
    dismiss: KeyboardDismissMode,
    children: &[Node],
    module: &nexa_ir::Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str("Column(modifier = Modifier.imePadding()");
    if matches!(dismiss, KeyboardDismissMode::Interactive) {
        out.push_str(".imeNestedScroll()");
    }
    out.push_str(".verticalScroll(rememberScrollState())) {\n");
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
