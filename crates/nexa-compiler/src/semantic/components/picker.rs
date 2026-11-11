//! Typed picker component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_picker_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;

    let span = inv.span;
    let mut args = inv.arguments;
    let comment = take_translator_comment(&mut args, "Picker")?;
    let items = take_required_arg(&mut args, &inv.name, "items", span)?;
    let selected = take_required_arg(&mut args, &inv.name, "selected", span)?;
    let icon = args
                .remove("icon")
                .map(|value| {
                    let name = require_string_literal(&value, "Picker icon")?;
                    cx.resolve_icon(&name).ok_or_else(|| {
                        CompileError::new(
                            value.span(),
                            format!("unknown icon `{name}` in Picker `icon`; use a Nexa shared icon or an alias from `assets.icons`"),
                        )
                    })
                })
                .transpose()?;
    let item_icons = args
        .remove("itemIcons")
        .map(|icons| lower_picker_item_icons(icons, cx))
        .transpose()?
        .unwrap_or_default();
    let state = require_mutable_binding(&selected, &Type::String, symbols, span, "Picker")?;
    let items = localize_ui_text(
        lower_expr(
            &items,
            Some(&Type::Array(Box::new(Type::String))),
            &cx.exprs(false),
        )?,
        comment.clone(),
    );
    let label = args
        .remove("label")
        .map(|value| lower_expr(&value, Some(&Type::String), &cx.exprs(false)))
        .transpose()?
        .map(|value| localize_ui_text(value, comment.clone()));
    let tint = args
        .remove("tint")
        .map(|value| lower_color_expression(value, "Picker tint", cx))
        .transpose()?;
    Ok(Node::Picker {
        items,
        state,
        icon,
        item_icons,
        label,
        tint,
    })
}
