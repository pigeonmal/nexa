use nexa_ir::{KeyboardDismissMode, Module, Node};

use super::{components::render_children, utils::indent};

pub(super) fn render_keyboard_aware(
    dismiss: KeyboardDismissMode,
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
    let mode = match dismiss {
        KeyboardDismissMode::Interactive => "interactively",
        KeyboardDismissMode::Never => "never",
    };
    out.push_str(&format!("}}.scrollDismissesKeyboard(.{mode})"));
}
