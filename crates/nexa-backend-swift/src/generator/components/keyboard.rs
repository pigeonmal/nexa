use nexa_codegen::SourceWriter;
use nexa_ir::{KeyboardDismissMode, Node, adaptive_layout::KeyboardBehavior};

use super::RenderScope;
use crate::generator::{components::render_children, utils::indent};

pub(crate) struct KeyboardAwareOptions {
    pub dismiss: KeyboardDismissMode,
    pub behavior: KeyboardBehavior,
    pub hide_horizontal_scrollbar: bool,
    pub hide_vertical_scrollbar: bool,
}

pub(crate) fn render_keyboard_aware(
    options: KeyboardAwareOptions,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let KeyboardAwareOptions {
        dismiss,
        behavior,
        hide_horizontal_scrollbar,
        hide_vertical_scrollbar,
    } = options;
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
    out.push_str(&format!(
        ", hideHorizontalScrollbar: {hide_horizontal_scrollbar}, hideVerticalScrollbar: {hide_vertical_scrollbar}) {{\n"
    ));
    render_children(children, scope.module, scope.features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
