use nexa_codegen::SourceWriter;
use nexa_ir::{AnimationSpec, ButtonShape, ColorValue, Expr, ShapeMeasurement};

use crate::generator::{colors, engine::expressions::expression, utils::number};

pub(super) struct ShapeDimensions<'a> {
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) width_expression: Option<&'a Expr>,
    pub(super) height_expression: Option<&'a Expr>,
    pub(super) measurement: ShapeMeasurement,
}

pub(super) fn render(
    shape: ButtonShape,
    fill: ColorValue,
    dimensions: ShapeDimensions<'_>,
    animation: Option<AnimationSpec>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let shape = match shape {
        ButtonShape::Capsule => "androidx.compose.foundation.shape.CircleShape".to_owned(),
        ButtonShape::Circle => "NexaCenteredCircleShape".to_owned(),
        ButtonShape::Rounded(radius) => format!(
            "androidx.compose.foundation.shape.RoundedCornerShape({}.dp)",
            number(radius)
        ),
    };
    let fill = colors::expression(fill);
    let has_animation = animation.is_some()
        && (dimensions.width_expression.is_some() || dimensions.height_expression.is_some());
    let use_measurement = has_animation || !dimensions.measurement.is_none();
    let mut dimension = |axis: &str, points: f32, dynamic: Option<&Expr>, measure: bool| {
        let Some(dynamic) = dynamic else {
            return format!("{axis} = {}.dp", number(points));
        };
        let value = format!("({}).dp.coerceAtLeast(0.dp)", expression(dynamic));
        if let Some(animation) = animation {
            let state = format!("nexaShapeSize{}", out.next_id());
            let spec = super::controls::kotlin_animation(animation);
            out.line_at(depth, format_args!(
                "val {state} = androidx.compose.animation.core.animateDpAsState(targetValue = {value}, animationSpec = {spec}, label = \"Nexa shape {axis}\")"
            ));
            format!("{axis}Reader = {{ {state}.value.value }}")
        } else if measure {
            format!("{axis}Reader = {{ {value}.value }}")
        } else {
            format!("{axis} = {value}")
        }
    };
    let width = dimension(
        "width",
        dimensions.width,
        dimensions.width_expression,
        matches!(
            dimensions.measurement,
            ShapeMeasurement::Width | ShapeMeasurement::Both
        ),
    );
    let height = dimension(
        "height",
        dimensions.height,
        dimensions.height_expression,
        matches!(
            dimensions.measurement,
            ShapeMeasurement::Height | ShapeMeasurement::Both
        ),
    );
    if use_measurement {
        out.line_at(
            depth,
            format_args!(
                "NexaMeasuredShapePrimitive(fill = {fill}, shape = {shape}, {width}, {height})"
            ),
        );
    } else {
        out.line_at(depth, format_args!(
            "Spacer(modifier = Modifier.size({width}, {height}).background({fill}, shape = {shape}))"
        ));
    }
}
