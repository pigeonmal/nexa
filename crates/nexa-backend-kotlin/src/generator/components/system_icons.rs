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

pub(crate) fn spec_with_catalog(
    icon: &SystemIcon,
    font_catalog: &nexa_codegen::FontCatalog,
) -> String {
    if let Some((family, glyph)) = icon.font_glyph_parts() {
        let Some(face) = font_catalog.resolve(family, nexa_ir::FontWeight::Normal) else {
            return spec(icon);
        };
        let resource_name = &face.android_resource_name;
        format!("NexaIconSpec(glyph = {glyph}, fontResource = R.font.{resource_name})")
    } else {
        spec(icon)
    }
}

#[cfg(test)]
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

pub(crate) fn render_with_catalog(
    icon: &SystemIcon,
    description: &str,
    size: f32,
    tint: &nexa_ir::ColorExpression,
    font_catalog: &nexa_codegen::FontCatalog,
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
            spec_with_catalog(icon, font_catalog),
            kotlin_string(description),
            crate::generator::engine::utils::number(render_size),
            colors::expression_for_color(tint),
        ),
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_font_with_catalog(
    font_family: &str,
    glyph: &nexa_ir::Expr,
    description: &str,
    size: &nexa_ir::Expr,
    tint: &nexa_ir::ColorExpression,
    font_catalog: &nexa_codegen::FontCatalog,
    depth: usize,
    out: &mut SourceWriter,
) {
    let resource_name = font_catalog
        .resolve(font_family, nexa_ir::FontWeight::Normal)
        .map(|face| face.android_resource_name.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| nexa_codegen::names::font_resource_name(font_family));
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
    use nexa_codegen::{FontCatalog, FontFace, SourceWriter};
    use nexa_ir::{Color, ColorExpression, ColorValue, SystemIcon};

    use super::{render, spec_with_catalog};

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

    #[test]
    fn configured_icon_family_uses_registered_android_resource() {
        let mut catalog = FontCatalog::default();
        catalog.insert(FontFace {
            family: "Brand Icons".to_owned(),
            weight: nexa_ir::FontWeight::Normal,
            postscript_name: "Mazicon-Regular".to_owned(),
            android_resource_name: "mazicon_file".to_owned(),
        });
        let icon = SystemIcon::font_glyph("Brand Icons", 0xE001).expect("valid glyph");

        assert_eq!(
            spec_with_catalog(&icon, &catalog),
            "NexaIconSpec(glyph = 57345, fontResource = R.font.mazicon_file)"
        );
    }
}
