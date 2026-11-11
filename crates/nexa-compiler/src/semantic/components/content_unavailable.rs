//! Typed content_unavailable component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_content_unavailable_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let span = inv.span;
    let mut args = inv.arguments;
    let comment = take_translator_comment(&mut args, "ContentUnavailable")?;
    let title = lower_expr(
        &take_required_arg(&mut args, &inv.name, "title", span)?,
        Some(&Type::String),
        &cx.exprs(false),
    )?;
    let icon_name = require_string_literal(
        &take_required_arg(&mut args, &inv.name, "icon", span)?,
        "ContentUnavailable icon",
    )?;
    let icon = cx.resolve_icon(&icon_name).ok_or_else(|| {
        CompileError::new(
            span,
            format!(
                "unknown icon `{icon_name}`; use a Nexa shared icon or an alias from `assets.icons`"
            ),
        )
    })?;
    let description = args
        .remove("description")
        .map(|description| {
            lower_expr(&description, Some(&Type::String), &cx.exprs(false))
                .map(|description| localize_ui_text(description, comment.clone()))
        })
        .transpose()?;
    let title_color = args
        .remove("titleColor")
        .map(|value| lower_color_expression(value, "ContentUnavailable titleColor", cx))
        .transpose()?;
    let description_color = args
        .remove("descriptionColor")
        .map(|value| lower_color_expression(value, "ContentUnavailable descriptionColor", cx))
        .transpose()?;
    let icon_tint = args
        .remove("iconTint")
        .map(|value| lower_color_expression(value, "ContentUnavailable iconTint", cx))
        .transpose()?;
    let background = args
        .remove("background")
        .map(|value| lower_color_expression(value, "ContentUnavailable background", cx))
        .transpose()?;
    Ok(Node::ContentUnavailable {
        title: localize_ui_text(title, comment.clone()),
        icon,
        description,
        title_color,
        description_color,
        icon_tint,
        background,
    })
}
