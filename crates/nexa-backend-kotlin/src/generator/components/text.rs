use nexa_codegen::SourceWriter;
use nexa_ir::{Expr, TextStyle};

use crate::generator::{
    colors,
    engine::{features::Features, imports::ImportSet},
    expressions::text_expression,
    utils::number,
};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.uses_text_node, "androidx.compose.material3.Text");
    imports.add(
        features.uses_text_node,
        "androidx.compose.ui.text.style.TextAlign",
    );
    imports.add(
        features.uses_font_weight || features.uses_text_node,
        "androidx.compose.ui.text.font.FontWeight",
    );
}

pub(crate) fn render(value: &Expr, style: &TextStyle, depth: usize, out: &mut SourceWriter) {
    out.text_at(
        depth,
        format_args!("NexaTextPrimitive(text = {}", text_expression(value)),
    );
    if let Some(color) = style.color {
        out.push_str(&format!(", color = {}", colors::expression(color)));
    }
    let semantic_metrics = style
        .font_style
        .map(nexa_codegen::design_system::text_metrics);
    let font_size = style
        .font_size
        .map(number)
        .or_else(|| semantic_metrics.map(|metrics| metrics.size.to_string()))
        .unwrap_or_else(|| nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE.to_string());
    if style.font_size.is_some() || style.font_style.is_some() {
        out.push_str(&format!(", fontSize = {font_size}.sp"));
    }
    let font_weight = style
        .font_weight
        .or_else(|| semantic_metrics.map(|metrics| metrics.weight))
        .unwrap_or(nexa_ir::FontWeight::Normal);
    if font_weight != nexa_ir::FontWeight::Normal {
        out.push_str(&format!(
            ", fontWeight = {}",
            kotlin_font_weight(font_weight)
        ));
    }
    if let Some(font_design) = style
        .font_design
        .filter(|design| *design != nexa_ir::TextFontDesign::Default)
    {
        let family = match font_design {
            nexa_ir::TextFontDesign::Default => "FontFamily.Default",
            nexa_ir::TextFontDesign::Serif => "FontFamily.Serif",
            nexa_ir::TextFontDesign::Rounded => {
                "FontFamily(android.graphics.Typeface.create(\"sans-serif-rounded\", android.graphics.Typeface.NORMAL))"
            }
            nexa_ir::TextFontDesign::Monospaced => "FontFamily.Monospace",
        };
        out.push_str(&format!(
            ", fontFamily = androidx.compose.ui.text.font.{family}"
        ));
    }
    if let Some(alignment) = style.alignment {
        let alignment = match alignment {
            nexa_ir::TextAlignment::Leading => "Start",
            nexa_ir::TextAlignment::Center => "Center",
            nexa_ir::TextAlignment::Trailing => "End",
        };
        out.push_str(&format!(", textAlign = TextAlign.{alignment}"));
    }
    if let Some(line_limit) = style.line_limit {
        out.push_str(&format!(", maxLines = {line_limit}"));
    }
    if let Some(line_height) = style.line_height {
        out.push_str(&format!(", lineHeight = {}.sp", number(line_height)));
    }
    if let Some(letter_spacing) = style.letter_spacing.filter(|spacing| *spacing != 0.0) {
        out.push_str(&format!(", letterSpacing = {}.sp", number(letter_spacing)));
    }
    if style.strikethrough {
        out.push_str(", strikethrough = true");
    }
    if style.italic {
        out.push_str(", italic = true");
    }
    if style.underline {
        out.push_str(", underline = true");
    }
    if style.selectable {
        out.push_str(", selectable = true");
    }
    if let Some(shadow) = style.effects.shadow {
        out.push_str(&format!(
            ", textShadow = NexaGlyphShadow(color = {}, radius = {}.dp, x = {}.dp, y = {}.dp)",
            colors::expression(shadow.color),
            number(shadow.radius),
            number(shadow.x),
            number(shadow.y)
        ));
    }
    append_visual_modifier(style, out);
    out.push(')');
}

fn append_visual_modifier(style: &TextStyle, out: &mut SourceWriter) {
    let effects = &style.effects;
    let has_modifier = style.padding.is_some()
        || style.opacity.is_some()
        || effects.scale.is_some()
        || effects.rotation.is_some()
        || effects.blur.is_some()
        || effects.clip_rounded.is_some()
        || effects.z_index.is_some();
    if !has_modifier {
        return;
    }
    out.push_str(", modifier = Modifier");
    if let Some(opacity) = style.opacity {
        out.push_str(&format!(".alpha({}f)", number(opacity)));
    }
    if effects.scale.is_some() || effects.rotation.is_some() {
        let scale = effects
            .scale
            .map(number)
            .unwrap_or_else(|| "1.0".to_owned());
        let rotation = effects
            .rotation
            .map(number)
            .unwrap_or_else(|| "0.0".to_owned());
        out.push_str(&format!(
            ".graphicsLayer(scaleX = {scale}f, scaleY = {scale}f, rotationZ = {rotation}f)"
        ));
    }
    if let Some(blur) = effects.blur {
        out.push_str(&format!(".blur({}.dp)", number(blur)));
    }
    if let Some(radius) = effects.clip_rounded {
        out.push_str(&format!(".clip(RoundedCornerShape({}.dp))", number(radius)));
    }
    if let Some(padding) = style.padding {
        out.push_str(&format!(".padding({}.dp)", number(padding)));
    }
    if let Some(z_index) = effects.z_index {
        out.push_str(&format!(".zIndex({}f)", z_index));
    }
}

fn kotlin_font_weight(weight: nexa_ir::FontWeight) -> &'static str {
    match weight {
        nexa_ir::FontWeight::Normal => "FontWeight.Normal",
        nexa_ir::FontWeight::Medium => "FontWeight.Medium",
        nexa_ir::FontWeight::Semibold => "FontWeight.SemiBold",
        nexa_ir::FontWeight::Bold => "FontWeight.Bold",
    }
}

#[cfg(test)]
mod tests {
    use super::render;
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Color, ColorValue, Expr, TextStyle, ViewEffects, ViewShadow};

    #[test]
    fn text_emphasis_is_passed_to_the_shared_native_primitive() {
        let mut output = SourceWriter::new();
        render(
            &Expr::String("Emphasis".to_owned()),
            &TextStyle {
                strikethrough: true,
                italic: true,
                underline: true,
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        let generated = output.finish();
        assert!(generated.contains("strikethrough = true, italic = true, underline = true"));
    }

    #[test]
    fn default_text_style_uses_shared_primitive_defaults() {
        let mut output = SourceWriter::new();
        render(
            &Expr::String("Default".to_owned()),
            &TextStyle::default(),
            0,
            &mut output,
        );

        assert_eq!(output.finish(), "NexaTextPrimitive(text = \"Default\")");
    }

    #[test]
    fn explicitly_requested_text_styles_override_shared_defaults() {
        let mut output = SourceWriter::new();
        render(
            &Expr::String("Title".to_owned()),
            &TextStyle {
                font_size: Some(23.0),
                font_weight: Some(nexa_ir::FontWeight::Bold),
                line_height: Some(30.0),
                letter_spacing: Some(0.5),
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        let generated = output.finish();
        assert!(generated.contains("fontSize = 23.sp"));
        assert!(generated.contains("fontWeight = FontWeight.Bold"));
        assert!(generated.contains("lineHeight = 30.sp"));
        assert!(generated.contains("letterSpacing = 0.5.sp"));
        assert!(!generated.contains("selectable = false"));
    }

    #[test]
    fn text_line_limit_is_forwarded_to_the_shared_primitive() {
        let mut output = SourceWriter::new();
        render(
            &Expr::String("This text should truncate at one line".to_owned()),
            &TextStyle {
                line_limit: Some(1),
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        assert!(output.finish().contains("maxLines = 1"));
    }

    #[test]
    fn text_shadow_is_applied_to_glyphs_not_a_rounded_rectangle() {
        let mut output = SourceWriter::new();
        render(
            &Expr::String("Shadow".to_owned()),
            &TextStyle {
                effects: ViewEffects {
                    shadow: Some(ViewShadow {
                        radius: 2.0,
                        x: 4.0,
                        y: 2.0,
                        color: ColorValue::Static(Color {
                            red: 0,
                            green: 0,
                            blue: 0,
                            alpha: 102,
                        }),
                    }),
                    ..ViewEffects::default()
                },
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        let generated = output.finish();
        assert!(generated.contains(
            "textShadow = NexaGlyphShadow(color = Color(0x66000000), radius = 2.dp, x = 4.dp, y = 2.dp)"
        ));
        assert!(!generated.contains("dropShadow"));
    }

    #[test]
    fn text_font_design_is_passed_to_the_shared_native_primitive() {
        let mut output = SourceWriter::new();
        render(
            &Expr::String("Serif".to_owned()),
            &TextStyle {
                font_design: Some(nexa_ir::TextFontDesign::Serif),
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        assert!(
            output
                .finish()
                .contains("fontFamily = androidx.compose.ui.text.font.FontFamily.Serif")
        );
    }

    #[test]
    fn rounded_text_design_uses_androids_public_typeface_bridge() {
        let mut output = SourceWriter::new();
        render(
            &Expr::String("Rounded".to_owned()),
            &TextStyle {
                font_design: Some(nexa_ir::TextFontDesign::Rounded),
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        assert!(output.finish().contains(
            "FontFamily(android.graphics.Typeface.create(\"sans-serif-rounded\", android.graphics.Typeface.NORMAL))"
        ));
    }
}
