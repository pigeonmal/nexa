//! Typed text component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_text_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let themes = cx.themes;

    let is_reveal = inv.name == "TextReveal";
    let is_number_flow = inv.name == "NumberFlow";
    let component_name = if is_reveal {
        "TextReveal"
    } else if is_number_flow {
        "NumberFlow"
    } else {
        "Text"
    };
    let mut args = inv.arguments;
    let value = if is_number_flow {
        take_required_arg(&mut args, component_name, "value", inv.span)?
    } else {
        let mut positional = inv.positional;
        take_positional(&mut positional, inv.span, component_name)?
    };
    let comment = take_translator_comment(&mut args, component_name)?;
    let reveal = if is_reveal {
        Some(lower_text_reveal(&mut args, inv.span, cx)?)
    } else {
        None
    };
    let number_flow = if is_number_flow {
        let value_type = infer_expr_type(&value, cx.symbols, cx.functions)
            .ok_or_else(|| CompileError::new(value.span(), "NumberFlow `value` must be numeric"))?;
        if !matches!(value_type, Type::Numeric(_)) {
            return Err(CompileError::new(
                value.span(),
                "NumberFlow `value` must be numeric",
            ));
        }
        Some(lower_number_flow(&mut args, inv.span)?)
    } else {
        None
    };
    let color = args.remove("color");
    let alignment = lower_text_alignment(args.remove("alignment"))?;
    let font_size = args.remove("fontSize");
    let font_style = lower_text_font_style(args.remove("fontStyle"))?;
    let font_design = lower_text_font_design(args.remove("fontDesign"))?;
    let font_family = lower_text_font_family(args.remove("fontFamily"), themes)?;
    let font_weight = args.remove("fontWeight");
    let line_limit = args.remove("lineLimit");
    let line_height = args.remove("lineHeight");
    let letter_spacing = args.remove("letterSpacing");
    let strikethrough = args.remove("strikethrough");
    let italic = args.remove("italic");
    let underline = args.remove("underline");
    let selectable = args.remove("selectable");
    let opacity = args.remove("opacity");
    let value = lower_expr(&value, None, &cx.exprs(false))?;
    let value = if is_number_flow {
        value
    } else {
        localize_ui_text(value, comment.clone())
    };
    let color = optional_color(color, "text color", themes)?;
    let font_size = optional_dimension(
        font_size,
        "fontSize",
        Some(ast::ThemeTokenKind::FontSize),
        themes,
    )?;
    let font_weight = lower_font_weight(font_weight)?;
    let line_limit = lower_line_limit(line_limit)?;
    let line_height = optional_dimension(line_height, "lineHeight", None, themes)?;
    let letter_spacing = optional_tracking(letter_spacing)?;
    let strikethrough = optional_bool(strikethrough, false, "strikethrough")?;
    let italic = optional_bool(italic, false, "italic")?;
    let underline = optional_bool(underline, false, "underline")?;
    let selectable = optional_bool(selectable, false, "selectable")?;
    let opacity = lower_opacity(opacity)?;
    let effects = lower_view_effects(
        args.remove("scale"),
        args.remove("scaleY"),
        args.remove("rotation"),
        None,
        args.remove("shadow"),
        args.remove("blur"),
        args.remove("clip"),
        args.remove("zIndex"),
        args.remove("glass"),
        themes,
        &cx.exprs(false),
    )?;
    Ok(Node::Text {
        value,
        style: TextStyle {
            alignment,
            color,
            font_size,
            font_style,
            font_design,
            font_family,
            font_weight,
            opacity,
            effects,
            line_limit,
            line_height,
            letter_spacing,
            strikethrough,
            italic,
            underline,
            selectable,
            reveal: reveal.map(Box::new),
            number_flow: number_flow.map(Box::new),
        },
    })
}
