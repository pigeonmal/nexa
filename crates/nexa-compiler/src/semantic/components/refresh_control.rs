//! Typed refresh_control component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_refresh_control_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let functions = cx.functions;
    let native_aliases = cx.native_aliases;
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let schema = invocation_schema(&inv)?;
    let mut args = inv.arguments;
    let is_refreshing = take_required_arg(&mut args, &inv.name, "isRefreshing", span)?;
    let children = match inv.children {
        ast::ChildBody::Nodes(children) => children,
        _ => return Err(child_mismatch(span)),
    };
    let mut modifiers = inv.modifiers;
    let actions = match take_modifier(&mut modifiers, "onRefresh") {
        Some(ast::ModifierBody::Actions(actions)) => actions,
        _ => {
            return Err(CompileError::new(
                span,
                catalog::required_modifier_message(schema, "onRefresh"),
            ));
        }
    };
    let state =
        require_mutable_binding(&is_refreshing, &Type::Bool, symbols, span, "RefreshControl")?;
    let mut lowered_children = lower_nodes(children, &child_cx)?;
    let actions = lower_actions_with_aliases(
        actions,
        symbols,
        functions,
        false,
        native_aliases,
        cx.type_registries(),
    )?;
    if lowered_children.len() == 1 {
        let Some(mut child) = lowered_children.pop() else {
            return Err(CompileError::new(
                span,
                "RefreshControl child was removed during lowering",
            ));
        };
        let refresh = FastListRefresh {
            state: state.clone(),
            actions: actions.clone(),
        };
        if attach_refresh_to_fast_list(&mut child, &refresh, span)? {
            return Ok(child);
        }
        lowered_children.push(child);
    }
    Ok(Node::RefreshControl {
        state,
        children: lowered_children,
        actions,
    })
}
