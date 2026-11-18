use nexa_codegen::SourceWriter;
use nexa_ir::{KeyboardDismissMode, KeyboardEventHandler, Node, adaptive_layout::KeyboardBehavior};

use super::RenderScope;
use crate::generator::{components::render_children, utils::indent};

pub(crate) struct KeyboardAwareOptions<'a> {
    pub dismiss: KeyboardDismissMode,
    pub behavior: KeyboardBehavior,
    pub hide_horizontal_scrollbar: bool,
    pub hide_vertical_scrollbar: bool,
    pub focused_input_offset: f32,
    pub on_show: Option<&'a KeyboardEventHandler>,
    pub on_hide: Option<&'a KeyboardEventHandler>,
}

pub(crate) fn render_keyboard_aware(
    options: KeyboardAwareOptions<'_>,
    children: &[Node],
    toolbar: Option<&[Node]>,
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let KeyboardAwareOptions {
        dismiss,
        behavior,
        hide_horizontal_scrollbar,
        hide_vertical_scrollbar,
        focused_input_offset,
        on_show,
        on_hide,
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
        ", hideHorizontalScrollbar: {hide_horizontal_scrollbar}, hideVerticalScrollbar: {hide_vertical_scrollbar}, focusedInputOffset: {focused_input_offset}, hasToolbar: {}, onShow: ",
        toolbar.is_some()
    ));
    render_event_handler(on_show, depth + 1, out);
    out.push_str(", onHide: ");
    render_event_handler(on_hide, depth + 1, out);
    out.push_str(") {\n");
    render_children(children, scope.module, scope.features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push_str("} toolbar: {\n");
    if let Some(toolbar) = toolbar {
        render_children(toolbar, scope.module, scope.features, depth + 1, out);
    } else {
        indent(out, depth + 1);
        out.push_str("EmptyView()\n");
    }
    indent(out, depth);
    out.push('}');
}

fn render_event_handler(
    handler: Option<&KeyboardEventHandler>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let Some(handler) = handler else {
        out.push_str("nil");
        return;
    };
    out.push_str(&format!(
        "{{ {} in\n",
        nexa_codegen::names::state_name(&handler.parameter)
    ));
    super::controls::render_actions(&handler.actions, depth + 1, out);
    indent(out, depth);
    out.push('}');
}
