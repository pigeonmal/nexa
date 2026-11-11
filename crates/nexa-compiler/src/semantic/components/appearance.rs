//! Typed appearance component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_status_bar_component(
    inv: ast::ComponentInvocation,
    _cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let mut args = inv.arguments;
    let style = args.remove("style");
    let hidden = args.remove("hidden");
    let background = args.remove("background");
    let navigation_bar_background = args.remove("navigationBarBackground");
    Ok(Node::StatusBar {
        config: lower_status_bar(style, hidden, background, navigation_bar_background)?,
    })
}

#[inline(never)]
pub(super) fn lower_appearance_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let allow_navigation_stack = cx.allow_navigation_stack;

    let span = inv.span;
    let mut args = inv.arguments;
    let mode = take_required_arg(&mut args, &inv.name, "mode", span)?;
    let mode = lower_expr(&mode, Some(&Type::String), &cx.exprs(false))?;
    if let Expr::String(value) = &mode
        && !matches!(value.as_str(), "system" | "light" | "dark")
    {
        return Err(CompileError::new(
            span,
            "Appearance mode must be `system`, `light`, or `dark`",
        ));
    }
    let appearance_cx = cx.with_navigation(allow_navigation_stack, cx.allow_navigation_back);
    let children = match inv.children {
        ast::ChildBody::Nodes(nodes) => lower_nodes(nodes, &appearance_cx)?,
        _ => return Err(child_mismatch(span)),
    };
    Ok(Node::Appearance { mode, children })
}

#[inline(never)]
pub(super) fn lower_direction_component(
    inv: ast::ComponentInvocation,
    _cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let span = inv.span;
    let mut args = inv.arguments;
    let value = take_required_arg(&mut args, &inv.name, "value", span)?;
    Ok(Node::Direction {
        config: lower_direction(value)?,
    })
}
