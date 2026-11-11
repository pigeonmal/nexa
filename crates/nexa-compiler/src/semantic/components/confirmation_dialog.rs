//! Typed confirmation_dialog component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_confirmation_dialog_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let mut args = inv.arguments;
    let comment = take_translator_comment(&mut args, "ConfirmationDialog")?;
    let is_presented = take_required_arg(&mut args, &inv.name, "isPresented", span)?;
    let title = take_required_arg(&mut args, &inv.name, "title", span)?;
    let state = require_mutable_binding(
        &is_presented,
        &Type::Bool,
        symbols,
        span,
        "ConfirmationDialog",
    )?;
    let title = localize_ui_text(
        lower_expr(&title, Some(&Type::String), &cx.exprs(false))?,
        comment,
    );
    let children = match inv.children {
        ast::ChildBody::Nodes(children) => children,
        _ => return Err(child_mismatch(span)),
    };
    let children = lower_nodes(children, &child_cx)?;
    if children.is_empty()
        || children
            .iter()
            .any(|child| !matches!(child, Node::Button { .. }))
    {
        return Err(CompileError::new(
            span,
            "ConfirmationDialog children must be Button actions",
        ));
    }
    Ok(Node::ConfirmationDialog {
        state,
        title,
        children,
    })
}
