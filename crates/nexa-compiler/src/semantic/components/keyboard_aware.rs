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
    let focused_input_offset = optional_dimension(
        args.remove("focusedInputOffset"),
        "KeyboardAware focusedInputOffset",
        None,
        cx.themes,
    )?
    .unwrap_or(0.0);
    let mut modifiers = inv.modifiers;
    let on_show = lower_keyboard_event_handler(
        take_modifier(&mut modifiers, "onShow"),
        inv.span,
        ".onShow",
        cx,
    )?;
    let on_hide = lower_keyboard_event_handler(
        take_modifier(&mut modifiers, "onHide"),
        inv.span,
        ".onHide",
        cx,
    )?;
    let child_nodes = match inv.children {
        ast::ChildBody::Nodes(children) => children,
        _ => return Err(child_mismatch(inv.span)),
    };
    let mut content_nodes = Vec::with_capacity(child_nodes.len());
    let mut toolbar_nodes = None;
    for child in child_nodes {
        match child {
            ast::Node::ComponentInvocation(toolbar) if toolbar.name == "KeyboardToolbar" => {
                if toolbar_nodes.is_some() {
                    return Err(CompileError::new(
                        toolbar.span,
                        "KeyboardAware accepts at most one direct KeyboardToolbar child",
                    ));
                }
                let ast::ChildBody::Nodes(children) = toolbar.children else {
                    return Err(child_mismatch(toolbar.span));
                };
                toolbar_nodes = Some(children);
            }
            node => content_nodes.push(node),
        }
    }
    let lowered_children = lower_nodes(content_nodes, &child_cx)?;
    let toolbar = toolbar_nodes
        .map(|nodes| lower_nodes(nodes, &child_cx))
        .transpose()?;
    Ok(Node::KeyboardAware {
        dismiss: lower_keyboard_dismiss(dismiss)?,
        behavior,
        hide_horizontal_scrollbar,
        hide_vertical_scrollbar,
        focused_input_offset,
        toolbar,
        on_show,
        on_hide,
        children: lowered_children,
    })
}

fn lower_keyboard_event_handler(
    body: Option<ast::ModifierBody>,
    span: nexa_diagnostics::Span,
    modifier_name: &str,
    cx: &SemanticContext<'_>,
) -> Result<Option<nexa_ir::KeyboardEventHandler>, CompileError> {
    let (parameters, actions) = match body {
        Some(ast::ModifierBody::EventActions {
            parameters,
            actions,
        }) => (parameters, actions),
        None => return Ok(None),
        Some(_) => return Err(child_mismatch(span)),
    };
    if parameters.len() != 1 {
        return Err(CompileError::new(
            span,
            format!("{modifier_name} requires one KeyboardEvent binding"),
        ));
    }
    let Some(parameter) = parameters.into_iter().next() else {
        return Err(CompileError::new(
            span,
            format!("{modifier_name} requires one KeyboardEvent binding"),
        ));
    };
    if cx.symbols.contains_key(&parameter) {
        return Err(CompileError::new(
            span,
            format!("{modifier_name} binding `{parameter}` shadows an existing value"),
        ));
    }
    let event_type = Type::Struct {
        name: "KeyboardEvent".to_owned(),
        fields: vec![
            ("height".to_owned(), Type::Numeric(NumericType::Float64)),
            ("duration".to_owned(), Type::Numeric(NumericType::Float64)),
        ],
    };
    let mut callback_symbols = cx.symbols.clone();
    callback_symbols.insert(parameter.clone(), (event_type, false));
    let actions = lower_actions_with_aliases(
        actions,
        &callback_symbols,
        cx.functions,
        false,
        cx.native_aliases,
        cx.type_registries(),
    )?;
    Ok(Some(nexa_ir::KeyboardEventHandler { parameter, actions }))
}
