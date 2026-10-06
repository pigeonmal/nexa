use nexa_codegen::SourceWriter;
use nexa_ir::SystemIcon;

use crate::generator::{
    colors,
    engine::{features::Features, imports::ImportSet},
    utils::{indent, kotlin_string},
};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let icons = &features.facts.ui.system_icons;
    if icons.is_empty() {
        return;
    }

    imports.add(true, "androidx.compose.material.icons.Icons");
    imports.add(true, "androidx.compose.material3.Icon");
    imports.add(true, "androidx.compose.foundation.layout.size");
    imports.add(true, "androidx.compose.ui.Modifier");
    imports.add(true, "androidx.compose.ui.unit.dp");
    for icon in icons {
        imports.add(true, icon.material_import());
    }
}

pub(crate) fn render(
    icon: &SystemIcon,
    description: &str,
    size: f32,
    tint: &nexa_ir::ColorExpression,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    out.push_str(&format!(
        "Icon(imageVector = {}, contentDescription = {}, modifier = Modifier.size({}.dp), tint = {})",
        icon.material_reference(),
        kotlin_string(description),
        crate::generator::engine::utils::number(size),
        colors::expression_for_color(tint),
    ));
}
