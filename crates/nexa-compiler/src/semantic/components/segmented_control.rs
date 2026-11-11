//! Typed segmented_control component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_segmented_control_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;

    let span = inv.span;
    let mut args = inv.arguments;
    let comment = take_translator_comment(&mut args, "SegmentedControl")?;
    let animation = args.remove("animation").map(parse_animation).transpose()?;
    let tint = args
        .remove("tint")
        .map(|value| lower_color_expression(value, "SegmentedControl tint", cx))
        .transpose()?;
    let background = args
        .remove("background")
        .map(|value| lower_color_expression(value, "SegmentedControl background", cx))
        .transpose()?;
    let selected_text_color = args
        .remove("selectedTextColor")
        .map(|value| lower_color_expression(value, "SegmentedControl selectedTextColor", cx))
        .transpose()?;
    let text_color = args
        .remove("textColor")
        .map(|value| lower_color_expression(value, "SegmentedControl textColor", cx))
        .transpose()?;
    let font_size = optional_dimension(
        args.remove("fontSize"),
        "fontSize",
        Some(ast::ThemeTokenKind::FontSize),
        cx.themes,
    )?;
    let font_weight = lower_font_weight(args.remove("fontWeight"))?;
    let height = optional_dimension(args.remove("height"), "height", None, cx.themes)?;
    let corner_radius =
        optional_dimension(args.remove("cornerRadius"), "cornerRadius", None, cx.themes)?;
    let border_color = args
        .remove("borderColor")
        .map(|value| lower_color_expression(value, "SegmentedControl borderColor", cx))
        .transpose()?;
    let border_width =
        optional_dimension(args.remove("borderWidth"), "borderWidth", None, cx.themes)?;
    let items = take_required_arg(&mut args, &inv.name, "items", span)?;
    let selected = take_required_arg(&mut args, &inv.name, "selected", span)?;
    let state =
        require_mutable_binding(&selected, &Type::String, symbols, span, "SegmentedControl")?;
    let items = localize_ui_text(
        lower_expr(
            &items,
            Some(&Type::Array(Box::new(Type::String))),
            &cx.exprs(false),
        )?,
        comment.clone(),
    );
    Ok(Node::SegmentedControl {
        items,
        state,
        animation,
        tint,
        background,
        text_color,
        selected_text_color,
        font_size,
        font_weight,
        height,
        corner_radius,
        border_color,
        border_width,
    })
}
