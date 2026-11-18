use nexa_codegen::SourceWriter;
use nexa_ir::{KeyboardDismissMode, KeyboardEventHandler, Node, adaptive_layout::KeyboardBehavior};

use crate::generator::{
    components::render_children, engine::features::Features, engine::utils::indent,
};

use crate::generator::engine::imports::ImportSet;

pub(crate) struct KeyboardAwareOptions<'a> {
    pub dismiss: KeyboardDismissMode,
    pub behavior: KeyboardBehavior,
    pub focused_input_offset: f32,
    pub toolbar: Option<&'a [Node]>,
    pub on_show: Option<&'a KeyboardEventHandler>,
    pub on_hide: Option<&'a KeyboardEventHandler>,
}

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_keyboard_aware || features.uses_refresh_scroll,
        "androidx.compose.foundation.rememberScrollState",
    );
    imports.add(
        features.uses_keyboard_aware || features.uses_refresh_scroll,
        "androidx.compose.foundation.verticalScroll",
    );
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.layout.imePadding",
    );
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.layout.isImeVisible",
    );
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.layout.imeNestedScroll",
    );
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.layout.padding",
    );
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.layout.fillMaxSize",
    );
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.ui.viewinterop.AndroidView",
    );
    imports.add(features.uses_keyboard_aware, "androidx.compose.ui.unit.dp");
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.layout.size",
    );
    imports.add(features.uses_keyboard_aware, "androidx.compose.ui.Modifier");
}

pub(crate) fn render_keyboard_aware(
    options: KeyboardAwareOptions<'_>,
    children: &[Node],
    module: &nexa_ir::Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let KeyboardAwareOptions {
        dismiss,
        behavior,
        focused_input_offset,
        toolbar,
        on_show,
        on_hide,
    } = options;
    let has_native_scroll_owner = !nexa_ir::walk::scroll_owners(children).is_empty();
    indent(out, depth);
    if has_native_scroll_owner {
        out.push_str("NexaKeyboardAwareOwnerPrimitive(interactiveDismiss = ");
    } else {
        out.push_str("NexaKeyboardAwareColumnPrimitive(interactiveDismiss = ");
    }
    out.push_str(if matches!(dismiss, KeyboardDismissMode::Interactive) {
        "true"
    } else {
        "false"
    });
    out.push_str(", avoidIme = ");
    out.push_str(if matches!(behavior, KeyboardBehavior::Avoid) {
        "true"
    } else {
        "false"
    });
    out.push_str(&format!(
        ", focusedInputOffset = {focused_input_offset}f, hasToolbar = {}, onShow = ",
        toolbar.is_some()
    ));
    render_event_handler(on_show, depth + 1, out);
    out.push_str(", onHide = ");
    render_event_handler(on_hide, depth + 1, out);
    out.push_str(", content = {\n");
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push_str("}, toolbar = {\n");
    if let Some(toolbar) = toolbar {
        render_children(toolbar, module, features, depth + 1, out);
    }
    indent(out, depth);
    out.push_str("})");
}

fn render_event_handler(
    handler: Option<&KeyboardEventHandler>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let Some(handler) = handler else {
        out.push_str("null");
        return;
    };
    let name = nexa_codegen::names::state_name(&handler.parameter);
    out.push_str(&format!("{{ {name} ->\n"));
    crate::generator::components::controls::render_actions(&handler.actions, depth + 1, out);
    indent(out, depth);
    out.push('}');
}
