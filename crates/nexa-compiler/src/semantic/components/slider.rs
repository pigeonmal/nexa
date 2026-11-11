//! Typed slider component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_slider_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;

    let functions = cx.functions;
    let native_aliases = cx.native_aliases;

    let span = inv.span;
    let mut args = inv.arguments;
    let mut modifiers = inv.modifiers;
    let value = take_required_arg(&mut args, &inv.name, "value", span)?;
    let state = require_mutable_binding(
        &value,
        &Type::Numeric(NumericType::Float64),
        symbols,
        span,
        "Slider",
    )?;
    let min_ast = take_required_arg(&mut args, &inv.name, "min", span)?;
    let max_ast = take_required_arg(&mut args, &inv.name, "max", span)?;
    let min_literal = matches!(min_ast, ast::Expr::Number(_, _))
        .then(|| require_f64_literal(&min_ast, "Slider min"))
        .transpose()?;
    let max_literal = matches!(max_ast, ast::Expr::Number(_, _))
        .then(|| require_f64_literal(&max_ast, "Slider max"))
        .transpose()?;
    let min = lower_expr(
        &min_ast,
        Some(&Type::Numeric(NumericType::Float64)),
        &cx.exprs(false),
    )?;
    let max = lower_expr(
        &max_ast,
        Some(&Type::Numeric(NumericType::Float64)),
        &cx.exprs(false),
    )?;
    let step = args
        .remove("step")
        .map(|step| require_f64_literal(&step, "Slider step"))
        .transpose()?
        .unwrap_or(0.0);
    let show_step_indicators =
        optional_bool(args.remove("showStepIndicators"), true, "Slider showStepIndicators")?;
    let tint = args
        .remove("tint")
        .map(|value| lower_color_expression(value, "Slider tint", cx))
        .transpose()?;
    let track_color = args
        .remove("trackColor")
        .map(|value| lower_color_expression(value, "Slider trackColor", cx))
        .transpose()?;
    let thumb_color = args
        .remove("thumbColor")
        .map(|value| lower_color_expression(value, "Slider thumbColor", cx))
        .transpose()?;
    let track_height = optional_dimension(
        args.remove("trackHeight"),
        "Slider trackHeight",
        None,
        cx.themes,
    )?;
    let thumb_size = optional_dimension(
        args.remove("thumbSize"),
        "Slider thumbSize",
        None,
        cx.themes,
    )?;
    let hit_slop = optional_dimension(args.remove("hitSlop"), "Slider hitSlop", None, cx.themes)?;
    if min_literal
        .zip(max_literal)
        .is_some_and(|(min, max)| min >= max)
    {
        return Err(CompileError::new(span, "Slider min must be less than max"));
    }
    if step < 0.0 {
        return Err(CompileError::new(
            span,
            "Slider step cannot be negative; use 0.0 for continuous values",
        ));
    }
    if let Some((min, max)) = min_literal.zip(max_literal)
        && step > 0.0
    {
        let intervals = (max - min) / step;
        let rounded_intervals = intervals.round();
        let tolerance = intervals.abs().max(1.0) * 1e-9;
        if !intervals.is_finite()
            || rounded_intervals < 1.0
            || rounded_intervals > i32::MAX as f64
            || (intervals - rounded_intervals).abs() > tolerance
        {
            return Err(CompileError::new(
                span,
                "Slider range must contain a whole number of steps",
            ));
        }
    }
    if min_literal
        .into_iter()
        .chain(max_literal)
        .any(|value| value.abs() > f32::MAX as f64)
    {
        return Err(CompileError::new(
            span,
            "Slider min and max must fit the native Android Float range",
        ));
    }
    if track_height.is_some_and(|value| value <= 0.0)
        || thumb_size.is_some_and(|value| value <= 0.0)
        || hit_slop.is_some_and(|value| value < 0.0)
    {
        return Err(CompileError::new(
            span,
            "Slider trackHeight and thumbSize must be positive, and hitSlop cannot be negative",
        ));
    }
    let on_change = lower_slider_event(
        take_modifier(&mut modifiers, "onChange"),
        span,
        "onChange",
        Type::Numeric(NumericType::Float64),
        symbols,
        functions,
        native_aliases,
        cx.type_registries(),
    )?
    .map(|(parameter, actions)| nexa_ir::SliderChange { parameter, actions });
    let on_editing_changed = lower_slider_event(
        take_modifier(&mut modifiers, "onEditingChanged"),
        span,
        "onEditingChanged",
        Type::Bool,
        symbols,
        functions,
        native_aliases,
        cx.type_registries(),
    )?
    .map(|(parameter, actions)| nexa_ir::SliderEditingChanged { parameter, actions });
    Ok(Node::Slider {
        state,
        animated: false,
        min,
        max,
        step,
        show_step_indicators,
        tint,
        track_color,
        thumb_color,
        track_height,
        thumb_size,
        hit_slop,
        on_change,
        on_editing_changed,
    })
}
