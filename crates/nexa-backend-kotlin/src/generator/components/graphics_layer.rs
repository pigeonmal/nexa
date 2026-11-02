use nexa_codegen::SourceWriter;
use nexa_ir::{Expr, ViewEffects};

use crate::generator::utils::{number, spaces};

#[derive(Default)]
struct CompositionValues {
    screen_width: bool,
    screen_height: bool,
    system_insets: [bool; 4],
    screen_width_dp: bool,
    screen_height_dp: bool,
}

impl CompositionValues {
    fn from_effects(effects: &ViewEffects) -> Self {
        let mut values = Self::default();
        for expression in [
            effects.scale_expression.as_deref(),
            effects.scale_y_expression.as_deref(),
            effects.opacity_expression.as_deref(),
            effects.offset_y_expression.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            nexa_ir::walk::walk_expression(expression, &mut |node| match node {
                Expr::ScreenWidth => values.screen_width = true,
                Expr::ScreenHeight => values.screen_height = true,
                Expr::SystemInset(edge) => {
                    values.system_insets[match edge {
                        nexa_ir::adaptive_layout::LayoutEdge::Top => 0,
                        nexa_ir::adaptive_layout::LayoutEdge::Bottom => 1,
                        nexa_ir::adaptive_layout::LayoutEdge::Leading => 2,
                        nexa_ir::adaptive_layout::LayoutEdge::Trailing => 3,
                    }] = true
                }
                Expr::IsRegularWidth | Expr::IsCompactWidth => values.screen_width_dp = true,
                Expr::IsRegularHeight | Expr::IsCompactHeight => values.screen_height_dp = true,
                _ => {}
            });
        }
        values
    }

    fn is_empty(&self) -> bool {
        !self.screen_width
            && !self.screen_height
            && !self.system_insets.iter().any(|used| *used)
            && !self.screen_width_dp
            && !self.screen_height_dp
    }

    fn write_bindings(&self, depth: usize, out: &mut SourceWriter, multiline: bool) {
        let mut write = |name: &str, value: &str| {
            if multiline {
                out.push_str(&format!("\n{}val {name} = {value}", spaces(depth)));
            } else {
                out.push_str(&format!("val {name} = {value}; "));
            }
        };
        if self.screen_width {
            write("nexaGraphicsLayerScreenWidth", SCREEN_WIDTH_EXPRESSION);
        }
        if self.screen_height {
            write("nexaGraphicsLayerScreenHeight", SCREEN_HEIGHT_EXPRESSION);
        }
        for (index, edge) in ["Top", "Bottom", "Leading", "Trailing"]
            .into_iter()
            .enumerate()
        {
            if self.system_insets[index] {
                write(
                    &format!("nexaGraphicsLayerSystemInset{edge}"),
                    &format!(
                        "LocalNexaWindowMetrics.current.systemInset(NexaSafeAreaEdge.{edge}, androidx.compose.ui.platform.LocalLayoutDirection.current).value.toDouble()"
                    ),
                );
            }
        }
        if self.screen_width_dp {
            write(
                "nexaGraphicsLayerScreenWidthDp",
                "androidx.compose.ui.platform.LocalConfiguration.current.screenWidthDp",
            );
        }
        if self.screen_height_dp {
            write(
                "nexaGraphicsLayerScreenHeightDp",
                "androidx.compose.ui.platform.LocalConfiguration.current.screenHeightDp",
            );
        }
    }

    fn capture(&self, expression: &Expr) -> String {
        let mut rendered = crate::generator::engine::expressions::expression(expression);
        if self.screen_width {
            rendered = rendered.replace(SCREEN_WIDTH_EXPRESSION, "nexaGraphicsLayerScreenWidth");
        }
        if self.screen_height {
            rendered = rendered.replace(SCREEN_HEIGHT_EXPRESSION, "nexaGraphicsLayerScreenHeight");
        }
        for (index, edge) in ["Top", "Bottom", "Leading", "Trailing"]
            .into_iter()
            .enumerate()
        {
            if self.system_insets[index] {
                rendered = rendered.replace(
                    &format!("LocalNexaWindowMetrics.current.systemInset(NexaSafeAreaEdge.{edge}, androidx.compose.ui.platform.LocalLayoutDirection.current).value.toDouble()"),
                    &format!("nexaGraphicsLayerSystemInset{edge}"),
                );
            }
        }
        if self.screen_width_dp {
            rendered = rendered.replace(
                "LocalConfiguration.current.screenWidthDp",
                "nexaGraphicsLayerScreenWidthDp",
            );
        }
        if self.screen_height_dp {
            rendered = rendered.replace(
                "LocalConfiguration.current.screenHeightDp",
                "nexaGraphicsLayerScreenHeightDp",
            );
        }
        rendered
    }
}

const SCREEN_WIDTH_EXPRESSION: &str = "LocalNexaWindowMetrics.current.viewportWidthDp.toDouble()";
const SCREEN_HEIGHT_EXPRESSION: &str = "LocalNexaWindowMetrics.current.viewportHeightDp.toDouble()";

pub(crate) fn append_multiline(effects: &ViewEffects, depth: usize, out: &mut SourceWriter) {
    let values = CompositionValues::from_effects(effects);
    let indent = spaces(depth);
    if values.is_empty() {
        append_multiline_layer(effects, &values, depth, false, out);
        return;
    }

    out.push_str(&format!("\n{indent}.then(Modifier.composed {{"));
    values.write_bindings(depth + 1, out, true);
    append_multiline_layer(effects, &values, depth + 1, true, out);
    out.push_str(&format!("\n{indent}}})"));
}

fn append_multiline_layer(
    effects: &ViewEffects,
    values: &CompositionValues,
    depth: usize,
    nested_modifier: bool,
    out: &mut SourceWriter,
) {
    let scale_x = scale_x(effects, values);
    let scale_y = scale_y(effects, values, &scale_x);
    let alpha = effects
        .opacity_expression
        .as_deref()
        .map(|value| format!("({}).toFloat().coerceIn(0f, 1f)", values.capture(value)))
        .unwrap_or_else(|| "1f".to_owned());
    let translation_y = effects
        .offset_y_expression
        .as_deref()
        .map(|value| format!("({}).toFloat().dp.toPx()", values.capture(value),))
        .unwrap_or_else(|| "0f".to_owned());
    let rotation = effects
        .rotation
        .map(number)
        .unwrap_or_else(|| "0.0".to_owned());
    let indent = spaces(depth);
    let modifier = if nested_modifier {
        "Modifier.graphicsLayer"
    } else {
        ".graphicsLayer"
    };
    out.push_str(&format!(
        "\n{indent}{modifier} {{\n{indent}    scaleX = {scale_x}\n{indent}    scaleY = {scale_y}\n{indent}    alpha = {alpha}\n{indent}    translationY = {translation_y}\n{indent}    rotationZ = {rotation}f\n{indent}}}"
    ));
}

pub(crate) fn append_inline(effects: &ViewEffects, out: &mut SourceWriter) {
    let values = CompositionValues::from_effects(effects);
    let scale_x = scale_x(effects, &values);
    let scale_y = scale_y(effects, &values, &scale_x);
    let alpha = effects
        .opacity_expression
        .as_deref()
        .map(|value| format!("({}).toFloat().coerceIn(0f, 1f)", values.capture(value)))
        .unwrap_or_else(|| "1f".to_owned());
    let translation_y = effects
        .offset_y_expression
        .as_deref()
        .map(|value| format!("({}).toFloat().dp.toPx()", values.capture(value),))
        .unwrap_or_else(|| "0f".to_owned());
    let rotation = effects
        .rotation
        .map(number)
        .unwrap_or_else(|| "0.0".to_owned());
    if values.is_empty() {
        out.push_str(&format!(
            ".graphicsLayer {{ scaleX = {scale_x}; scaleY = {scale_y}; alpha = {alpha}; translationY = {translation_y}; rotationZ = {rotation}f }}"
        ));
    } else {
        out.push_str(".then(Modifier.composed { ");
        values.write_bindings(0, out, false);
        out.push_str(&format!(
            "Modifier.graphicsLayer {{ scaleX = {scale_x}; scaleY = {scale_y}; alpha = {alpha}; translationY = {translation_y}; rotationZ = {rotation}f }} }})"
        ));
    }
}

fn scale_x(effects: &ViewEffects, values: &CompositionValues) -> String {
    effects
        .scale_expression
        .as_deref()
        .map(|value| format!("({}).toFloat()", values.capture(value)))
        .or_else(|| effects.scale.map(|value| format!("{}f", number(value))))
        .unwrap_or_else(|| "1.0f".to_owned())
}

fn scale_y(effects: &ViewEffects, values: &CompositionValues, scale_x: &str) -> String {
    effects
        .scale_y_expression
        .as_deref()
        .map(|value| format!("({}).toFloat()", values.capture(value)))
        .or_else(|| effects.scale_y.map(|value| format!("{}f", number(value))))
        .unwrap_or_else(|| scale_x.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{append_inline, append_multiline};
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, NumericType, Type, ViewEffects};

    #[test]
    fn composable_screen_dimensions_are_captured_outside_the_layer_block() {
        let effects = ViewEffects {
            scale_y_expression: Some(Box::new(Expr::ScreenHeight)),
            ..ViewEffects::default()
        };
        let mut output = SourceWriter::new();
        append_multiline(&effects, 0, &mut output);

        let generated = output.finish();
        assert!(generated.contains(".then(Modifier.composed {"));
        assert!(generated.contains("val nexaGraphicsLayerScreenHeight = "));
        assert!(generated.contains("scaleY = (nexaGraphicsLayerScreenHeight).toFloat()"));
        let layer = generated.find(".graphicsLayer {").unwrap();
        assert!(!generated[layer..].contains("LocalWindowInfo.current"));
    }

    #[test]
    fn animation_state_stays_in_the_layer_block_when_context_is_captured() {
        let effects = ViewEffects {
            scale_y_expression: Some(Box::new(Expr::Arithmetic {
                op: nexa_ir::ArithmeticOp::Multiply,
                left: Box::new(Expr::AnimatedState(
                    "progress".to_owned(),
                    Type::Numeric(NumericType::Float64),
                )),
                right: Box::new(Expr::ScreenHeight),
                ty: NumericType::Float64,
            })),
            ..ViewEffects::default()
        };
        let mut output = SourceWriter::new();
        append_inline(&effects, &mut output);

        let generated = output.finish();
        assert!(generated.contains("val nexaGraphicsLayerScreenHeight = "));
        assert!(generated.contains("nexa_progressAnimated.value.toDouble()"));
        assert!(generated.contains("nexaGraphicsLayerScreenHeight"));
        let layer = generated.find("Modifier.graphicsLayer {").unwrap();
        assert!(!generated[layer..].contains("LocalWindowInfo.current.containerSize.height"));
    }
}
