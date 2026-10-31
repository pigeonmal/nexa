use nexa_codegen::SourceWriter;
use nexa_ir::SystemIcon;

use crate::generator::{
    colors,
    engine::{features::Features, imports::ImportSet},
    utils::kotlin_string,
};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let icons = &features.facts.ui.system_icons;
    let font_icons =
        features.facts.ui.font_icons || icons.iter().any(|icon| icon.font_glyph_parts().is_some());
    if icons.is_empty() && !font_icons {
        return;
    }

    let material_icons = icons
        .iter()
        .filter(|icon| icon.font_glyph_parts().is_none())
        .collect::<Vec<_>>();
    imports.add(
        !material_icons.is_empty(),
        "androidx.compose.material.icons.Icons",
    );
    imports.add(
        !material_icons.is_empty(),
        "androidx.compose.material3.Icon",
    );
    imports.add(
        !material_icons.is_empty(),
        "androidx.compose.foundation.layout.size",
    );
    imports.add(!material_icons.is_empty(), "androidx.compose.ui.Modifier");
    imports.add(!material_icons.is_empty(), "androidx.compose.ui.unit.dp");
    imports.add(
        font_icons,
        "androidx.compose.ui.semantics.clearAndSetSemantics",
    );
    imports.add(
        font_icons,
        "androidx.compose.ui.semantics.contentDescription",
    );
    imports.add(font_icons, "androidx.compose.ui.semantics.semantics");
    for icon in material_icons {
        imports.add(true, icon.material_import());
    }
}

pub(crate) fn spec(icon: &SystemIcon) -> String {
    if let Some((family, glyph)) = icon.font_glyph_parts() {
        let resource_name = nexa_codegen::names::font_resource_name(family);
        format!("NexaIconSpec(glyph = {glyph}, fontResource = R.font.{resource_name})")
    } else {
        format!("NexaIconSpec(vector = {})", icon.material_reference())
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
    let render_size = if icon.font_glyph_parts().is_some() {
        size
    } else {
        size * nexa_codegen::design_system::MATERIAL_ICON_SIZE_SCALE
    };
    out.line_at(
        depth,
        format_args!(
            "NexaIconContent(icon = {}, description = {}, size = {}.dp, tint = {})",
            spec(icon),
            kotlin_string(description),
            crate::generator::engine::utils::number(render_size),
            colors::expression_for_color(tint),
        ),
    );
}

pub(crate) fn render_font(
    font_family: &str,
    glyph: &nexa_ir::Expr,
    description: &str,
    size: &nexa_ir::Expr,
    tint: &nexa_ir::ColorExpression,
    depth: usize,
    out: &mut SourceWriter,
) {
    let resource_name = nexa_codegen::names::font_resource_name(font_family);
    out.line_at(
        depth,
        format_args!(
            "NexaFontIconPrimitive(glyph = {}, fontResource = R.font.{resource_name}, description = {}, size = ({}).toFloat().coerceIn(1f, 512f).dp, tint = {})",
            crate::generator::engine::expressions::expression(glyph),
            kotlin_string(description),
            crate::generator::engine::expressions::expression(size),
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
