//! Typed keyboard_aware component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_keyboard_aware_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let mut args = inv.arguments;
    let dismiss = args.remove("dismiss");
    let behavior = lower_keyboard_behavior(args.remove("behavior"))?;
    let hide_horizontal_scrollbar = optional_bool(
        args.remove("hideHorizontalScrollbar"),
        false,
        "KeyboardAware hideHorizontalScrollbar",
    )?;
    let hide_vertical_scrollbar = optional_bool(
        args.remove("hideVerticalScrollbar"),
        false,
        "KeyboardAware hideVerticalScrollbar",
    )?;
    let children = match inv.children {
        ast::ChildBody::Nodes(children) => children,
        _ => return Err(child_mismatch(inv.span)),
    };
    let lowered_children = lower_nodes(children, &child_cx)?;
    Ok(Node::KeyboardAware {
        dismiss: lower_keyboard_dismiss(dismiss)?,
        behavior,
        hide_horizontal_scrollbar,
        hide_vertical_scrollbar,
        children: lowered_children,
    })
}
