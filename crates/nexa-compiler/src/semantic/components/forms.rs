//! Typed forms component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_form_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let mut args = inv.arguments;
    let show_dividers = optional_bool(args.remove("showDividers"), true, "Form showDividers")?;
    let hide_horizontal_scrollbar = optional_bool(
        args.remove("hideHorizontalScrollbar"),
        false,
        "Form hideHorizontalScrollbar",
    )?;
    let hide_vertical_scrollbar = optional_bool(
        args.remove("hideVerticalScrollbar"),
        false,
        "Form hideVerticalScrollbar",
    )?;
    let background = args
        .remove("background")
        .map(|value| parse_color(value, cx.themes, "Form background"))
        .transpose()?;
    let children = match inv.children {
        ast::ChildBody::Nodes(children) => children,
        _ => return Err(child_mismatch(span)),
    };
    Ok(Node::Form {
        show_dividers,
        hide_horizontal_scrollbar,
        hide_vertical_scrollbar,
        background,
        children: lower_nodes(children, &child_cx)?,
    })
}

#[inline(never)]
pub(super) fn lower_section_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let mut args = inv.arguments;
    let comment = take_translator_comment(&mut args, "Section")?;
    let show_dividers =
        optional_bool_override(args.remove("showDividers"), "Section showDividers")?;
    let background = args
        .remove("background")
        .map(|value| parse_color(value, cx.themes, "Section background"))
        .transpose()?;
    let title = args
        .remove("title")
        .map(|value| lower_expr(&value, Some(&Type::String), &cx.exprs(false)))
        .transpose()?
        .map(|value| localize_ui_text(value, comment.clone()));
    let footer = args
        .remove("footer")
        .map(|value| lower_expr(&value, Some(&Type::String), &cx.exprs(false)))
        .transpose()?
        .map(|value| localize_ui_text(value, comment.clone()));
    let children = match inv.children {
        ast::ChildBody::Nodes(children) => children,
        _ => return Err(child_mismatch(span)),
    };
    Ok(Node::FormSection {
        title,
        footer,
        show_dividers,
        background,
        children: lower_nodes(children, &child_cx)?,
    })
}
