//! Typed lifecycle component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_on_appear_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let functions = cx.functions;
    let native_aliases = cx.native_aliases;

    let span = inv.span;
    let asynchronous = inv.flags.iter().any(|flag| flag == "async");
    let actions = match inv.children {
        ast::ChildBody::Actions(actions) => actions,
        _ => return Err(child_mismatch(span)),
    };
    Ok(Node::OnAppear {
        actions: lower_actions_with_aliases(
            actions,
            symbols,
            functions,
            asynchronous,
            native_aliases,
            cx.type_registries(),
        )?,
        asynchronous,
    })
}

#[inline(never)]
pub(super) fn lower_on_disappear_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let functions = cx.functions;
    let native_aliases = cx.native_aliases;

    let span = inv.span;
    let actions = match inv.children {
        ast::ChildBody::Actions(actions) => actions,
        _ => return Err(child_mismatch(span)),
    };
    Ok(Node::OnDisappear {
        actions: lower_actions_with_aliases(
            actions,
            symbols,
            functions,
            false,
            native_aliases,
            cx.type_registries(),
        )?,
    })
}

#[inline(never)]
pub(super) fn lower_on_active_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let functions = cx.functions;
    let native_aliases = cx.native_aliases;

    let span = inv.span;
    let actions = match inv.children {
        ast::ChildBody::Actions(actions) => actions,
        _ => return Err(child_mismatch(span)),
    };
    Ok(Node::OnActive {
        actions: lower_actions_with_aliases(
            actions,
            symbols,
            functions,
            false,
            native_aliases,
            cx.type_registries(),
        )?,
    })
}

#[inline(never)]
pub(super) fn lower_on_inactive_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let functions = cx.functions;
    let native_aliases = cx.native_aliases;

    let span = inv.span;
    let actions = match inv.children {
        ast::ChildBody::Actions(actions) => actions,
        _ => return Err(child_mismatch(span)),
    };
    Ok(Node::OnInactive {
        actions: lower_actions_with_aliases(
            actions,
            symbols,
            functions,
            false,
            native_aliases,
            cx.type_registries(),
        )?,
    })
}

#[inline(never)]
pub(super) fn lower_on_background_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let functions = cx.functions;
    let native_aliases = cx.native_aliases;

    let span = inv.span;
    let actions = match inv.children {
        ast::ChildBody::Actions(actions) => actions,
        _ => return Err(child_mismatch(span)),
    };
    Ok(Node::OnBackground {
        actions: lower_actions_with_aliases(
            actions,
            symbols,
            functions,
            false,
            native_aliases,
            cx.type_registries(),
        )?,
    })
}
