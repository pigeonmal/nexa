//! Typed dialog component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_dialog_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let mut args = inv.arguments;
    let comment = take_translator_comment(&mut args, "Dialog")?;
    let is_presented = take_required_arg(&mut args, &inv.name, "isPresented", span)?;
    let title = take_required_arg(&mut args, &inv.name, "title", span)?;
    let message = take_required_arg(&mut args, &inv.name, "message", span)?;
    let state = require_mutable_binding(&is_presented, &Type::Bool, symbols, span, "Dialog")?;
    let title = localize_ui_text(
        lower_expr(&title, Some(&Type::String), &cx.exprs(false))?,
        comment.clone(),
    );
    let message = localize_ui_text(
        lower_expr(&message, Some(&Type::String), &cx.exprs(false))?,
        comment.clone(),
    );
    let children = match inv.children {
        ast::ChildBody::Nodes(children) => children,
        _ => return Err(child_mismatch(span)),
    };
    let lowered_children = lower_nodes(children, &child_cx)?;
    let mut text_input_count = 0;
    for child in &lowered_children {
        match child {
            Node::TextInput {
                secure: false,
                multiline: false,
                searchable: false,
                actions,
                ..
            } if actions.is_empty() => text_input_count += 1,
            Node::Button { .. } => {}
            Node::TextInput { .. } => {
                return Err(CompileError::new(
                    span,
                    "Dialog text input must be a single-line, plain TextInput without submit actions",
                ));
            }
            _ => {
                return Err(CompileError::new(
                    span,
                    "Dialog children must be Button actions and at most one TextInput",
                ));
            }
        }
    }
    if text_input_count > 1 {
        return Err(CompileError::new(
            span,
            "Dialog supports at most one TextInput",
        ));
    }
    Ok(Node::Dialog {
        state,
        title,
        message,
        children: lowered_children,
    })
}
