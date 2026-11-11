//! Typed progress_bar component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_progress_bar_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let mut args = inv.arguments;
    if inv.name == "ProgressBar" {
        let progress = args
            .remove("progress")
            .map(|progress| {
                lower_expr(
                    &progress,
                    Some(&Type::Numeric(NumericType::Float64)),
                    &cx.exprs(false),
                )
            })
            .transpose()?;
        lower_progress_bar(progress, args, cx)
    } else {
        let progress = args
            .remove("progress")
            .map(|progress| {
                lower_expr(
                    &progress,
                    Some(&Type::Numeric(NumericType::Float64)),
                    &cx.exprs(false),
                )
            })
            .transpose()?;
        let tint = args
            .remove("tint")
            .map(|value| lower_color_expression(value, "ProgressRing tint", cx))
            .transpose()?;
        let track_color = args
            .remove("trackColor")
            .map(|value| lower_color_expression(value, "ProgressRing trackColor", cx))
            .transpose()?;
        let size = optional_dimension(args.remove("size"), "ProgressRing size", None, cx.themes)?;
        let thickness = optional_dimension(
            args.remove("thickness"),
            "ProgressRing thickness",
            None,
            cx.themes,
        )?;
        let android_wave = args
            .remove("androidWave")
            .map(|value| lower_expr(&value, Some(&Type::Bool), &cx.exprs(false)))
            .transpose()?;
        let style = if tint.is_some()
            || track_color.is_some()
            || size.is_some()
            || thickness.is_some()
            || android_wave.is_some()
        {
            Some(Box::new(nexa_ir::ProgressRingStyle {
                tint,
                track_color,
                size,
                thickness,
                android_wave,
            }))
        } else {
            None
        };
        Ok(Node::ProgressRing { progress, style })
    }
}
