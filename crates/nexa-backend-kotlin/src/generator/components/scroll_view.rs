use nexa_codegen::SourceWriter;
use nexa_ir::{Node, ViewStyle, adaptive_layout::ScrollAxis};

use crate::generator::{
    components::render_children,
    engine::{expressions::expression, imports::ImportSet},
    features::Features,
    utils::indent,
};

use super::RenderScope;

pub(crate) fn imports(
    features: &Features,
    include_all_shared_primitives: bool,
    imports: &mut ImportSet,
) {
    // The generated helper is shared with DevRuntime and emitted in every
    // host, so its imports are present even when this app has no instance.
    let enabled = features.uses_scroll_view || include_all_shared_primitives;
    imports.add(enabled, "androidx.compose.foundation.horizontalScroll");
    imports.add(enabled, "androidx.compose.foundation.verticalScroll");
    imports.add(enabled, "androidx.compose.foundation.rememberScrollState");
    imports.add(enabled, "androidx.compose.foundation.layout.Column");
    imports.add(enabled, "androidx.compose.foundation.layout.Row");
    imports.add(enabled, "androidx.compose.foundation.layout.Box");
    imports.add(enabled, "androidx.compose.ui.Alignment");
    imports.add(enabled, "androidx.compose.foundation.layout.fillMaxSize");
    imports.add(enabled, "androidx.compose.foundation.layout.padding");
    imports.add(enabled, "androidx.compose.ui.Modifier");
    imports.add(enabled, "androidx.compose.ui.unit.dp");
    imports.add(enabled, "androidx.compose.ui.platform.LocalDensity");
    imports.add(enabled, "androidx.compose.runtime.rememberCoroutineScope");
    imports.add(enabled, "kotlinx.coroutines.launch");
}

pub(crate) fn render_scroll_view(
    axis: ScrollAxis,
    style: &ViewStyle,
    component_ref: Option<&nexa_ir::Expr>,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let (axis_name, container) = match axis {
        ScrollAxis::Vertical => (
            "Vertical",
            "NexaColumnPrimitive(spacing = 0f, horizontalAlignment = Alignment.Start)",
        ),
        ScrollAxis::Horizontal => (
            "Horizontal",
            "NexaRowPrimitive(spacing = 0f, verticalAlignment = Alignment.CenterVertically)",
        ),
        // The public ScrollView API only accepts a single axis.
        ScrollAxis::Both => (
            "Vertical",
            "NexaColumnPrimitive(spacing = 0f, horizontalAlignment = Alignment.Start)",
        ),
    };
    let reference = component_ref
        .map(expression)
        .unwrap_or_else(|| "null".to_owned());
    indent(out, depth);
    out.push_str(&format!(
        "NexaScrollViewPrimitive(\n{}axis = \"{axis_name}\",\n{}componentRef = {reference},\n{}modifier = nexaSafeAreaScrollViewportModifier(\"{axis_name}\")",
        " ".repeat((depth + 1) * 4),
        " ".repeat((depth + 1) * 4),
        " ".repeat((depth + 1) * 4),
    ));
    super::layout::render_modifiers(style, depth + 2, out);
    out.push_str(".fillMaxSize()");
    out.push_str(&format!(",\n{}content = {{\n", " ".repeat((depth + 1) * 4)));
    let pad = " ".repeat((depth + 2) * 4);
    out.push_str(&format!("{pad}{container} {{\n"));
    render_children(children, scope.module, scope.features, depth + 3, out);
    out.push('\n');
    out.push_str(&format!("{pad}}}\n"));
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth, format_args!(" )"));
}
