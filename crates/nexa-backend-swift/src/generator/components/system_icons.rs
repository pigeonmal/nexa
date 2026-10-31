use nexa_codegen::SourceWriter;
use nexa_ir::SystemIcon;

use crate::generator::{colors, utils::swift_string};

pub(crate) fn spec(icon: &SystemIcon) -> String {
    if let Some((family, glyph)) = icon.font_glyph_parts() {
        format!(
            "NexaIconSpec(glyph: Int32({glyph}), fontFamily: {})",
            swift_string(family)
        )
    } else {
        format!(
            "NexaIconSpec(symbol: {})",
            swift_string(&icon.sf_symbol_name())
        )
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
    out.line_at(
        depth,
        format_args!(
            "NexaIconView(icon: {}, description: {}, size: {}, tint: {})",
            spec(icon),
            swift_string(description),
            size,
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
    out.line_at(
        depth,
        format_args!(
            "NexaFontIconPrimitive(glyph: {}, fontFamily: {}, description: {}, size: CGFloat(max(1.0, min(512.0, {}))), tint: {})",
            crate::generator::engine::expressions::expression(glyph),
            swift_string(font_family),
            swift_string(description),
            crate::generator::engine::expressions::expression(size),
            colors::expression_for_color(tint),
        ),
    );
}
