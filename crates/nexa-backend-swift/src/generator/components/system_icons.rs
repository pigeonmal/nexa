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

pub(crate) fn spec_with_catalog(
    icon: &SystemIcon,
    font_catalog: &nexa_codegen::FontCatalog,
) -> String {
    if let Some((family, glyph)) = icon.font_glyph_parts() {
        let family = font_catalog
            .resolve(family, nexa_ir::FontWeight::Normal)
            .map(|face| face.postscript_name.as_str())
            .unwrap_or(family);
        format!(
            "NexaIconSpec(glyph: Int32({glyph}), fontFamily: {})",
            swift_string(family)
        )
    } else {
        spec(icon)
    }
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
    out.line_at(
        depth,
        format_args!(
            "NexaIconView(icon: {}, description: {}, size: {}, tint: {})",
            spec_with_catalog(icon, font_catalog),
            swift_string(description),
            size,
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
    let font_family = font_catalog
        .resolve(font_family, nexa_ir::FontWeight::Normal)
        .map(|face| face.postscript_name.as_str())
        .unwrap_or(font_family);
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

#[cfg(test)]
mod tests {
    use nexa_codegen::{FontCatalog, FontFace};
    use nexa_ir::{FontWeight, SystemIcon};

    use super::spec_with_catalog;

    #[test]
    fn configured_icon_family_uses_registered_postscript_name() {
        let mut catalog = FontCatalog::default();
        catalog.insert(FontFace {
            family: "Brand Icons".to_owned(),
            weight: FontWeight::Normal,
            postscript_name: "Mazicon-Regular".to_owned(),
            android_resource_name: "mazicon_file".to_owned(),
        });
        let icon = SystemIcon::font_glyph("Brand Icons", 0xE001).expect("valid glyph");

        assert_eq!(
            spec_with_catalog(&icon, &catalog),
            "NexaIconSpec(glyph: Int32(57345), fontFamily: \"Mazicon-Regular\")"
        );
    }
}
