//! Typed bottom_sheet component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_bottom_sheet_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let span = inv.span;
    let mut args = inv.arguments;
    let component_ref = take_required_arg(&mut args, &inv.name, "ref", span)?;
    let component_ref = lower_expr(
        &component_ref,
        Some(&Type::ComponentRef {
            namespace: "Nexa".to_owned(),
            name: "BottomSheet".to_owned(),
        }),
        &cx.exprs(false),
    )?;
    let content_padding = args
        .remove("contentPadding")
        .map(|value| number_value(&value, "BottomSheet contentPadding"))
        .transpose()?
        .unwrap_or(0.0);
    if !content_padding.is_finite() || content_padding < 0.0 {
        return Err(CompileError::new(
            span,
            "BottomSheet contentPadding must be a finite non-negative number",
        ));
    }
    let corner_radius = args
        .remove("cornerRadius")
        .map(|value| number_value(&value, "BottomSheet cornerRadius"))
        .transpose()?;
    if corner_radius.is_some_and(|radius| !radius.is_finite() || radius < 0.0) {
        return Err(CompileError::new(
            span,
            "BottomSheet cornerRadius must be a finite non-negative number",
        ));
    }
    let content_sized = optional_bool(
        args.remove("contentSized"),
        false,
        "BottomSheet contentSized",
    )?;
    let detents_argument = args.remove("detents");
    let initial_detent_argument = args.remove("initialDetent");
    if content_sized && (detents_argument.is_some() || initial_detent_argument.is_some()) {
        return Err(CompileError::new(
            span,
            "BottomSheet contentSized cannot be combined with detents or initialDetent",
        ));
    }
    let detents = if content_sized {
        Vec::new()
    } else {
        bottom_sheet_detents(detents_argument)?
    };
    let initial_detent = initial_detent_argument
        .as_ref()
        .map(|value| number_value(value, "BottomSheet initialDetent"))
        .transpose()?;
    if let Some(initial_detent) = initial_detent {
        if !(initial_detent.is_finite() && 0.0 < initial_detent && initial_detent <= 1.0) {
            return Err(CompileError::new(
                span,
                "BottomSheet initialDetent must be greater than 0 and at most 1.0",
            ));
        }
        if !detents.contains(&initial_detent) {
            return Err(CompileError::new(
                span,
                "BottomSheet initialDetent must be included in detents",
            ));
        }
    }
    let show_drag_indicator = args.remove("showDragIndicator");
    let allows_dismissal = args.remove("allowsDismissal");
    let allows_background_interaction = args.remove("allowsBackgroundInteraction");
    let should_dismiss_on_back_press = args.remove("shouldDismissOnBackPress");
    let background_color = args
        .remove("backgroundColor")
        .map(|color| lower_color_expression(color, "BottomSheet backgroundColor", cx))
        .transpose()?;
    let scrim_color = args
        .remove("scrimColor")
        .map(|color| lower_color_expression(color, "BottomSheet scrimColor", cx))
        .transpose()?;
    let mut modifiers = inv.modifiers;
    let on_dismiss = match take_modifier(&mut modifiers, "onDismiss") {
        Some(ast::ModifierBody::Actions(actions)) => lower_actions_with_aliases(
            actions,
            cx.symbols,
            cx.functions,
            false,
            cx.native_aliases,
            cx.type_registries(),
        )?,
        Some(_) => return Err(child_mismatch(span)),
        None => Vec::new(),
    };
    let children = match inv.children {
        ast::ChildBody::Nodes(children) => children,
        _ => return Err(child_mismatch(span)),
    };
    let sheet_cx = cx.with_navigation(true, cx.allow_navigation_back);
    let lowered_children = lower_nodes(children, &sheet_cx)?;
    Ok(Node::BottomSheet {
        component_ref,
        content_sized,
        content_padding,
        corner_radius,
        detents,
        initial_detent,
        show_drag_indicator: optional_bool(
            show_drag_indicator,
            true,
            "BottomSheet showDragIndicator",
        )?,
        allows_dismissal: optional_bool(allows_dismissal, true, "BottomSheet allowsDismissal")?,
        allows_background_interaction: optional_bool(
            allows_background_interaction,
            false,
            "BottomSheet allowsBackgroundInteraction",
        )?,
        should_dismiss_on_back_press: optional_bool(
            should_dismiss_on_back_press,
            true,
            "BottomSheet shouldDismissOnBackPress",
        )?,
        background_color,
        scrim_color,
        on_dismiss,
        children: lowered_children,
    })
}
