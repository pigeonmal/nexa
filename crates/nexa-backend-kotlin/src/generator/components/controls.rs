use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{
    Action, AnimationSpec, CollectionMutation, ErrorCatchArm, Expr, HapticStyle, Module, Node,
    SystemIcon, TaskExecutor,
};

use crate::generator::{
    components::render_children, expressions::expression, features::Features, utils::indent,
};

use crate::generator::engine::imports::ImportSet;
use crate::generator::engine::types::kotlin_type;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.uses_haptic, "android.view.HapticFeedbackConstants");
    imports.add(
        features.uses_clickable || features.uses_link,
        "androidx.compose.foundation.clickable",
    );
    imports.add(
        features.uses_long_press || features.uses_double_tap || features.uses_context_menu,
        "androidx.compose.foundation.combinedClickable",
    );
    imports.add(
        features.facts.ui.pressable.fill_max_size,
        "androidx.compose.foundation.layout.fillMaxSize",
    );
    imports.add(
        features.uses_long_press,
        "androidx.compose.runtime.CompositionLocalProvider",
    );
    imports.add(
        features.uses_long_press,
        "androidx.compose.runtime.remember",
    );
    imports.add(
        features.uses_long_press,
        "androidx.compose.ui.platform.LocalViewConfiguration",
    );
    imports.add(
        features.uses_long_press,
        "androidx.compose.ui.platform.ViewConfiguration",
    );
    imports.add(
        features.uses_context_menu,
        "androidx.compose.material3.DropdownMenu",
    );
    imports.add(
        features.uses_context_menu,
        "androidx.compose.material3.DropdownMenuItem",
    );
    imports.add(
        features.uses_context_menu,
        "androidx.compose.runtime.mutableStateOf",
    );
    imports.add(
        features.uses_context_menu,
        "androidx.compose.runtime.remember",
    );
    imports.add(
        features.uses_context_menu,
        "androidx.compose.material3.Text",
    );
    imports.add(
        features.uses_drag && !features.uses_pinch,
        "androidx.compose.foundation.gestures.detectDragGestures",
    );
    imports.add(
        features.uses_pinch,
        "androidx.compose.foundation.gestures.detectTransformGestures",
    );
    imports.add(
        features.uses_drag || features.uses_pinch,
        "androidx.compose.ui.input.pointer.pointerInput",
    );
    imports.add(
        features.uses_drag_velocity && !features.uses_pinch,
        "androidx.compose.ui.input.pointer.util.VelocityTracker",
    );
    imports.add(
        features.uses_drag_velocity && !features.uses_pinch,
        "androidx.compose.ui.input.pointer.util.addPointerInputChange",
    );
    imports.add(
        features.uses_drag_velocity && features.uses_pinch,
        "android.os.SystemClock",
    );
    imports.add(
        features.uses_drag,
        "androidx.compose.ui.platform.LocalDensity",
    );
    imports.add(features.uses_button, "androidx.compose.material3.Button");
    imports.add(
        features.uses_button_loading,
        "androidx.compose.material3.CircularProgressIndicator",
    );
    imports.add(features.uses_switch, "androidx.compose.material3.Switch");
    imports.add(features.uses_slider, "androidx.compose.material3.Slider");
    imports.add(
        features.uses_segmented_control,
        "androidx.compose.material3.SegmentedButton",
    );
    imports.add(
        features.uses_segmented_control,
        "androidx.compose.material3.SegmentedButtonDefaults",
    );
    imports.add(
        features.uses_segmented_control,
        "androidx.compose.material3.SingleChoiceSegmentedButtonRow",
    );
    imports.add(
        features.uses_picker,
        "androidx.compose.foundation.layout.Box",
    );
    imports.add(
        features.uses_picker,
        "androidx.compose.foundation.layout.Row",
    );
    imports.add(
        features.uses_picker,
        "androidx.compose.foundation.layout.Spacer",
    );
    imports.add(features.uses_picker, "androidx.compose.ui.Alignment");
    imports.add(features.uses_picker, "androidx.compose.ui.Modifier");
    imports.add(
        features.uses_picker,
        "androidx.compose.material3.DropdownMenu",
    );
    imports.add(
        features.uses_picker,
        "androidx.compose.material3.DropdownMenuItem",
    );
    imports.add(
        features.uses_picker,
        "androidx.compose.material3.TextButton",
    );
    imports.add(features.uses_picker, "androidx.compose.material3.Icon");
    imports.add(
        features.uses_picker,
        "androidx.compose.ui.graphics.vector.ImageVector",
    );
    imports.add(
        features.uses_picker,
        "androidx.compose.runtime.mutableStateOf",
    );
    imports.add(features.uses_picker, "androidx.compose.runtime.remember");
    imports.add(
        features.uses_date_picker,
        "androidx.compose.material3.DatePicker",
    );
    imports.add(
        features.uses_date_picker,
        "androidx.compose.foundation.layout.Column",
    );
    imports.add(
        features.uses_date_picker,
        "androidx.compose.material3.Switch",
    );
    imports.add(
        features.uses_date_picker,
        "androidx.compose.material3.TimePicker",
    );
    imports.add(
        features.uses_date_picker,
        "androidx.compose.material3.rememberDatePickerState",
    );
    imports.add(
        features.uses_date_picker,
        "androidx.compose.material3.rememberTimePickerState",
    );
    imports.add(
        features.uses_date_picker,
        "androidx.compose.runtime.LaunchedEffect",
    );
    imports.add(features.uses_date_picker, "java.util.Calendar");
    imports.add(
        features.uses_progress_bar,
        "androidx.compose.material3.LinearProgressIndicator",
    );
    imports.add(
        features.uses_progress_ring,
        "androidx.compose.material3.CircularProgressIndicator",
    );
    imports.add(
        features.uses_button || features.uses_segmented_control || features.uses_picker,
        "androidx.compose.material3.Text",
    );
    imports.add(
        features.uses_pressable || features.uses_accessibility_role,
        "androidx.compose.ui.semantics.Role",
    );
    imports.add(
        features.uses_haptic,
        "androidx.compose.ui.platform.LocalView",
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_button(
    label: &Expr,
    icon: Option<&SystemIcon>,
    loading: Option<&Expr>,
    disabled: Option<&Expr>,
    tint: Option<&nexa_ir::ColorExpression>,
    actions: &[Action],
    depth: usize,
    out: &mut SourceWriter,
) {
    if let Some(loading) = loading {
        indent(out, depth);
        out.push_str("Button(onClick = {");
        if actions.is_empty() {
            out.push_str(" }, enabled = !");
            out.push_str(&expression(loading));
        } else {
            out.push('\n');
            render_actions(actions, depth + 1, out);
            indent(out, depth);
            out.push_str("}, enabled = !");
            out.push_str(&expression(loading));
        }
        if let Some(disabled) = disabled {
            out.push_str(" && !");
            out.push_str(&expression(disabled));
        }
        append_button_tint(tint, out);
        out.push_str(") {");
        out.push('\n');
        out.line_at(depth + 1, format_args!("if ({}) {{", expression(loading)));
        indent(out, depth + 2);
        out.push_str("CircularProgressIndicator()\n");
        indent(out, depth + 1);
        out.push_str("} else {\n");
        render_button_content(label, icon, depth + 2, out);
        indent(out, depth + 1);
        out.push('}');
        out.push('\n');
        indent(out, depth);
        out.push('}');
        return;
    }
    indent(out, depth);
    out.push_str("Button(onClick = {");
    if actions.is_empty() {
        out.push_str(" }");
    } else {
        out.push('\n');
        render_actions(actions, depth + 1, out);
        indent(out, depth);
        out.push('}');
    }
    if let Some(disabled) = disabled {
        out.push_str(", enabled = !");
        out.push_str(&expression(disabled));
    }
    append_button_tint(tint, out);
    out.push_str(") {\n");
    render_button_content(label, icon, depth + 1, out);
    indent(out, depth);
    out.push('}');
}

fn append_button_tint(tint: Option<&nexa_ir::ColorExpression>, out: &mut SourceWriter) {
    if let Some(tint) = tint {
        out.push_str(&format!(
            ", colors = androidx.compose.material3.ButtonDefaults.buttonColors(containerColor = {})",
            crate::generator::colors::expression_for_color(tint)
        ));
    }
}

fn render_button_content(
    label: &Expr,
    icon: Option<&SystemIcon>,
    depth: usize,
    out: &mut SourceWriter,
) {
    if let Some(icon) = icon {
        out.line_at(
            depth,
            format_args!(
                "Icon(imageVector = {}, contentDescription = null)",
                icon.material_reference()
            ),
        );
    }
    out.line_at(depth, format_args!("Text({})", expression(label)));
}

pub(crate) fn render_switch(
    state: &str,
    label: &nexa_ir::Expr,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.text_at(depth, format_args!("Switch(checked = {}, onCheckedChange = {{ {} = it }}, modifier = Modifier.semantics {{ contentDescription = {} }})",
        state_name(state),
        state_name(state),
        expression(label)
    ));
}

pub(crate) fn render_slider(
    state: &str,
    animated: bool,
    min: f64,
    max: f64,
    step: f64,
    depth: usize,
    out: &mut SourceWriter,
) {
    let intervals = ((max - min) / step).round() as i32;
    let value = if animated {
        format!("{}Animated", state_name(state))
    } else {
        state_name(state)
    };
    out.line_at(
        depth,
        format_args!(
            "Slider(value = {value}.toFloat(), onValueChange = {{ {} = it.toDouble() }}, valueRange = {}f..{}f, steps = {})",
            state_name(state),
            min as f32,
            max as f32,
            intervals.saturating_sub(1).max(0),
        ),
    );
}

pub(crate) fn render_progress_bar(progress: &Expr, depth: usize, out: &mut SourceWriter) {
    out.line_at(
        depth,
        format_args!(
            "LinearProgressIndicator(progress = {{ ({}.toFloat()).coerceIn(0f, 1f) }})",
            expression(progress)
        ),
    );
}

pub(crate) fn render_progress_ring(progress: &Expr, depth: usize, out: &mut SourceWriter) {
    out.line_at(
        depth,
        format_args!(
            "CircularProgressIndicator(progress = {{ ({}.toFloat()).coerceIn(0f, 1f) }})",
            expression(progress)
        ),
    );
}

pub(crate) fn render_segmented_control(
    items: &Expr,
    state: &str,
    depth: usize,
    out: &mut SourceWriter,
) {
    let options = expression(items);
    let selected = state_name(state);
    out.line_at(depth, format_args!("SingleChoiceSegmentedButtonRow {{"));
    out.line_at(depth + 1, format_args!("val options = {options}"));
    out.line_at(
        depth + 1,
        format_args!("options.forEachIndexed {{ index, item ->"),
    );
    out.line_at(depth + 2, format_args!("SegmentedButton("));
    out.line_at(depth + 3, format_args!("selected = {selected} == item,"));
    out.line_at(
        depth + 3,
        format_args!("onClick = {{ {selected} = item }},"),
    );
    out.line_at(
        depth + 3,
        format_args!(
            "shape = SegmentedButtonDefaults.itemShape(index = index, count = options.size),"
        ),
    );
    out.line_at(depth + 2, format_args!(") {{"));
    out.line_at(depth + 3, format_args!("Text(item)"));
    out.line_at(depth + 2, format_args!("}}"));
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(depth, format_args!("}}"));
}

pub(crate) fn render_picker(
    items: &Expr,
    state: &str,
    icon: Option<&SystemIcon>,
    label: Option<&Expr>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let selected = state_name(state);
    let icon = icon.map_or_else(|| "null".to_owned(), SystemIcon::material_reference);
    let label = label.map_or_else(|| "null".to_owned(), expression);
    out.line_at(
        depth,
        format_args!(
            "nexaPickerMenu({}, {selected}, {icon}, {label}) {{ {selected} = it }}",
            expression(items),
        ),
    );
}

pub(crate) fn render_date_picker(
    timestamp_state: &str,
    has_time_state: &str,
    depth: usize,
    out: &mut SourceWriter,
) {
    let timestamp = state_name(timestamp_state);
    let has_time = state_name(has_time_state);
    let pad = "    ".repeat(depth);
    out.push_str(&format!(
        "Column {{\n{pad}    val nexaDatePickerState = rememberDatePickerState(initialSelectedDateMillis = {timestamp})\n{pad}    val nexaTimePickerState = rememberTimePickerState()\n{pad}    Switch(checked = {has_time}, onCheckedChange = {{ {has_time} = it }}, modifier = Modifier.semantics {{ contentDescription = \"Time\" }})\n{pad}    DatePicker(state = nexaDatePickerState, title = {{ Text(\"Select Date\") }})\n{pad}    if ({has_time}) TimePicker(state = nexaTimePickerState)\n{pad}    LaunchedEffect({timestamp}) {{ nexaDatePickerState.selectedDateMillis = {timestamp} }}\n{pad}    LaunchedEffect(nexaDatePickerState.selectedDateMillis, nexaTimePickerState.hour, nexaTimePickerState.minute, {has_time}) {{\n{pad}        nexaDatePickerState.selectedDateMillis?.let {{ selectedMillis ->\n{pad}            val calendar = Calendar.getInstance().apply {{\n{pad}                timeInMillis = selectedMillis\n{pad}                set(Calendar.HOUR_OF_DAY, if ({has_time}) nexaTimePickerState.hour else 0)\n{pad}                set(Calendar.MINUTE, if ({has_time}) nexaTimePickerState.minute else 0)\n{pad}                set(Calendar.SECOND, 0)\n{pad}                set(Calendar.MILLISECOND, 0)\n{pad}            }}\n{pad}            {timestamp} = calendar.timeInMillis\n{pad}        }}\n{pad}    }}\n{pad}}}"
    ));
}

pub(crate) fn render_picker_helper(out: &mut SourceWriter) {
    out.push_str(
        "@Composable\ninternal fun nexaPickerMenu(\n    items: List<String>,\n    selected: String,\n    icon: ImageVector?,\n    label: String?,\n    onSelectionChanged: (String) -> Unit,\n) {\n    val expanded = remember { mutableStateOf(false) }\n    Row(verticalAlignment = Alignment.CenterVertically) {\n        if (label != null) {\n            Text(label)\n            Spacer(Modifier.weight(1f))\n        }\n        Box {\n            TextButton(onClick = { expanded.value = true }) {\n                if (icon != null) {\n                    Icon(imageVector = icon, contentDescription = selected)\n                } else {\n                    Text(selected)\n                }\n            }\n            DropdownMenu(\n                expanded = expanded.value,\n                onDismissRequest = { expanded.value = false },\n            ) {\n                items.forEach { item ->\n                    DropdownMenuItem(\n                        text = { Text(item) },\n                        onClick = {\n                            onSelectionChanged(item)\n                            expanded.value = false\n                        },\n                    )\n                }\n            }\n        }\n    }\n}\n\n",
    );
}

pub(crate) fn render_pressable(
    node: &Node,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let Node::Pressable {
        disabled,
        haptic,
        fill_max_size,
        children,
        actions,
        double_tap_actions,
        long_press_duration_ms,
        long_press_actions,
        context_menu,
        drag_parameters,
        drag_actions,
        pinch_parameter,
        pinch_actions,
    } = node
    else {
        return;
    };
    let has_context_menu = !context_menu.is_empty();
    let has_double_tap = !double_tap_actions.is_empty();
    let has_combined_clickable =
        !long_press_actions.is_empty() || has_double_tap || has_context_menu;
    let modifier = if has_combined_clickable {
        "combinedClickable"
    } else {
        "clickable"
    };
    let has_long_press = !long_press_actions.is_empty();
    let has_drag = drag_parameters.len() == 4;
    let has_pinch = pinch_parameter.is_some();
    let has_drag_velocity = has_drag
        && nexa_ir::walk::actions_reference_state(
            drag_actions,
            &[drag_parameters[2].as_str(), drag_parameters[3].as_str()],
        );
    let enabled = format!("!({})", expression(disabled));
    let mut depth = depth;
    if has_long_press {
        let duration = expression(long_press_duration_ms);
        out.line_at(depth, format_args!("CompositionLocalProvider("));
        out.line_at(
            depth + 1,
            format_args!(
                "LocalViewConfiguration provides remember(LocalViewConfiguration.current, {duration}) {{"
            ),
        );
        out.line_at(
            depth + 2,
            format_args!("object : ViewConfiguration by LocalViewConfiguration.current {{"),
        );
        out.line_at(
            depth + 3,
            format_args!(
                "override val longPressTimeoutMillis: Long = ({duration}).toLong().coerceAtLeast(1L)"
            ),
        );
        out.line_at(depth + 2, format_args!("}}"));
        out.line_at(depth + 1, format_args!("}}"));
        out.line_at(depth, format_args!(") {{"));
        depth += 1;
    }
    if has_drag {
        out.line_at(
            depth,
            format_args!("val nexaDragDensity = LocalDensity.current.density"),
        );
    }
    if has_context_menu {
        out.line_at(depth, format_args!("Box {{"));
        depth += 1;
        out.line_at(
            depth,
            format_args!("val nexaContextMenuExpanded = remember {{ mutableStateOf(false) }}"),
        );
    }
    out.text_at(
        depth,
        format_args!("Box(\n{}    modifier = Modifier", "    ".repeat(depth)),
    );
    if *fill_max_size {
        out.push_str(".fillMaxSize()");
    }
    if has_pinch {
        out.push_str(".pointerInput(");
        if has_drag {
            out.push_str("nexaDragDensity, ");
        }
        out.push_str(&format!("{enabled}) {{\n"));
        out.line_at(depth + 2, format_args!("if ({enabled}) {{"));
        if has_drag {
            out.line_at(depth + 3, format_args!("var nexaTranslationX = 0f"));
            out.line_at(depth + 3, format_args!("var nexaTranslationY = 0f"));
            if has_drag_velocity {
                out.line_at(depth + 3, format_args!("var nexaLastGestureTime = 0L"));
            }
        }
        out.line_at(
            depth + 3,
            format_args!("detectTransformGestures {{ _, nexaPan, nexaZoomChange, _ ->"),
        );
        if has_drag {
            out.line_at(
                depth + 4,
                format_args!("if (nexaPan.x != 0f || nexaPan.y != 0f) {{"),
            );
            out.line_at(depth + 5, format_args!("nexaTranslationX += nexaPan.x"));
            out.line_at(depth + 5, format_args!("nexaTranslationY += nexaPan.y"));
            let names = drag_parameters
                .iter()
                .map(|name| state_name(name))
                .collect::<Vec<_>>();
            out.line_at(
                depth + 5,
                format_args!(
                    "val {} = (nexaTranslationX / nexaDragDensity).toDouble()",
                    names[0]
                ),
            );
            out.line_at(
                depth + 5,
                format_args!(
                    "val {} = (nexaTranslationY / nexaDragDensity).toDouble()",
                    names[1]
                ),
            );
            if has_drag_velocity {
                out.line_at(
                    depth + 5,
                    format_args!("val nexaNow = SystemClock.uptimeMillis()"),
                );
                out.line_at(depth + 5, format_args!("val nexaElapsed = if (nexaLastGestureTime == 0L || nexaNow - nexaLastGestureTime > 100L) 0L else nexaNow - nexaLastGestureTime"));
                out.line_at(depth + 5, format_args!("nexaLastGestureTime = nexaNow"));
                out.line_at(depth + 5, format_args!("val nexaVelocityX = if (nexaElapsed == 0L) 0f else nexaPan.x * (1000f / nexaElapsed.toFloat())"));
                out.line_at(depth + 5, format_args!("val nexaVelocityY = if (nexaElapsed == 0L) 0f else nexaPan.y * (1000f / nexaElapsed.toFloat())"));
                out.line_at(
                    depth + 5,
                    format_args!(
                        "val {} = (nexaVelocityX / nexaDragDensity).toDouble()",
                        names[2]
                    ),
                );
                out.line_at(
                    depth + 5,
                    format_args!(
                        "val {} = (nexaVelocityY / nexaDragDensity).toDouble()",
                        names[3]
                    ),
                );
            }
            render_actions(drag_actions, depth + 5, out);
            out.line_at(depth + 4, format_args!("}}"));
        }
        out.line_at(depth + 4, format_args!("if (nexaZoomChange != 1f) {{"));
        let pinch_name = pinch_parameter
            .as_deref()
            .map(state_name)
            .unwrap_or_else(|| "nexaScaleFactor".to_owned());
        out.line_at(
            depth + 5,
            format_args!("val {pinch_name} = nexaZoomChange.toDouble()"),
        );
        render_actions(pinch_actions, depth + 5, out);
        out.line_at(depth + 4, format_args!("}}"));
        out.line_at(depth + 3, format_args!("}}"));
        out.line_at(depth + 2, format_args!("}}"));
        out.push('}');
    }
    if has_drag && !has_pinch {
        out.push_str(&format!(".pointerInput(nexaDragDensity, {enabled}) {{\n"));
        out.line_at(depth + 2, format_args!("if ({enabled}) {{"));
        if has_drag_velocity {
            out.line_at(
                depth + 3,
                format_args!("val nexaVelocityTracker = VelocityTracker()"),
            );
        }
        out.line_at(depth + 3, format_args!("var nexaTranslationX = 0f"));
        out.line_at(depth + 3, format_args!("var nexaTranslationY = 0f"));
        out.line_at(depth + 3, format_args!("detectDragGestures("));
        out.line_at(depth + 4, format_args!("onDragStart = {{"));
        out.line_at(depth + 5, format_args!("nexaTranslationX = 0f"));
        out.line_at(depth + 5, format_args!("nexaTranslationY = 0f"));
        if has_drag_velocity {
            out.line_at(
                depth + 5,
                format_args!("nexaVelocityTracker.resetTracking()"),
            );
            out.line_at(depth + 4, format_args!("}},"));
            out.line_at(
                depth + 4,
                format_args!("onDragEnd = {{ nexaVelocityTracker.resetTracking() }},"),
            );
            out.line_at(
                depth + 4,
                format_args!("onDragCancel = {{ nexaVelocityTracker.resetTracking() }},"),
            );
        } else {
            out.line_at(depth + 4, format_args!("}},"));
        }
        out.line_at(depth + 4, format_args!("onDrag = {{ change, dragAmount ->"));
        out.line_at(depth + 5, format_args!("nexaTranslationX += dragAmount.x"));
        out.line_at(depth + 5, format_args!("nexaTranslationY += dragAmount.y"));
        let names = drag_parameters
            .iter()
            .map(|name| state_name(name))
            .collect::<Vec<_>>();
        out.line_at(
            depth + 5,
            format_args!(
                "val {} = (nexaTranslationX / nexaDragDensity).toDouble()",
                names[0]
            ),
        );
        out.line_at(
            depth + 5,
            format_args!(
                "val {} = (nexaTranslationY / nexaDragDensity).toDouble()",
                names[1]
            ),
        );
        if has_drag_velocity {
            out.line_at(
                depth + 5,
                format_args!("nexaVelocityTracker.addPointerInputChange(change)"),
            );
            out.line_at(
                depth + 5,
                format_args!("val nexaVelocity = nexaVelocityTracker.calculateVelocity()"),
            );
            out.line_at(
                depth + 5,
                format_args!(
                    "val {} = (nexaVelocity.x / nexaDragDensity).toDouble()",
                    names[2]
                ),
            );
            out.line_at(
                depth + 5,
                format_args!(
                    "val {} = (nexaVelocity.y / nexaDragDensity).toDouble()",
                    names[3]
                ),
            );
        }
        render_actions(drag_actions, depth + 5, out);
        out.line_at(depth + 5, format_args!("change.consume()"));
        out.line_at(depth + 4, format_args!("}},"));
        out.line_at(depth + 3, format_args!(")"));
        out.line_at(depth + 2, format_args!("}}"));
        out.push('}');
    }
    out.push_str(&format!(".{modifier}(\n"));
    out.line_at(depth + 2, format_args!("enabled = {enabled},"));
    out.line_at(depth + 2, format_args!("interactionSource = null,"));
    out.line_at(depth + 2, format_args!("indication = null,"));
    out.line_at(depth + 2, format_args!("role = Role.Button,"));
    out.text_at(depth + 2, format_args!("onClick = {{"));
    if actions.is_empty() && haptic.is_none() {
        out.push_str(if has_combined_clickable {
            " },\n"
        } else {
            " }),\n"
        });
    } else {
        out.push('\n');
        if let Some(haptic) = haptic {
            render_haptic(*haptic, depth + 3, out);
        }
        render_actions(actions, depth + 3, out);
        indent(out, depth + 2);
        out.push_str(if has_combined_clickable {
            "},\n"
        } else {
            "}),\n"
        });
    }
    if has_long_press || has_context_menu {
        indent(out, depth + 2);
        out.push_str("onLongClick = {\n");
        if has_context_menu {
            out.line_at(
                depth + 3,
                format_args!("nexaContextMenuExpanded.value = true"),
            );
        }
        if let Some(haptic) = haptic {
            render_haptic(*haptic, depth + 3, out);
        }
        render_actions(long_press_actions, depth + 3, out);
        indent(out, depth + 2);
        out.push_str("},\n");
    }
    if has_double_tap {
        indent(out, depth + 2);
        out.push_str("onDoubleClick = {\n");
        if let Some(haptic) = haptic {
            render_haptic(*haptic, depth + 3, out);
        }
        render_actions(double_tap_actions, depth + 3, out);
        indent(out, depth + 2);
        out.push_str("},\n");
    }
    if has_combined_clickable {
        indent(out, depth + 1);
        out.push_str("),\n");
    }
    indent(out, depth);
    out.push_str(") {\n");
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
    if has_context_menu {
        out.push('\n');
        indent(out, depth);
        out.push_str("DropdownMenu(\n");
        out.line_at(
            depth + 1,
            format_args!("expanded = nexaContextMenuExpanded.value,"),
        );
        out.line_at(
            depth + 1,
            format_args!("onDismissRequest = {{ nexaContextMenuExpanded.value = false }},"),
        );
        out.line_at(depth, format_args!(") {{"));
        render_context_menu(context_menu, depth + 1, out);
        out.push('\n');
        indent(out, depth);
        out.push('}');
        depth -= 1;
        out.push('\n');
        indent(out, depth);
        out.push('}');
    }
    if has_long_press {
        indent(out, depth - 1);
        out.push('}');
    }
}

fn render_context_menu(nodes: &[Node], depth: usize, out: &mut SourceWriter) {
    for node in nodes {
        let Node::Button {
            label,
            icon,
            disabled,
            actions,
            ..
        } = node
        else {
            continue;
        };
        out.line_at(depth, format_args!("DropdownMenuItem("));
        out.line_at(
            depth + 1,
            format_args!("text = {{ Text({}) }},", expression(label)),
        );
        if let Some(icon) = icon {
            out.line_at(
                depth + 1,
                format_args!(
                    "leadingIcon = {{ Icon(imageVector = {}, contentDescription = null) }},",
                    icon.material_reference()
                ),
            );
        }
        let enabled = disabled.as_ref().map_or_else(
            || "true".to_owned(),
            |disabled| format!("!({})", expression(disabled)),
        );
        out.line_at(depth + 1, format_args!("enabled = {enabled},"));
        out.line_at(depth + 1, format_args!("onClick = {{"));
        render_actions(actions, depth + 2, out);
        out.line_at(
            depth + 2,
            format_args!("nexaContextMenuExpanded.value = false"),
        );
        out.line_at(depth + 1, format_args!("}},"));
        out.line_at(depth, format_args!(")"));
    }
}

fn render_haptic(style: HapticStyle, depth: usize, out: &mut SourceWriter) {
    indent(out, depth);
    let constant = match style {
        HapticStyle::Light => "KEYBOARD_TAP",
        HapticStyle::Medium => "VIRTUAL_KEY",
        HapticStyle::Heavy => "LONG_PRESS",
    };
    out.push_str(&format!(
        "nexaHapticView.performHapticFeedback(HapticFeedbackConstants.{constant})\n"
    ));
}

pub(crate) fn render_event_closure(
    parameters: &[String],
    actions: &[Action],
    depth: usize,
) -> String {
    let mut out = SourceWriter::new();
    out.push('{');
    if !parameters.is_empty() {
        out.push(' ');
        out.push_str(
            &parameters
                .iter()
                .map(|name| state_name(name))
                .collect::<Vec<_>>()
                .join(", "),
        );
        out.push_str(" ->");
    }
    if actions.is_empty() {
        out.push_str(" }");
    } else {
        out.push('\n');
        render_actions(actions, depth + 1, &mut out);
        indent(&mut out, depth);
        out.push('}');
    }
    out.finish()
}

pub(crate) fn render_actions(actions: &[Action], depth: usize, out: &mut SourceWriter) {
    for action in actions {
        match action {
            Action::Expression(value) => {
                out.line_at(depth, format_args!("{}", expression(value)));
            }
            Action::Let { name, ty, value } => {
                out.line_at(
                    depth,
                    format_args!(
                        "val {}: {} = {}",
                        state_name(name),
                        kotlin_type(ty),
                        expression(value)
                    ),
                );
            }
            Action::Return { value } => {
                out.line_at(depth, format_args!("return {}", expression(value)));
            }
            Action::Assign { name, value } => {
                out.line_at(
                    depth,
                    format_args!("{} = {}", state_name(name), expression(value)),
                );
            }
            Action::NativePropertyAssign {
                receiver,
                property,
                value,
            } => {
                out.line_at(
                    depth,
                    format_args!(
                        "{}.{} = {}",
                        expression(receiver),
                        property,
                        expression(value)
                    ),
                );
            }
            Action::NativeEventSubscribe {
                receiver,
                property,
                parameters,
                actions,
            } => {
                out.text_at(
                    depth,
                    format_args!("{}.{} = {{", expression(receiver), property),
                );
                if !parameters.is_empty() {
                    out.push(' ');
                    out.push_str(
                        &parameters
                            .iter()
                            .map(|name| state_name(name))
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                    out.push_str(" ->");
                }
                out.push('\n');
                render_actions(actions, depth + 1, out);
                indent(out, depth);
                out.push_str("}\n");
            }
            Action::NetworkStatusSubscribe { parameter, actions } => {
                out.text_at(
                    depth,
                    format_args!("NexaNetwork.onStatusChange(NexaRuntime.context()) {{ "),
                );
                out.push_str(&state_name(parameter));
                out.push_str(" ->\n");
                render_actions(actions, depth + 1, out);
                indent(out, depth);
                out.push_str("}\n");
            }
            Action::TaskLaunch {
                handle,
                executor,
                actions,
            } => {
                let handle = handle.as_ref().map(|handle| state_name(handle));
                if let Some(handle) = &handle {
                    out.line_at(depth, format_args!("{handle}?.cancel()"));
                }
                let dispatcher = match executor {
                    TaskExecutor::Main => "Dispatchers.Main.immediate",
                    TaskExecutor::Background => "Dispatchers.Default",
                };
                out.line_at(
                    depth,
                    format_args!(
                        "run {{\n{}val nexaTaskJob = nexaTaskScope.launch(context = {dispatcher} + nexaTaskExceptionHandler, start = CoroutineStart.LAZY) {{",
                        "    ".repeat(depth + 1)
                    ),
                );
                render_actions(actions, depth + 1, out);
                indent(out, depth + 1);
                out.push_str("}\n");
                if let Some(handle) = &handle {
                    out.line_at(depth + 1, format_args!("{handle} = nexaTaskJob"));
                }
                out.line_at(depth + 1, format_args!("nexaTaskJob.start()"));
                out.line_at(depth, format_args!("}}"));
            }
            Action::TaskCancel { handle } => {
                out.line_at(depth, format_args!("{}?.cancel()", state_name(handle)));
            }
            Action::WithAnimation {
                animation,
                animated_states,
                actions,
            } => {
                let spec = kotlin_animation(*animation);
                for state in animated_states {
                    out.line_at(
                        depth,
                        format_args!("{}AnimationSpec = {spec}", state_name(state)),
                    );
                }
                render_actions(actions, depth, out);
            }
            Action::CollectionMutation {
                name,
                operation,
                arguments,
            } => {
                indent(out, depth);
                let state = state_name(name);
                let rendered = arguments.iter().map(expression).collect::<Vec<_>>();
                match operation {
                    CollectionMutation::ArrayAppend => {
                        out.push_str(&format!("{state}.add({})\n", rendered[0]));
                    }
                    CollectionMutation::ArrayRemoveAt => {
                        out.push_str(&format!("{state}.removeAt({})\n", rendered[0]));
                    }
                    CollectionMutation::ArrayMove => {
                        let suffix = out.next_id();
                        let from = format!("nexaMoveFrom{suffix}");
                        let to = format!("nexaMoveTo{suffix}");
                        let moved_value = format!("nexaMovedValue{suffix}");
                        let nested_indent = "    ".repeat(depth + 1);
                        let line_indent = "    ".repeat(depth);
                        out.push_str(&format!(
                            "val {from} = {}\n{line_indent}val {to} = {}\n{line_indent}if ({from} in {state}.indices && {to} in {state}.indices && {from} != {to}) {{\n{nested_indent}val {moved_value} = {state}.removeAt({from})\n{nested_indent}{state}.add({to}, {moved_value})\n{line_indent}}}\n",
                            rendered[0], rendered[1],
                        ));
                    }
                    CollectionMutation::ArrayMoveSubset => {
                        let suffix = out.next_id();
                        let from = format!("nexaMoveFrom{suffix}");
                        let to = format!("nexaMoveTo{suffix}");
                        let original = format!("nexaOriginalSubset{suffix}");
                        let reordered = format!("nexaReorderedSubset{suffix}");
                        let cursor = format!("nexaSubsetCursor{suffix}");
                        let slots = format!("nexaSubsetSlots{suffix}");
                        let moved = format!("nexaSubsetMoved{suffix}");
                        let nested_indent = "    ".repeat(depth + 1);
                        let inner_indent = "    ".repeat(depth + 2);
                        let line_indent = "    ".repeat(depth);
                        out.push_str(&format!(
                            "val {from} = {}\n{line_indent}val {to} = {}\n{line_indent}val {original} = {}\n{line_indent}val {reordered} = {original}.toMutableList()\n{line_indent}if ({from} in {reordered}.indices && {to} in {reordered}.indices && {from} != {to}) {{\n{nested_indent}val {moved} = {reordered}.removeAt({from})\n{nested_indent}{reordered}.add({to}, {moved})\n{line_indent}}}\n{line_indent}var {cursor} = 0\n{line_indent}val {slots} = mutableListOf<Int>()\n{line_indent}for (nexaBackingIndex{suffix} in {state}.indices) {{\n{nested_indent}if ({cursor} < {original}.size && {state}[nexaBackingIndex{suffix}] == {original}[{cursor}]) {{\n{inner_indent}{slots}.add(nexaBackingIndex{suffix})\n{inner_indent}{cursor}++\n{nested_indent}}}\n{line_indent}}}\n{line_indent}if ({cursor} == {original}.size) {{\n{nested_indent}for (nexaSubsetIndex{suffix} in {original}.indices) {{\n{inner_indent}{state}[{slots}[nexaSubsetIndex{suffix}]] = {reordered}[nexaSubsetIndex{suffix}]\n{nested_indent}}}\n{line_indent}}}\n",
                            rendered[0], rendered[1], rendered[2],
                        ));
                    }
                    CollectionMutation::SetInsert => {
                        out.push_str(&format!("{state}.add({})\n", rendered[0]));
                    }
                    CollectionMutation::SetRemove => {
                        out.push_str(&format!("{state}.remove({})\n", rendered[0]));
                    }
                    CollectionMutation::MapSet => {
                        out.push_str(&format!("{state}[{}] = {}\n", rendered[0], rendered[1]));
                    }
                    CollectionMutation::MapRemove => {
                        out.push_str(&format!("{state}.remove({})\n", rendered[0]));
                    }
                    CollectionMutation::MapClear => {
                        out.push_str(&format!("{state}.clear()\n"));
                    }
                    // The state is a snapshot-state collection, so the value
                    // is replaced by swapping the contents. The overload is
                    // chosen from the static types.
                    CollectionMutation::Replace => {
                        out.push_str(&format!("nexaReplace({state}, {})\n", rendered[0]));
                    }
                }
            }
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                out.line_at(depth, format_args!("if ({}) {{", expression(condition)));
                render_actions(then_branch, depth + 1, out);
                if let Some(else_branch) = else_branch {
                    indent(out, depth);
                    out.push_str("} else {\n");
                    render_actions(else_branch, depth + 1, out);
                }
                indent(out, depth);
                out.push_str("}\n");
            }
            Action::For {
                name,
                iterable,
                body,
            } => {
                out.line_at(
                    depth,
                    format_args!(
                        "for ({} in nexaSnapshotValues({})) {{",
                        state_name(name),
                        expression(iterable)
                    ),
                );
                render_actions(body, depth + 1, out);
                indent(out, depth);
                out.push_str("}\n");
            }
            Action::ForMap {
                key_name,
                value_name,
                iterable,
                body,
            } => {
                out.line_at(
                    depth,
                    format_args!(
                        "for (({}, {}) in nexaSnapshotEntries({})) {{",
                        state_name(key_name),
                        state_name(value_name),
                        expression(iterable)
                    ),
                );
                render_actions(body, depth + 1, out);
                indent(out, depth);
                out.push_str("}\n");
            }
            Action::While { condition, body } => {
                out.line_at(depth, format_args!("while ({}) {{", expression(condition)));
                render_actions(body, depth + 1, out);
                indent(out, depth);
                out.push_str("}\n");
            }
            Action::TryCatch {
                body,
                error_catches,
                catch_body,
            } => {
                indent(out, depth);
                out.push_str("try {\n");
                render_actions(body, depth + 1, out);
                render_kotlin_error_catches(error_catches, catch_body.as_deref(), depth, out);
            }
            Action::Break => {
                indent(out, depth);
                out.push_str("break\n");
            }
            Action::Continue => {
                indent(out, depth);
                out.push_str("continue\n");
            }
        }
    }
}

fn kotlin_animation(animation: AnimationSpec) -> String {
    match animation {
        AnimationSpec::Spring { response, damping } => {
            let stiffness = (100.0 / f64::from(response).powi(2)) as f32;
            format!(
                "androidx.compose.animation.core.spring(dampingRatio = {damping}f, stiffness = {stiffness}f)"
            )
        }
        AnimationSpec::EaseIn => "androidx.compose.animation.core.tween(easing = androidx.compose.animation.core.FastOutLinearInEasing)".to_owned(),
        AnimationSpec::EaseOut => "androidx.compose.animation.core.tween(easing = androidx.compose.animation.core.LinearOutSlowInEasing)".to_owned(),
        AnimationSpec::EaseInOut => "androidx.compose.animation.core.tween(easing = androidx.compose.animation.core.FastOutSlowInEasing)".to_owned(),
        AnimationSpec::Linear => "androidx.compose.animation.core.tween(easing = androidx.compose.animation.core.LinearEasing)".to_owned(),
    }
}

fn render_kotlin_error_catches(
    catches: &[ErrorCatchArm],
    catch_all: Option<&[Action]>,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    if catches.is_empty() {
        out.push_str("} catch (_: Exception) {\n");
        if let Some(catch_all) = catch_all {
            render_actions(catch_all, depth + 1, out);
        }
        indent(out, depth);
        out.push_str("}\n");
        return;
    }

    out.push_str("} catch (error: Exception) {\n");
    indent(out, depth + 1);
    out.push_str("when (error) {\n");
    for arm in catches {
        indent(out, depth + 2);
        if arm.parameters.is_empty() {
            out.push_str(&format!("{}.{} -> {{\n", arm.error_type, arm.variant));
        } else {
            out.push_str(&format!("is {}.{} -> {{\n", arm.error_type, arm.variant));
            for (binding, property, _) in &arm.parameters {
                out.line_at(
                    depth + 3,
                    format_args!("val {} = error.{}", state_name(binding), property),
                );
            }
        }
        render_actions(&arm.body, depth + 3, out);
        indent(out, depth + 2);
        out.push_str("}\n");
    }
    indent(out, depth + 2);
    out.push_str("else -> {\n");
    if let Some(catch_all) = catch_all {
        render_actions(catch_all, depth + 3, out);
    } else {
        indent(out, depth + 3);
        out.push_str("throw error\n");
    }
    indent(out, depth + 2);
    out.push_str("}\n");
    indent(out, depth + 1);
    out.push_str("}\n");
    indent(out, depth);
    out.push_str("}\n");
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Action, CollectionMutation, Expr, NumericType, TaskExecutor, Type};

    use super::{render_actions, render_progress_bar, render_progress_ring, render_slider};

    #[test]
    fn renders_native_progress_indicators_with_bounded_float_values() {
        let progress = Expr::State(
            "progress".to_owned(),
            nexa_ir::Type::Numeric(nexa_ir::NumericType::Float64),
        );
        let mut output = SourceWriter::new();
        render_progress_bar(&progress, 0, &mut output);
        render_progress_ring(&progress, 0, &mut output);
        assert_eq!(
            output.as_str(),
            "LinearProgressIndicator(progress = { (nexa_progress.toFloat()).coerceIn(0f, 1f) })\nCircularProgressIndicator(progress = { (nexa_progress.toFloat()).coerceIn(0f, 1f) })\n"
        );
    }

    #[test]
    fn renders_slider_as_a_compose_float_binding_with_static_steps() {
        let mut output = SourceWriter::new();
        render_slider("volume", false, 0.0, 1.0, 0.1, 0, &mut output);
        assert_eq!(
            output.as_str(),
            "Slider(value = nexa_volume.toFloat(), onValueChange = { nexa_volume = it.toDouble() }, valueRange = 0f..1f, steps = 9)\n"
        );
    }

    #[test]
    fn renders_array_move_as_one_native_remove_and_insert() {
        let int = |raw: &str| Expr::Number {
            raw: raw.to_owned(),
            ty: NumericType::Int32,
        };
        let actions = [Action::CollectionMutation {
            name: "items".to_owned(),
            operation: CollectionMutation::ArrayMove,
            arguments: vec![int("2"), int("0")],
        }];
        let mut output = SourceWriter::new();

        render_actions(&actions, 0, &mut output);

        assert!(output.contains("val nexaMoveFrom"));
        assert!(output.contains("in nexa_items.indices"));
        assert!(output.contains("nexa_items.removeAt(nexaMoveFrom"));
        assert!(output.contains("nexa_items.add(nexaMoveTo"));
    }

    #[test]
    fn renders_map_clear_on_snapshot_state_map() {
        let actions = [Action::CollectionMutation {
            name: "quantities".to_owned(),
            operation: CollectionMutation::MapClear,
            arguments: Vec::new(),
        }];
        let mut output = SourceWriter::new();

        render_actions(&actions, 0, &mut output);

        assert_eq!(output.as_str(), "nexa_quantities.clear()\n");
    }

    #[test]
    fn renders_typed_native_event_handler_on_its_instance() {
        let actions = [Action::NativeEventSubscribe {
            receiver: Expr::State(
                "player".to_owned(),
                Type::Plugin {
                    namespace: "Video".to_owned(),
                    name: "VideoPlayer".to_owned(),
                },
            ),
            property: "onProgressChanged".to_owned(),
            parameters: vec!["position".to_owned(), "duration".to_owned()],
            actions: vec![Action::Assign {
                name: "latest".to_owned(),
                value: Expr::State("position".to_owned(), Type::Numeric(NumericType::Float64)),
            }],
        }];
        let mut output = SourceWriter::new();

        render_actions(&actions, 0, &mut output);

        assert_eq!(
            output.as_str(),
            "nexa_player.onProgressChanged = { nexa_position, nexa_duration ->\n    nexa_latest = nexa_position\n}\n"
        );
    }

    #[test]
    fn renders_network_status_subscription_as_a_native_callback() {
        let actions = [Action::NetworkStatusSubscribe {
            parameter: "online".to_owned(),
            actions: vec![Action::Assign {
                name: "connected".to_owned(),
                value: Expr::State("online".to_owned(), Type::Bool),
            }],
        }];
        let mut output = SourceWriter::new();

        render_actions(&actions, 0, &mut output);

        assert!(
            output.contains("NexaNetwork.onStatusChange(NexaRuntime.context()) { nexa_online ->")
        );
        assert!(output.contains("nexa_connected = nexa_online"));
    }

    #[test]
    fn renders_task_launch_and_cancellation_on_the_selected_dispatcher() {
        let actions = [
            Action::TaskLaunch {
                handle: Some("refreshTask".to_owned()),
                executor: TaskExecutor::Main,
                actions: vec![Action::Assign {
                    name: "finished".to_owned(),
                    value: Expr::Bool(true),
                }],
            },
            Action::TaskLaunch {
                handle: Some("workerTask".to_owned()),
                executor: TaskExecutor::Background,
                actions: vec![Action::Expression(Expr::Bool(true))],
            },
            Action::TaskLaunch {
                handle: None,
                executor: TaskExecutor::Main,
                actions: vec![Action::Expression(Expr::Bool(false))],
            },
            Action::TaskCancel {
                handle: "refreshTask".to_owned(),
            },
        ];
        let mut output = SourceWriter::new();

        render_actions(&actions, 0, &mut output);

        assert!(output.contains(
            "nexa_refreshTask?.cancel()\nrun {\n    val nexaTaskJob = nexaTaskScope.launch(context = Dispatchers.Main.immediate + nexaTaskExceptionHandler, start = CoroutineStart.LAZY) {"
        ));
        assert!(output.contains("    nexa_finished = true"));
        assert!(output.contains(
            "nexa_workerTask?.cancel()\nrun {\n    val nexaTaskJob = nexaTaskScope.launch(context = Dispatchers.Default + nexaTaskExceptionHandler, start = CoroutineStart.LAZY) {"
        ));
        assert!(output.contains(
            "val nexaTaskJob = nexaTaskScope.launch(context = Dispatchers.Main.immediate"
        ));
        assert!(output.ends_with("nexa_refreshTask?.cancel()\n"));
    }

    #[test]
    fn renders_explicit_throwing_call_recovery() {
        let actions = [Action::TryCatch {
            body: vec![Action::Expression(Expr::TryAwait(Box::new(
                Expr::NativeCall {
                    receiver: None,
                    namespace: "Camera".to_owned(),
                    name: "capture".to_owned(),
                    arguments: Vec::new(),
                    codecs: Vec::new(),
                    return_type: Type::Void,
                    source_span: None,
                    is_async: true,
                    is_throwing: true,
                },
            )))],
            error_catches: Vec::new(),
            catch_body: Some(vec![Action::Assign {
                name: "failed".to_owned(),
                value: Expr::Bool(true),
            }]),
        }];
        let mut output = SourceWriter::new();

        render_actions(&actions, 0, &mut output);

        assert!(output.contains("try {\n    CameraPlugin.instance.capture()"));
        assert!(output.contains("} catch (_: Exception) {\n    nexa_failed = true\n}"));
    }

    #[test]
    fn renders_typed_error_variants_and_payload_bindings() {
        let actions = [Action::TryCatch {
            body: vec![Action::Expression(Expr::TryAwait(Box::new(
                Expr::NativeCall {
                    receiver: None,
                    namespace: "Video".to_owned(),
                    name: "prepare".to_owned(),
                    arguments: Vec::new(),
                    codecs: Vec::new(),
                    return_type: Type::Void,
                    source_span: None,
                    is_async: true,
                    is_throwing: true,
                },
            )))],
            error_catches: vec![
                nexa_ir::ErrorCatchArm {
                    namespace: "Video".to_owned(),
                    error_type: "PlayerError".to_owned(),
                    variant: "invalidUrl".to_owned(),
                    parameters: Vec::new(),
                    body: vec![Action::Assign {
                        name: "failed".to_owned(),
                        value: Expr::Bool(true),
                    }],
                },
                nexa_ir::ErrorCatchArm {
                    namespace: "Video".to_owned(),
                    error_type: "PlayerError".to_owned(),
                    variant: "decodingFailed".to_owned(),
                    parameters: vec![("message".to_owned(), "message".to_owned(), Type::String)],
                    body: vec![Action::Assign {
                        name: "loadError".to_owned(),
                        value: Expr::State("message".to_owned(), Type::String),
                    }],
                },
            ],
            catch_body: None,
        }];
        let mut output = SourceWriter::new();

        render_actions(&actions, 0, &mut output);

        assert!(output.contains("catch (error: Exception)"));
        assert!(output.contains("PlayerError.invalidUrl -> {"));
        assert!(output.contains("is PlayerError.decodingFailed -> {"));
        assert!(output.contains("val nexa_message = error.message"));
        assert!(output.contains("nexa_loadError = nexa_message"));
        assert!(output.contains("else -> {\n            throw error"));
    }
}
