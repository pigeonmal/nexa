//! Typed pressable component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_pressable_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let functions = cx.functions;
    let native_aliases = cx.native_aliases;
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let mut args = inv.arguments;
    let disabled = args.remove("disabled");
    let haptic = args.remove("haptic");
    let fill_max_size = match args.remove("fillMaxSize") {
        Some(ast::Expr::Bool(value, _)) => value,
        Some(value) => {
            return Err(CompileError::new(
                value.span(),
                "Pressable `fillMaxSize` must be a Boolean literal",
            ));
        }
        None => false,
    };
    let children = match inv.children {
        ast::ChildBody::Nodes(children) => children,
        _ => return Err(child_mismatch(span)),
    };
    let mut modifiers = inv.modifiers;
    let actions = match take_modifier_decl(&mut modifiers, "onTap").map(|modifier| modifier.body) {
        Some(ast::ModifierBody::Actions(actions)) => actions,
        _ => {
            return Err(CompileError::new(
                span,
                "Pressable requires a tap handler using `.onTap { ... }`",
            ));
        }
    };
    let (long_press_duration_ms, long_press_actions) =
        match take_modifier_decl(&mut modifiers, "onLongPress") {
            Some(modifier) => {
                let duration = modifier.arguments.get("durationMs");
                let duration = duration
                    .map(|value| {
                        lower_expr(
                            value,
                            Some(&Type::Numeric(NumericType::Int32)),
                            &cx.exprs(false),
                        )
                    })
                    .transpose()?
                    .unwrap_or(Expr::Number {
                        raw: "500".to_owned(),
                        ty: NumericType::Int32,
                    });
                let actions = match modifier.body {
                    ast::ModifierBody::Actions(actions) => actions,
                    _ => return Err(child_mismatch(span)),
                };
                (duration, actions)
            }
            None => (
                Expr::Number {
                    raw: "500".to_owned(),
                    ty: NumericType::Int32,
                },
                Vec::new(),
            ),
        };
    let context_menu = take_modifier_nodes(&mut modifiers, span, "contextMenu")?;
    let double_tap_actions = match take_modifier(&mut modifiers, "onDoubleTap") {
        Some(ast::ModifierBody::Actions(actions)) => actions,
        Some(_) => return Err(child_mismatch(span)),
        None => Vec::new(),
    };
    let (drag_parameters, drag_actions) = match take_modifier(&mut modifiers, "onDrag") {
        Some(ast::ModifierBody::EventActions {
            parameters,
            actions,
        }) => (parameters, actions),
        Some(_) => return Err(child_mismatch(span)),
        None => (Vec::new(), Vec::new()),
    };
    let (pinch_parameter, pinch_actions) = match take_modifier(&mut modifiers, "onPinch") {
        Some(ast::ModifierBody::EventActions {
            parameters,
            actions,
        }) => {
            if parameters.len() != 1 {
                return Err(CompileError::new(
                    span,
                    ".onPinch requires one binding for its scaleFactor delta",
                ));
            }
            let Some(parameter) = parameters.into_iter().next() else {
                return Err(CompileError::new(
                    span,
                    ".onPinch requires one binding for its scaleFactor delta",
                ));
            };
            if symbols.contains_key(&parameter) {
                return Err(CompileError::new(
                    span,
                    format!(".onPinch binding `{parameter}` shadows an existing value"),
                ));
            }
            (Some(parameter), actions)
        }
        Some(_) => return Err(child_mismatch(span)),
        None => (None, Vec::new()),
    };
    let mut drag_symbols = symbols.clone();
    if !drag_parameters.is_empty() {
        if drag_parameters.len() != 4 {
            return Err(CompileError::new(
                span,
                ".onDrag requires four bindings: translationX, translationY, velocityX, and velocityY",
            ));
        }
        let mut unique_parameters = HashSet::with_capacity(drag_parameters.len());
        for parameter in &drag_parameters {
            if !unique_parameters.insert(parameter.as_str()) {
                return Err(CompileError::new(
                    span,
                    format!(".onDrag binds `{parameter}` more than once"),
                ));
            }
            if symbols.contains_key(parameter) {
                return Err(CompileError::new(
                    span,
                    format!(".onDrag binding `{parameter}` shadows an existing value"),
                ));
            }
            drag_symbols.insert(
                parameter.clone(),
                (Type::Numeric(NumericType::Float64), false),
            );
        }
    }
    let mut pinch_symbols = symbols.clone();
    if let Some(parameter) = &pinch_parameter {
        pinch_symbols.insert(
            parameter.clone(),
            (Type::Numeric(NumericType::Float64), false),
        );
    }
    let disabled = disabled
        .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
        .transpose()?
        .unwrap_or(Expr::Bool(false));
    let haptic = lower_haptic(haptic)?;
    let lowered_children = lower_nodes(children, &child_cx)?;
    let context_menu = context_menu
                .map(|nodes| {
                    if nodes.is_empty()
                        || !nodes
                            .iter()
                            .any(|node| matches!(node, ast::Node::ComponentInvocation(inv) if inv.name == "Button"))
                    {
                        return Err(CompileError::new(
                            span,
                            "Pressable `contextMenu` requires at least one Button action",
                        ));
                    }
                    let lowered = lower_nodes(nodes, &child_cx)?;
                    if lowered
                        .iter()
                        .any(|node| !matches!(node, Node::Button { .. }))
                    {
                        return Err(CompileError::new(
                            span,
                            "Pressable `contextMenu` currently accepts only Button actions",
                        ));
                    }
                    Ok(lowered)
                })
                .transpose()?
                .unwrap_or_default();
    let actions = lower_actions_with_aliases(
        actions,
        symbols,
        functions,
        false,
        native_aliases,
        cx.type_registries(),
    )?;
    let long_press_actions = lower_actions_with_aliases(
        long_press_actions,
        symbols,
        functions,
        false,
        native_aliases,
        cx.type_registries(),
    )?;
    let double_tap_actions = lower_actions_with_aliases(
        double_tap_actions,
        symbols,
        functions,
        false,
        native_aliases,
        cx.type_registries(),
    )?;
    let drag_actions = lower_actions_with_aliases(
        drag_actions,
        &drag_symbols,
        functions,
        false,
        native_aliases,
        cx.type_registries(),
    )?;
    let pinch_actions = lower_actions_with_aliases(
        pinch_actions,
        &pinch_symbols,
        functions,
        false,
        native_aliases,
        cx.type_registries(),
    )?;
    Ok(Node::Pressable {
        disabled,
        haptic,
        fill_max_size,
        children: lowered_children,
        actions,
        double_tap_actions,
        long_press_duration_ms,
        long_press_actions,
        context_menu,
        drag_parameters,
        drag_actions,
        pinch_parameter,
        pinch_actions,
    })
}
