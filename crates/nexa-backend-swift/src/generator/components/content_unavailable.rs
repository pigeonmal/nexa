use nexa_codegen::SourceWriter;
use nexa_ir::{ColorExpression, Expr};

use crate::generator::{colors, engine::expressions::localized_text_view};

#[derive(Clone, Copy, Default)]
pub(crate) struct ColorOverrides<'a> {
    pub(crate) title: Option<&'a ColorExpression>,
    pub(crate) description: Option<&'a ColorExpression>,
    pub(crate) icon: Option<&'a ColorExpression>,
    pub(crate) background: Option<&'a ColorExpression>,
}

pub(crate) fn render(
    title: &Expr,
    icon: &Expr,
    description: Option<&Expr>,
    overrides: ColorOverrides<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(
        depth,
        format_args!(
            "NexaContentUnavailablePrimitive(title: {}, icon: {}, description: {}, titleColor: {}, descriptionColor: {}, iconTint: {}, background: {})",
            localized_text_view(title, None),
            crate::generator::engine::expressions::expression(icon),
            description
                .map(|value| localized_text_view(value, None))
                .unwrap_or_else(|| "nil".to_owned()),
            overrides.title.map(colors::expression_for_color).unwrap_or_else(|| "nil".to_owned()),
            overrides.description.map(colors::expression_for_color).unwrap_or_else(|| "nil".to_owned()),
            overrides.icon.map(colors::expression_for_color).unwrap_or_else(|| "nil".to_owned()),
            overrides.background.map(colors::expression_for_color).unwrap_or_else(|| "nil".to_owned()),
        ),
    );
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, IconSpec, SystemIcon};

    use super::{ColorOverrides, render};

    #[test]
    fn content_unavailable_uses_shared_native_primitive() {
        let mut out = SourceWriter::new();
        render(
            &Expr::String("Nothing here".to_owned()),
            &Expr::Icon(Box::new(IconSpec::system(SystemIcon::shared("inbox").expect("catalog icon")))),
            Some(&Expr::String("Add an item to begin.".to_owned())),
            ColorOverrides::default(),
            0,
            &mut out,
        );
        let swift = out.finish();
        assert!(swift.contains("NexaContentUnavailablePrimitive("));
        assert!(swift.contains("NexaIconSpec(symbol: \"tray.fill\", preservesColor: false)"));
    }

    #[test]
    fn content_unavailable_omits_missing_description() {
        let mut out = SourceWriter::new();
        render(
            &Expr::String("Nothing here".to_owned()),
            &Expr::Icon(Box::new(IconSpec::system(SystemIcon::shared("inbox").expect("catalog icon")))),
            None,
            ColorOverrides::default(),
            0,
            &mut out,
        );
        assert!(out.finish().contains("description: nil"));
    }

    #[test]
    fn content_unavailable_renders_a_registered_font_glyph() {
        let mut out = SourceWriter::new();
        render(
            &Expr::String("No songs".to_owned()),
            &Expr::Icon(Box::new(IconSpec::font_glyph("Mazicon", "Mazicon", 61696))),
            None,
            ColorOverrides::default(),
            0,
            &mut out,
        );

        let swift = out.finish();
        assert!(swift.contains(
            "NexaIconSpec(glyph: Int32(61696), fontFamily: \"Mazicon\", preservesColor: true)"
        ));
    }
}
