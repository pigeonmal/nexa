use nexa_codegen::SourceWriter;
use nexa_ir::{KeyboardDismissMode, Module, Node, adaptive_layout::KeyboardBehavior};

use crate::generator::features::Features;
use crate::generator::{components::render_children, utils::indent};

pub(crate) fn render_keyboard_aware(
    dismiss: KeyboardDismissMode,
    behavior: KeyboardBehavior,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let has_native_scroll_owner = !nexa_ir::walk::scroll_owners(children).is_empty();
    indent(out, depth);
    out.push_str("NexaKeyboardAwarePrimitive(behavior: .");
    out.push_str(match behavior {
        KeyboardBehavior::Stay => "stay",
        KeyboardBehavior::Avoid => "avoid",
    });
    out.push_str(", hasNativeScrollOwner: ");
    out.push_str(if has_native_scroll_owner {
        "true"
    } else {
        "false"
    });
    out.push_str(", dismissMode: .");
    out.push_str(match dismiss {
        KeyboardDismissMode::Interactive => "interactively",
        KeyboardDismissMode::Never => "never",
    });
    out.push_str(") {\n");
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
