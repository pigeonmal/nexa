use nexa_codegen::SourceWriter;
use nexa_ir::{Expr, TextStyle};

use crate::generator::{
    colors,
    engine::{features::Features, imports::ImportSet},
    expressions::text_expression,
    utils::{indent, number},
};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.uses_text_node, "androidx.compose.material3.Text");
    imports.add(
        features.uses_text_node,
        "androidx.compose.ui.text.style.TextAlign",
    );
    imports.add(
        features.uses_font_weight,
        "androidx.compose.ui.text.font.FontWeight",
    );
    imports.add(
        features.uses_selectable_text,
        "androidx.compose.foundation.text.selection.SelectionContainer",
    );
    imports.add(
        features.uses_text_node,
        "androidx.compose.ui.text.style.TextDecoration",
    );
}

pub(crate) fn render(value: &Expr, style: &TextStyle, depth: usize, out: &mut SourceWriter) {
    let text_depth = if style.selectable {
        indent(out, depth);
        out.push_str("SelectionContainer {\n");
        depth + 1
    } else {
        depth
    };
    out.text_at(text_depth, format_args!("Text({}", text_expression(value)));
    if let Some(color) = style.color {
        out.push_str(&format!(", color = {}", colors::expression(color)));
    }
    if let Some(font_size) = style.font_size {
        out.push_str(&format!(", fontSize = {}.sp", number(font_size)));
    }
    if let Some(font_weight) = style.font_weight {
        out.push_str(&format!(
            ", fontWeight = {}",
            kotlin_font_weight(font_weight)
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
    if let Some(letter_spacing) = style.letter_spacing {
        out.push_str(&format!(", letterSpacing = {}.sp", number(letter_spacing)));
    }
    if style.strikethrough {
        out.push_str(", textDecoration = TextDecoration.LineThrough");
    }
    append_visual_modifier(style, out);
    out.push(')');
    if style.selectable {
        out.push('\n');
        indent(out, depth);
        out.push('}');
    }
}

fn append_visual_modifier(style: &TextStyle, out: &mut SourceWriter) {
    let effects = &style.effects;
    if style.padding.is_none() && style.opacity.is_none() && !effects.has_modifiers() {
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
    if let Some(shadow) = effects.shadow {
        let radius = effects.clip_rounded.unwrap_or(0.0);
        out.push_str(&format!(
            ".dropShadow(shape = RoundedCornerShape({}.dp), shadow = Shadow(radius = {}.dp, color = {}, offset = DpOffset({}.dp, {}.dp)))",
            number(radius),
            number(shadow.radius),
            colors::expression(shadow.color),
            number(shadow.x),
            number(shadow.y)
        ));
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
