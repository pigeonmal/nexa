//! Typed shapes component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_spacer_component(
    inv: ast::ComponentInvocation,
    _cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let mut args = inv.arguments;
    let width = super::super::styles::optional_layout_dimension(
        args.remove("width"),
        "Spacer width",
        super::super::styles::DimensionAxis::Width,
    )?;
    let height = super::super::styles::optional_layout_dimension(
        args.remove("height"),
        "Spacer height",
        super::super::styles::DimensionAxis::Height,
    )?;
    Ok(Node::Spacer { width, height })
}

#[inline(never)]
pub(super) fn lower_divider_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let themes = cx.themes;

    let span = inv.span;
    let mut args = inv.arguments;
    let color = optional_color(args.remove("color"), "divider color", themes)?
        .ok_or_else(|| CompileError::new(span, "Divider requires a `color` value"))?;
    let thickness = optional_dimension(
        args.remove("thickness"),
        "divider thickness",
        Some(ast::ThemeTokenKind::Spacing),
        themes,
    )?
    .ok_or_else(|| CompileError::new(span, "Divider requires a `thickness` value"))?;
    Ok(Node::Divider { color, thickness })
}

#[inline(never)]
pub(super) fn lower_shape_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let themes = cx.themes;

    let span = inv.span;
    let mut args = inv.arguments;
    let shape = parse_shape(
        take_required_arg(&mut args, "Shape", "shape", span)?,
        "shape",
    )?;
    let mut lower_size = |name: &str| -> Result<(f32, Option<Expr>), CompileError> {
        let expression = take_required_arg(&mut args, "Shape", name, span)?;
        match &expression {
            ast::Expr::Number(_, _) => Ok((number_value(&expression, name)?, None)),
            ast::Expr::ThemeToken(_, token_span) => Err(CompileError::new(
                *token_span,
                format!("Shape {name} does not accept a theme token"),
            )),
            _ => Ok((
                0.0,
                Some(lower_expr(
                    &expression,
                    Some(&Type::Numeric(NumericType::Float64)),
                    &cx.exprs(false),
                )?),
            )),
        }
    };
    let (width, width_expression) = lower_size("width")?;
    let (height, height_expression) = lower_size("height")?;
    let fill = optional_color(args.remove("fill"), "Shape fill", themes)?
        .ok_or_else(|| CompileError::new(span, "Shape requires a `fill` value"))?;
    let animation = args.remove("animation").map(parse_animation).transpose()?;
    let measurement = if animation.is_none() {
        let reads_state = |expression: Option<&Expr>| match expression {
            Some(expression) => reads_numeric_state(expression),
            None => Some(false),
        };
        match (
            reads_state(width_expression.as_ref()),
            reads_state(height_expression.as_ref()),
        ) {
            (Some(true), Some(true)) => nexa_ir::ShapeMeasurement::Both,
            (Some(true), Some(false)) => nexa_ir::ShapeMeasurement::Width,
            (Some(false), Some(true)) => nexa_ir::ShapeMeasurement::Height,
            // Preserve the authored order when either dimension evaluates a call.
            _ => nexa_ir::ShapeMeasurement::None,
        }
    } else {
        nexa_ir::ShapeMeasurement::None
    };
    Ok(Node::Shape {
        shape,
        width,
        height,
        fill,
        width_expression,
        height_expression,
        measurement,
        animation,
    })
}

/// Classify scalar arithmetic that measurement can reevaluate without running
/// app functions, plugin calls, or operations that can throw.
fn reads_numeric_state(expression: &Expr) -> Option<bool> {
    match expression {
        Expr::State(_, Type::Numeric(_)) | Expr::AnimatedState(_, Type::Numeric(_)) => Some(true),
        Expr::Number { .. } => Some(false),
        Expr::Located { expression, .. } => reads_numeric_state(expression),
        Expr::Add(left, right, NumericType::Float32 | NumericType::Float64)
        | Expr::Arithmetic {
            left,
            right,
            ty: NumericType::Float32 | NumericType::Float64,
            ..
        } => {
            let left_reads = reads_numeric_state(left)?;
            let right_reads = reads_numeric_state(right)?;
            Some(left_reads || right_reads)
        }
        Expr::Negate { value, .. } => reads_numeric_state(value),
        _ => None,
    }
}
