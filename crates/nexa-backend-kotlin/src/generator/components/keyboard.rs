use nexa_codegen::SourceWriter;
use nexa_ir::{KeyboardDismissMode, Node, adaptive_layout::KeyboardBehavior};

use crate::generator::{
    components::render_children, engine::features::Features, engine::utils::indent,
};

use crate::generator::engine::imports::ImportSet;

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
    imports.add(features.uses_keyboard_aware, "androidx.compose.ui.Modifier");
}

pub(crate) fn render_keyboard_aware(
    dismiss: KeyboardDismissMode,
    behavior: KeyboardBehavior,
    children: &[Node],
    module: &nexa_ir::Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
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
    out.push_str(") {\n");
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}
