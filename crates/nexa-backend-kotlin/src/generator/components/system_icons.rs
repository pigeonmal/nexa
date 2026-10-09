use nexa_codegen::SourceWriter;
use nexa_ir::SystemIcon;

use crate::generator::{
    colors,
    engine::{features::Features, imports::ImportSet},
    utils::kotlin_string,
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
    let material_size = size * nexa_codegen::design_system::MATERIAL_ICON_SIZE_SCALE;
    out.line_at(
        depth,
        format_args!(
            "NexaSystemIconPrimitive(image = {}, description = {}, size = {}.dp, tint = {})",
            icon.material_reference(),
            kotlin_string(description),
            crate::generator::engine::utils::number(material_size),
            colors::expression_for_color(tint),
        ),
    );
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Color, ColorExpression, ColorValue, SystemIcon};

    use super::render;

    #[test]
    fn system_icon_size_scales_to_match_swiftui_symbol_metrics() {
        let icon = SystemIcon::Shared("palette".to_owned());
        let tint = ColorExpression::Static(ColorValue::Static(Color {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        }));
        let mut output = SourceWriter::new();

        render(&icon, "", 20.0, &tint, 0, &mut output);

        assert!(output.finish().contains("size = 24.dp"));
    }
}
