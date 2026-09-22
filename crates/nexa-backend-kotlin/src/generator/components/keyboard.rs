use nexa_ir::{KeyboardDismissMode, Node};

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
        features.uses_keyboard_interactive,
        "androidx.compose.foundation.layout.imeNestedScroll",
    );
}

pub(crate) fn render_keyboard_aware(
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
