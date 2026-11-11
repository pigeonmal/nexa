//! Typed switch component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_switch_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let functions = cx.functions;
    let native_aliases = cx.native_aliases;

    let span = inv.span;
    let mut args = inv.arguments;
    let mut modifiers = inv.modifiers;
    let comment = take_translator_comment(&mut args, "Switch")?;
    let value = take_required_arg(&mut args, &inv.name, "value", span)?;
    let label = take_required_arg(&mut args, &inv.name, "label", span)?;
    let state = require_mutable_binding(&value, &Type::Bool, symbols, span, "Switch")?;
    let label = localize_ui_text(
        lower_expr(&label, Some(&Type::String), &cx.exprs(false))?,
        comment.clone(),
    );
    let on_change = match take_modifier(&mut modifiers, "onChange") {
        Some(ast::ModifierBody::EventActions {
            parameters,
            actions,
        }) => {
            if parameters.len() != 1 {
                return Err(CompileError::new(
                    span,
                    ".onChange requires one Bool binding",
                ));
            }
            let Some(parameter) = parameters.into_iter().next() else {
                return Err(CompileError::new(
                    span,
                    ".onChange requires one Bool binding",
                ));
            };
            if symbols.contains_key(&parameter) {
                return Err(CompileError::new(
                    span,
                    format!(".onChange binding `{parameter}` shadows an existing value"),
                ));
            }
            let mut callback_symbols = symbols.clone();
            callback_symbols.insert(parameter.clone(), (Type::Bool, false));
            Some(nexa_ir::SwitchChange {
                parameter,
                actions: lower_actions_with_aliases(
                    actions,
                    &callback_symbols,
                    functions,
                    false,
                    native_aliases,
                    cx.type_registries(),
                )?,
            })
        }
        Some(_) => return Err(child_mismatch(span)),
        None => None,
    };
    Ok(Node::Switch {
        state,
        label,
        on_change,
    })
}
