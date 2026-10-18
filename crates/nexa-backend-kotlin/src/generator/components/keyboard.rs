use nexa_codegen::SourceWriter;
use nexa_ir::{KeyboardDismissMode, Node};

use crate::generator::{
    components::render_children, engine::features::Features, engine::utils::indent,
};

use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.gestures.awaitEachGesture",
    );
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.foundation.gestures.awaitFirstDown",
    );
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
        "androidx.compose.ui.input.pointer.PointerEventPass",
    );
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.ui.input.pointer.pointerInput",
    );
    imports.add(features.uses_keyboard_aware, "androidx.compose.ui.Modifier");
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.ui.platform.LocalFocusManager",
    );
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.ui.platform.LocalViewConfiguration",
    );
    imports.add(
        features.uses_keyboard_aware,
        "androidx.compose.runtime.rememberUpdatedState",
    );
}

pub(crate) fn render_keyboard_aware(
    dismiss: KeyboardDismissMode,
    children: &[Node],
    module: &nexa_ir::Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    out.push_str("NexaKeyboardAwareColumnPrimitive(interactiveDismiss = ");
    out.push_str(if matches!(dismiss, KeyboardDismissMode::Interactive) {
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
