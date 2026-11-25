use nexa_codegen::SourceWriter;
use nexa_ir::{IconSource, IconSpec, SystemIcon};

use crate::generator::{
    colors,
    engine::{features::Features, imports::ImportSet},
    utils::kotlin_string,
};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let icons = &features.facts.ui.system_icons;
    let font_icons = features.facts.ui.font_icons;
    if icons.is_empty() && !font_icons {
        return;
    }

    let material_icons = icons.iter().collect::<Vec<_>>();
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

pub(crate) fn spec(icon: &IconSpec) -> String {
    let (source, preserves_color, icon_layers) = match &icon.source {
        IconSource::System(icon) => (
            if matches!(icon, SystemIcon::Shared(name) if name == "search") {
                format!(
                    "vector = {}, symbolScale = {}f",
                    icon.material_reference(),
                    nexa_codegen::design_system::MATERIAL_SEARCH_SIZE_SCALE
                )
            } else {
                format!("vector = {}", icon.material_reference())
            },
            false,
            &[][..],
        ),
        IconSource::FontGlyph {
            font_resource,
            glyph,
            layers,
            ..
        } => {
            let resource_name = nexa_codegen::names::font_resource_name(font_resource);
            (
                format!("glyph = {glyph}, fontResource = R.font.{resource_name}"),
                true,
                layers.as_slice(),
            )
        }
    };
    let description = icon
        .description
        .as_deref()
        .map(|value| format!(", description = {}", kotlin_string(value)))
        .unwrap_or_default();
    let size = icon
        .size
        .map(|value| {
            format!(
                ", size = {}.dp",
                crate::generator::engine::utils::number(value)
            )
        })
        .unwrap_or_default();
    let tint = icon
        .color
        .as_ref()
        .map(|value| format!(", tint = {}", colors::expression_for_color(value)))
        .unwrap_or_default();
    let layers = if icon_layers.is_empty() {
        String::new()
    } else {
        format!(", layers = listOf({})", kotlin_layers(icon_layers))
    };
    format!(
        "NexaIconSpec({source}{description}{size}{tint}, preservesColor = {preserves_color}{layers})"
    )
}

fn kotlin_layers(layers: &[nexa_ir::IconGlyphLayer]) -> String {
    layers
        .iter()
        .map(|layer| {
            let [red, green, blue, alpha] = layer.rgba;
            format!(
                "NexaIconLayer(glyph = {}, color = androidx.compose.ui.graphics.Color(red = {:.6}f, green = {:.6}f, blue = {:.6}f, alpha = {:.6}f))",
                layer.glyph,
                f32::from(red) / 255.0,
                f32::from(green) / 255.0,
                f32::from(blue) / 255.0,
                f32::from(alpha) / 255.0,
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn render(
    icon: &SystemIcon,
    description: Option<&str>,
    size: Option<f32>,
    tint: Option<&nexa_ir::ColorExpression>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let mut spec_icon = IconSpec::system(icon.clone());
    spec_icon.size = size;
    out.line_at(
        depth,
        format_args!(
            "NexaIconContent(icon = {}, description = {}, size = null, tint = {})",
            spec(&spec_icon),
            description
                .map(kotlin_string)
                .unwrap_or_else(|| "null".to_owned()),
            tint.map(colors::expression_for_color).unwrap_or_else(|| {
                "androidx.compose.material3.MaterialTheme.colorScheme.onSurface".to_owned()
            }),
        ),
    );
}

pub(crate) struct FontIconRenderArgs<'a> {
    pub(crate) font_resource: &'a str,
    pub(crate) glyph: &'a nexa_ir::Expr,
    pub(crate) layers: &'a [nexa_ir::IconGlyphLayer],
    pub(crate) description: Option<&'a str>,
    pub(crate) size: Option<&'a nexa_ir::Expr>,
    pub(crate) tint: Option<&'a nexa_ir::ColorExpression>,
}

pub(crate) fn render_font(icon: FontIconRenderArgs<'_>, depth: usize, out: &mut SourceWriter) {
    let resource_name = nexa_codegen::names::font_resource_name(icon.font_resource);
    out.line_at(
        depth,
        format_args!(
            "NexaFontIconPrimitive(glyph = {}, fontResource = R.font.{resource_name}, description = {}, size = {}, tint = {}, layers = listOf({}))",
            crate::generator::engine::expressions::expression(icon.glyph),
            icon.description.map(kotlin_string).unwrap_or_else(|| "null".to_owned()),
            icon.size.map(|value| format!("({}).toFloat().coerceIn(1f, 512f).dp", crate::generator::engine::expressions::expression(value))).unwrap_or_else(|| "null".to_owned()),
            icon.tint.map(colors::expression_for_color).unwrap_or_else(|| "null".to_owned()),
            kotlin_layers(icon.layers),
        ),
    );
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Color, ColorExpression, ColorValue, IconSpec, SystemIcon};

    use super::render;

    #[test]
    fn system_icon_size_is_passed_to_the_shared_primitive_in_logical_units() {
        let icon = SystemIcon::Shared("palette".to_owned());
        let tint = ColorExpression::Static(ColorValue::Static(Color {
            red: 255,
            green: 0,
            blue: 0,
            alpha: 255,
        }));
        let mut output = SourceWriter::new();

        render(&icon, Some(""), Some(20.0), Some(&tint), 0, &mut output);

        assert!(output.finish().contains("size = 20.dp"));
    }

    #[test]
    fn shared_search_uses_its_measured_symbol_scale() {
        let icon = IconSpec::system(SystemIcon::Shared("search".to_owned()));
        assert!(super::spec(&icon).contains("symbolScale = 1.32f"));
    }
}
