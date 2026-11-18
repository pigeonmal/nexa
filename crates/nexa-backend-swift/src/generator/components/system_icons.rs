use nexa_codegen::SourceWriter;
use nexa_ir::{IconSource, IconSpec, SystemIcon};

use crate::generator::{colors, utils::swift_string};

pub(crate) fn spec(icon: &IconSpec) -> String {
    let (constructor, preserves_color, layers) = match &icon.source {
        IconSource::System(icon) => (
            format!("symbol: {}", swift_string(&icon.sf_symbol_name())),
            false,
            &[][..],
        ),
        IconSource::FontGlyph {
            font_family,
            glyph,
            layers,
            ..
        } => (
            format!(
                "glyph: Int32({glyph}), fontFamily: {}",
                swift_string(font_family)
            ),
            true,
            layers.as_slice(),
        ),
    };
    let description = icon
        .description
        .as_deref()
        .map(|value| format!(", description: {}", swift_string(value)))
        .unwrap_or_default();
    let size = icon
        .size
        .map(|value| format!(", size: {value}"))
        .unwrap_or_default();
    let tint = icon
        .color
        .as_ref()
        .map(|value| format!(", tint: {}", colors::expression_for_color(value)))
        .unwrap_or_default();
    let layers = if layers.is_empty() {
        String::new()
    } else {
        format!(", layers: [{}]", swift_layers(layers))
    };
    format!(
        "NexaIconSpec({constructor}{description}{size}{tint}{layers}, preservesColor: {preserves_color})"
    )
}

fn swift_layers(layers: &[nexa_ir::IconGlyphLayer]) -> String {
    layers
        .iter()
        .map(|layer| {
            let [red, green, blue, alpha] = layer.rgba;
            format!(
                "NexaIconLayer(glyph: Int32({}), color: Color(.sRGB, red: {:.6}, green: {:.6}, blue: {:.6}, opacity: {:.6}))",
                layer.glyph,
                f64::from(red) / 255.0,
                f64::from(green) / 255.0,
                f64::from(blue) / 255.0,
                f64::from(alpha) / 255.0,
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
    out.line_at(
        depth,
        format_args!(
            "NexaSystemIconPrimitive(symbol: {}, description: {}, size: {}, tint: {})",
            swift_string(&icon.sf_symbol_name()),
            description
                .map(swift_string)
                .unwrap_or_else(|| "nil".to_owned()),
            size.map(|value| format!("CGFloat({value})"))
                .unwrap_or_else(|| "nil".to_owned()),
            tint.map(colors::expression_for_color)
                .unwrap_or_else(|| "nil".to_owned()),
        ),
    );
}

pub(crate) struct FontIconRenderArgs<'a> {
    pub(crate) font_family: &'a str,
    pub(crate) glyph: &'a nexa_ir::Expr,
    pub(crate) layers: &'a [nexa_ir::IconGlyphLayer],
    pub(crate) description: Option<&'a str>,
    pub(crate) size: Option<&'a nexa_ir::Expr>,
    pub(crate) tint: Option<&'a nexa_ir::ColorExpression>,
}

pub(crate) fn render_font(icon: FontIconRenderArgs<'_>, depth: usize, out: &mut SourceWriter) {
    out.line_at(
        depth,
        format_args!(
            "NexaFontIconPrimitive(glyph: {}, fontFamily: {}, description: {}, size: {}, tint: {}, layers: [{}])",
            crate::generator::engine::expressions::expression(icon.glyph),
            swift_string(icon.font_family),
            icon.description
                .map(swift_string)
                .unwrap_or_else(|| "nil".to_owned()),
            icon.size.map(|value| format!(
                "CGFloat(max(1.0, min(512.0, {})))",
                crate::generator::engine::expressions::expression(value)
            ))
            .unwrap_or_else(|| "nil".to_owned()),
            icon.tint.map(colors::expression_for_color)
                .unwrap_or_else(|| "nil".to_owned()),
            swift_layers(icon.layers),
        ),
    );
}
