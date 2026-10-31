use nexa_codegen::SourceWriter;
use nexa_ir::{Node, adaptive_layout::ScrollHeaderBehavior};

use crate::generator::{
    components::render_children,
    engine::{features::Features, utils::indent},
};

use crate::generator::engine::imports::{ImportContext, ImportSet};

pub(crate) fn imports(context: &ImportContext<'_>, imports: &mut ImportSet) {
    let enabled = context.features.uses_scroll_header || context.include_all_shared_primitives;
    for import in [
        "androidx.compose.animation.core.animateDpAsState",
        "androidx.compose.animation.core.tween",
        "androidx.compose.foundation.layout.fillMaxSize",
        "androidx.compose.foundation.layout.fillMaxWidth",
        "androidx.compose.ui.draw.clipToBounds",
        "androidx.compose.ui.layout.onSizeChanged",
        "androidx.compose.ui.unit.dp",
    ] {
        imports.add(enabled, import);
    }
}

pub(crate) fn render_scroll_header(
    plan: nexa_ir::adaptive_layout::ScrollHeaderPlan,
    header: &[Node],
    children: &[Node],
    module: &nexa_ir::Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let behavior = match plan.behavior {
        ScrollHeaderBehavior::Pinned => "Pinned",
        ScrollHeaderBehavior::Collapse => "Collapse",
        ScrollHeaderBehavior::HideOnScroll => "HideOnScroll",
    };
    let collapsed_height = plan.collapsed_height.unwrap_or(56.0);
    indent(out, depth);
    out.push_str(&format!(
        "NexaAdaptiveScrollHeaderPrimitive(behavior = \"{behavior}\", collapsedHeight = {collapsed_height}.dp, header = {{\n"
    ));
    render_children(header, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push_str("}, content = {\n");
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push_str("})");
}
