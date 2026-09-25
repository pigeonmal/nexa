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
        features.uses_font_weight,
        "androidx.compose.ui.text.font.FontWeight",
    );
    imports.add(
        features.uses_selectable_text,
        "androidx.compose.foundation.text.selection.SelectionContainer",
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
    if let Some(line_limit) = style.line_limit {
        out.push_str(&format!(", maxLines = {line_limit}"));
    }
    if let Some(line_height) = style.line_height {
        out.push_str(&format!(", lineHeight = {}.sp", number(line_height)));
    }
    if let Some(letter_spacing) = style.letter_spacing {
        out.push_str(&format!(", letterSpacing = {}.sp", number(letter_spacing)));
    }
    out.push(')');
    if style.selectable {
        out.push('\n');
        indent(out, depth);
        out.push('}');
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
