use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{
    Action, AnimationEasing, AnimationSpec, CollectionMutation, ErrorCatchArm, Expr, HapticStyle,
    Module, Node, SystemIcon, TaskExecutor,
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
        "androidx.compose.material3.MenuDefaults",
    );
    imports.add(features.uses_context_menu, "androidx.compose.ui.unit.dp");
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
        features.uses_drag_without_pinch,
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
        features.uses_drag || features.uses_pinch,
        "androidx.compose.runtime.mutableStateOf",
    );
    imports.add(
        features.uses_drag || features.uses_pinch,
        "androidx.compose.runtime.remember",
    );
    imports.add(
        features.uses_drag_velocity_without_pinch,
        "androidx.compose.ui.input.pointer.util.VelocityTracker",
    );
    imports.add(
        features.uses_drag_velocity_without_pinch,
        "androidx.compose.ui.input.pointer.util.addPointerInputChange",
    );
    imports.add(
        features.uses_drag_velocity_with_pinch,
        "android.os.SystemClock",
    );
    imports.add(
        features.uses_drag,
        "androidx.compose.ui.platform.LocalDensity",
    );
    imports.add(features.uses_button, "androidx.compose.material3.Button");
    imports.add(
        features.uses_button,
        "androidx.compose.material3.OutlinedButton",
    );
    imports.add(
        features.uses_button,
        "androidx.compose.material3.TextButton",
    );
    imports.add(
        features.uses_button,
        "androidx.compose.foundation.layout.PaddingValues",
    );
    imports.add(
        features.uses_button,
        "androidx.compose.foundation.layout.defaultMinSize",
    );
    imports.add(
        features.uses_button,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_button,
        "androidx.compose.foundation.layout.Spacer",
    );
    imports.add(
        features.uses_button,
        "androidx.compose.foundation.layout.width",
    );
    imports.add(
        features.uses_button,
        "androidx.compose.foundation.layout.size",
    );
    imports.add(features.uses_button, "androidx.compose.ui.Modifier");
    imports.add(features.uses_button, "androidx.compose.ui.unit.dp");
    imports.add(features.uses_button, "androidx.compose.ui.unit.sp");
    imports.add(
        features.uses_button || features.uses_switch || features.uses_picker,
        "androidx.compose.ui.text.font.FontWeight",
    );
    imports.add(
        features.uses_button_loading,
        "androidx.compose.material3.CircularProgressIndicator",
    );
    imports.add(features.uses_switch, "androidx.compose.material3.Switch");
    imports.add(
        features.uses_switch,
        "androidx.compose.material3.SwitchDefaults",
    );
    imports.add(features.uses_switch, "androidx.compose.material3.Text");
    imports.add(
        features.uses_switch,
        "androidx.compose.material3.MaterialTheme",
    );
    imports.add(
        features.uses_switch,
        "androidx.compose.foundation.layout.Row",
    );
    imports.add(
        features.uses_switch,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_switch,
        "androidx.compose.foundation.selection.toggleable",
    );
    imports.add(features.uses_switch, "androidx.compose.ui.Alignment");
    imports.add(features.uses_switch, "androidx.compose.ui.Modifier");
    imports.add(features.uses_switch, "androidx.compose.ui.unit.sp");
    imports.add(features.uses_switch, "androidx.compose.ui.graphics.Color");
    imports.add(features.uses_slider, "androidx.compose.material3.Slider");
    imports.add(
        features.uses_picker,
        "androidx.compose.foundation.layout.Row",
    );
    imports.add(
        features.uses_picker,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_picker,
        "androidx.compose.foundation.layout.wrapContentWidth",
    );
    imports.add(
        features.uses_picker,
        "androidx.compose.foundation.layout.Box",
    );
    imports.add(
        features.uses_picker,
        "androidx.compose.foundation.layout.Spacer",
    );
    imports.add(
        features.uses_picker,
        "androidx.compose.foundation.layout.width",
    );
    imports.add(
        features.uses_picker,
        "androidx.compose.foundation.layout.padding",
    );
    imports.add(
        features.uses_picker,
        "androidx.compose.foundation.layout.size",
    );
    imports.add(features.uses_picker, "androidx.compose.ui.Alignment");
    imports.add(
        features.uses_picker,
        "androidx.compose.foundation.clickable",
    );
    imports.add(features.uses_picker, "androidx.compose.ui.Modifier");
    imports.add(features.uses_picker, "androidx.compose.ui.unit.dp");
    imports.add(features.uses_picker, "androidx.compose.ui.unit.sp");
    imports.add(features.uses_picker, "androidx.compose.ui.graphics.Color");
    imports.add(
        features.uses_picker,
        "androidx.compose.ui.graphics.compositeOver",
    );
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
        "androidx.compose.material3.MaterialTheme",
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
        features.uses_progress_bar,
        "androidx.compose.material3.LinearProgressIndicator",
    );
    imports.add(
        features.uses_progress_bar,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_progress_bar,
        "androidx.compose.foundation.layout.height",
    );
    imports.add(features.uses_progress_bar, "androidx.compose.ui.draw.clip");
    imports.add(
        features.uses_progress_bar,
        "androidx.compose.ui.graphics.Color",
    );
    imports.add(
        features.uses_progress_bar,
        "androidx.compose.foundation.shape.RoundedCornerShape",
    );
    imports.add(features.uses_progress_bar, "androidx.compose.ui.unit.dp");
    imports.add(features.uses_progress_bar, "androidx.compose.ui.Modifier");
    imports.add(
        features.uses_progress_ring,
        "androidx.compose.material3.CircularProgressIndicator",
    );
    imports.add(
        features.uses_segmented_control,
        "androidx.compose.foundation.layout.offset",
    );
    imports.add(
        features.uses_segmented_control,
        "androidx.compose.foundation.border",
    );
    imports.add(
        features.uses_segmented_control,
        "androidx.compose.ui.unit.dp",
    );
    imports.add(
        features.uses_segmented_control,
        "androidx.compose.ui.unit.sp",
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
    style: Option<nexa_ir::ButtonStyle>,
    size: Option<nexa_ir::ButtonSize>,
    shape: Option<nexa_ir::ButtonShape>,
    tint: Option<&nexa_ir::ColorExpression>,
    actions: &[Action],
    full_width: bool,
    leading_content: bool,
    depth: usize,
    out: &mut SourceWriter,
) {
    let style = match style {
        Some(style) => {
            let style = match style {
                nexa_ir::ButtonStyle::Bordered => "Bordered",
                nexa_ir::ButtonStyle::Borderless => "Borderless",
                nexa_ir::ButtonStyle::Plain => "Plain",
                nexa_ir::ButtonStyle::BorderedProminent => "BorderedProminent",
            };
            format!("\"{style}\"")
        }
        None => "null".to_owned(),
    };
    let size = match size {
        Some(nexa_ir::ButtonSize::Small) => "\"Small\"",
        Some(nexa_ir::ButtonSize::Regular) => "\"Regular\"",
        Some(nexa_ir::ButtonSize::Large) => "\"Large\"",
        None => "null",
    };
    let shape = shape.map_or_else(
        || "null".to_owned(),
        |shape| match shape {
            nexa_ir::ButtonShape::Capsule => {
                "androidx.compose.foundation.shape.RoundedCornerShape(percent = 50)".to_owned()
            }
            nexa_ir::ButtonShape::Circle => {
                "androidx.compose.foundation.shape.CircleShape".to_owned()
            }
            nexa_ir::ButtonShape::Rounded(radius) => format!(
                "androidx.compose.foundation.shape.RoundedCornerShape({}.dp)",
                crate::generator::utils::number(radius)
            ),
        },
    );
    let icon = icon.map_or_else(|| "null".to_owned(), SystemIcon::material_reference);
    let tint = tint.map_or_else(
        || "null".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let loading = loading.map_or_else(|| "false".to_owned(), expression);
    let disabled = disabled.map_or_else(|| "false".to_owned(), expression);
    let icon_only_circle = icon != "null"
        && shape != "null"
        && matches!(
            shape.as_str(),
            "androidx.compose.foundation.shape.CircleShape"
        )
        && is_empty_button_label(label);

    out.line_at(depth, format_args!("NexaButtonPrimitive("));
    out.line_at(depth + 1, format_args!("label = {},", expression(label)));
    out.line_at(depth + 1, format_args!("icon = {icon},"));
    out.line_at(depth + 1, format_args!("style = {style},"));
    out.line_at(depth + 1, format_args!("size = {size},"));
    out.line_at(depth + 1, format_args!("shape = {shape},"));
    out.line_at(depth + 1, format_args!("tint = {tint},"));
    out.line_at(depth + 1, format_args!("loading = {loading},"));
    out.line_at(depth + 1, format_args!("disabled = {disabled},"));
    out.line_at(
        depth + 1,
        format_args!("iconOnlyCircle = {icon_only_circle},"),
    );
    out.line_at(depth + 1, format_args!("fullWidth = {full_width},"));
    out.line_at(
        depth + 1,
        format_args!("leadingContent = {leading_content},"),
    );
    out.line_at(depth + 1, format_args!("onClick = {{"));
    render_actions(actions, depth + 2, out);
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth, format_args!(")"));
}

fn is_empty_button_label(label: &Expr) -> bool {
    match label {
        Expr::String(value) => value.is_empty(),
        Expr::LocalizedText { value, .. } => {
            matches!(value.as_ref(), Expr::String(value) if value.is_empty())
        }
        _ => false,
    }
}

pub(crate) fn render_switch(
    state: &str,
    label: &nexa_ir::Expr,
    depth: usize,
    out: &mut SourceWriter,
) {
    let label = crate::generator::engine::expressions::text_expression(label);
    out.line_at(depth, format_args!("NexaSwitchPrimitive("));
    out.line_at(depth + 1, format_args!("label = {label},"));
    out.line_at(depth + 1, format_args!("checked = {},", state_name(state)));
    out.line_at(
        depth + 1,
        format_args!("onCheckedChange = {{ {} = it }},", state_name(state)),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "fontSize = {}.sp,",
            nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE
        ),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "lineHeight = {}.sp * {}f,",
            nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE,
            nexa_codegen::design_system::DEFAULT_LINE_HEIGHT_MULTIPLIER,
        ),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "offTrackColor = if (MaterialTheme.colorScheme.background == Color.Black) Color(0x{:08X}) else Color(0xFFE5E5EA),",
            nexa_codegen::design_system::FORM_SWITCH_OFF_TRACK_ARGB,
        ),
    );
    out.line_at(depth, format_args!(")"));
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
    let intervals = if step > 0.0 {
        ((max - min) / step).round() as i32
    } else {
        0
    };
    let on_value_change = if step > 0.0 {
        // Material3 Slider stores its value as Float. Convert stepped values back to
        // the declared Nexa Float64 range so binary Float error does not leak into
        // observable state (for example, 0.9 becoming 0.8999999761581421).
        format!("({min:?} + kotlin.math.round((it.toDouble() - {min:?}) / {step:?}) * {step:?})")
    } else {
        "it.toDouble()".to_owned()
    };
    let value = if animated {
        format!("{}Animated", state_name(state))
    } else {
        state_name(state)
    };
    out.line_at(
        depth,
        format_args!(
            "NexaSliderPrimitive(value = {value}.toFloat(), onValueChange = {{ {} = {on_value_change} }}, valueRange = {}f..{}f, steps = {})",
            state_name(state),
            min as f32,
            max as f32,
            intervals.saturating_sub(1).max(0),
        ),
    );
}

pub(crate) fn render_progress_bar(
    progress: Option<&Expr>,
    style: Option<&nexa_ir::ProgressBarStyle>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let tint = style.and_then(|style| style.tint.as_ref());
    let track_color = style.and_then(|style| style.track_color.as_ref());
    let thickness = style.and_then(|style| style.thickness);
    let corner_radius = style.and_then(|style| style.corner_radius);
    let android_wave = style.and_then(|style| style.android_wave.as_ref());
    let tint = tint.map_or_else(
        || "null".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let track_color = track_color.map_or_else(
        || "null".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let thickness = thickness.map_or_else(|| "null".to_owned(), |value| format!("{value}f"));
    let corner_radius =
        corner_radius.map_or_else(|| "null".to_owned(), |value| format!("{value}f"));
    let mut arguments = format!(
        "progress = {}, tint = {}, trackColor = {}, thickness = {}, cornerRadius = {}",
        progress.map_or_else(
            || "null".to_owned(),
            |progress| format!("{}.toFloat()", expression(progress))
        ),
        tint,
        track_color,
        thickness,
        corner_radius
    );
    if let Some(android_wave) = android_wave {
        arguments.push_str(&format!(", androidWave = {}", expression(android_wave)));
    }
    out.line_at(depth, format_args!("NexaProgressBarPrimitive({arguments})"));
}

pub(crate) fn render_progress_ring(
    progress: Option<&Expr>,
    style: Option<&nexa_ir::ProgressRingStyle>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let tint = style.and_then(|style| style.tint.as_ref()).map_or_else(
        || "null".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let track_color = style
        .and_then(|style| style.track_color.as_ref())
        .map_or_else(
            || "null".to_owned(),
            crate::generator::colors::expression_for_color,
        );
    let size = style
        .and_then(|style| style.size)
        .map_or_else(|| "null".to_owned(), |value| format!("{value}f"));
    let thickness = style
        .and_then(|style| style.thickness)
        .map_or_else(|| "null".to_owned(), |value| format!("{value}f"));
    let mut arguments = format!(
        "progress = {}, tint = {}, trackColor = {}, size = {}, thickness = {}",
        progress.map_or_else(
            || "null".to_owned(),
            |progress| format!("{}.toFloat()", expression(progress))
        ),
        tint,
        track_color,
        size,
        thickness
    );
    if let Some(android_wave) = style.and_then(|style| style.android_wave.as_ref()) {
        arguments.push_str(&format!(", androidWave = {}", expression(android_wave)));
    }
    out.line_at(
        depth,
        format_args!("NexaProgressRingPrimitive({arguments})"),
    );
}

#[derive(Clone, Copy)]
pub(crate) struct SegmentedControlStyle<'a> {
    pub tint: Option<&'a nexa_ir::ColorExpression>,
    pub background: Option<&'a nexa_ir::ColorExpression>,
    pub text_color: Option<&'a nexa_ir::ColorExpression>,
    pub selected_text_color: Option<&'a nexa_ir::ColorExpression>,
    pub font_size: Option<f32>,
    pub font_weight: Option<nexa_ir::FontWeight>,
    pub height: Option<f32>,
    pub corner_radius: Option<f32>,
    pub border_color: Option<&'a nexa_ir::ColorExpression>,
    pub border_width: Option<f32>,
}

pub(crate) fn render_segmented_control(
    items: &Expr,
    state: &str,
    animation: Option<AnimationSpec>,
    style: SegmentedControlStyle<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let options = expression(items);
    let selected = state_name(state);
    let animation = animation.map_or_else(|| "null".to_owned(), kotlin_animation);
    let color = |value: Option<&nexa_ir::ColorExpression>| {
        value.map_or_else(
            || "null".to_owned(),
            crate::generator::colors::expression_for_color,
        )
    };
    let tint = color(style.tint);
    let background = color(style.background);
    let text_color = color(style.text_color);
    let selected_text_color = color(style.selected_text_color);
    let font_size = style
        .font_size
        .map_or_else(|| "null".to_owned(), |value| format!("{value}f.sp"));
    let font_weight = style.font_weight.map_or_else(
        || "null".to_owned(),
        |weight| {
            match weight {
                nexa_ir::FontWeight::Normal => "androidx.compose.ui.text.font.FontWeight.Normal",
                nexa_ir::FontWeight::Medium => "androidx.compose.ui.text.font.FontWeight.Medium",
                nexa_ir::FontWeight::Semibold => {
                    "androidx.compose.ui.text.font.FontWeight.SemiBold"
                }
                nexa_ir::FontWeight::Bold => "androidx.compose.ui.text.font.FontWeight.Bold",
            }
            .to_owned()
        },
    );
    let height = style
        .height
        .map_or_else(|| "null".to_owned(), |value| format!("{value}f.dp"));
    let corner_radius = style
        .corner_radius
        .map_or_else(|| "null".to_owned(), |value| format!("{value}f.dp"));
    let border_color = color(style.border_color);
    let border_width = style
        .border_width
        .map_or_else(|| "null".to_owned(), |value| format!("{value}f.dp"));
    out.line_at(
        depth,
        format_args!(
            "NexaSegmentedControlPrimitive(items = {options}, selected = {selected}, animation = {animation}, tint = {tint}, background = {background}, textColor = {text_color}, selectedTextColor = {selected_text_color}, fontSize = {font_size}, fontWeight = {font_weight}, height = {height}, cornerRadius = {corner_radius}, borderColor = {border_color}, borderWidth = {border_width}, onSelected = {{ {selected} = it }})"
        ),
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_picker(
    items: &Expr,
    item_icons: &[SystemIcon],
    state: &str,
    icon: Option<&SystemIcon>,
    label: Option<&Expr>,
    tint: Option<&nexa_ir::ColorExpression>,
    fill_max_width: bool,
    depth: usize,
    out: &mut SourceWriter,
) {
    let selected = state_name(state);
    let icon = icon.map_or_else(|| "null".to_owned(), SystemIcon::material_reference);
    let item_icons = if item_icons.is_empty() {
        "emptyList()".to_owned()
    } else {
        format!(
            "listOf({})",
            item_icons
                .iter()
                .map(SystemIcon::material_reference)
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let label = label.map_or_else(|| "null".to_owned(), expression);
    let tint = tint.map_or_else(
        || "null".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let fill_max_width = fill_max_width.to_string();
    out.line_at(
        depth,
        format_args!(
            "NexaPickerPrimitive(items = {}, itemIcons = {item_icons}, selected = {selected}, icon = {icon}, label = {label}, tint = {tint}, fillMaxWidth = {fill_max_width}) {{ {selected} = it }}",
            expression(items),
        ),
    );
}

pub(crate) fn render_picker_date(node: &Node, depth: usize, out: &mut SourceWriter) {
    let Node::PickerDate { props } = node else {
        return;
    };
    let nexa_ir::PickerDateProps {
        picker_type,
        style,
        selection,
        timestamp_state,
        start_timestamp_state,
        end_timestamp_state,
        minimum,
        maximum,
        tint,
        text_color,
        background_color,
        picker_background_color,
        border_color,
        border_width,
        corner_radius,
        locale,
        time_zone,
        is_24_hour,
        enabled,
        accessibility_label,
    } = props.as_ref();
    let kind = match picker_type {
        nexa_ir::PickerDateType::Date => "Date",
        nexa_ir::PickerDateType::Time => "Time",
        nexa_ir::PickerDateType::DateTime => "DateTime",
    };
    let style = match style {
        nexa_ir::PickerDateStyle::Automatic => "Automatic",
        nexa_ir::PickerDateStyle::Graphical => "Graphical",
        nexa_ir::PickerDateStyle::Compact => "Compact",
        nexa_ir::PickerDateStyle::Input => "Input",
        nexa_ir::PickerDateStyle::Wheel => "Wheel",
    };
    let selection = match selection {
        nexa_ir::PickerDateSelection::Single => "Single",
        nexa_ir::PickerDateSelection::Range => "Range",
    };
    let binding = |state: &Option<String>| {
        state
            .as_deref()
            .map_or_else(|| "null".to_owned(), state_name)
    };
    let optional_expr = |value: Option<&Expr>| value.map_or_else(|| "null".to_owned(), expression);
    let color = |value: Option<&nexa_ir::ColorExpression>| {
        value.map_or_else(
            || "null".to_owned(),
            crate::generator::colors::expression_for_color,
        )
    };
    let optional_float =
        |value: Option<f32>| value.map_or_else(|| "null".to_owned(), |v| format!("{v}f.dp"));
    let timestamp_callback = timestamp_state.as_deref().map_or_else(
        || "null".to_owned(),
        |state| format!("{{ value -> {} = value }}", state_name(state)),
    );
    let range_callback = match (
        start_timestamp_state.as_deref(),
        end_timestamp_state.as_deref(),
    ) {
        (Some(start), Some(end)) => format!(
            "{{ start, finish -> {} = start; {} = finish }}",
            state_name(start),
            state_name(end)
        ),
        _ => "null".to_owned(),
    };
    out.line_at(depth, format_args!("NexaDatePickerPrimitive("));
    out.line_at(
        depth + 1,
        format_args!("timestamp = {},", binding(timestamp_state)),
    );
    out.line_at(
        depth + 1,
        format_args!("startTimestamp = {},", binding(start_timestamp_state)),
    );
    out.line_at(
        depth + 1,
        format_args!("endTimestamp = {},", binding(end_timestamp_state)),
    );
    out.line_at(depth + 1, format_args!("pickerType = {kind:?},"));
    out.line_at(depth + 1, format_args!("style = {style:?},"));
    out.line_at(depth + 1, format_args!("selection = {selection:?},"));
    out.line_at(
        depth + 1,
        format_args!("minimum = {},", optional_expr(minimum.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!("maximum = {},", optional_expr(maximum.as_ref())),
    );
    out.line_at(depth + 1, format_args!("tint = {},", color(tint.as_ref())));
    out.line_at(
        depth + 1,
        format_args!("textColor = {},", color(text_color.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!("backgroundColor = {},", color(background_color.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "pickerBackgroundColor = {},",
            color(picker_background_color.as_ref())
        ),
    );
    out.line_at(
        depth + 1,
        format_args!("borderColor = {},", color(border_color.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!("borderWidth = {},", optional_float(*border_width)),
    );
    out.line_at(
        depth + 1,
        format_args!("cornerRadius = {},", optional_float(*corner_radius)),
    );
    out.line_at(
        depth + 1,
        format_args!("localeTag = {},", optional_expr(locale.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!("timeZoneId = {},", optional_expr(time_zone.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!("is24Hour = {},", optional_expr(is_24_hour.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!("enabled = {},", optional_expr(enabled.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "accessibilityLabel = {},",
            optional_expr(accessibility_label.as_ref())
        ),
    );
    out.line_at(
        depth + 1,
        format_args!("onTimestampChanged = {timestamp_callback},"),
    );
    out.line_at(
        depth + 1,
        format_args!("onRangeChanged = {range_callback},"),
    );
    out.line_at(depth, format_args!(")"));
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
        let configuration_id = out.next_id();
        out.line_at(
            depth,
            format_args!(
                "val nexaBaseViewConfiguration{configuration_id} = LocalViewConfiguration.current"
            ),
        );
        out.line_at(
            depth,
            format_args!(
                "val nexaLongPressViewConfiguration{configuration_id} = remember(nexaBaseViewConfiguration{configuration_id}, {duration}) {{"
            ),
        );
        out.line_at(
            depth + 1,
            format_args!(
                "object : ViewConfiguration by nexaBaseViewConfiguration{configuration_id} {{"
            ),
        );
        out.line_at(
            depth + 2,
            format_args!(
                "override val longPressTimeoutMillis: Long = ({duration}).toLong().coerceAtLeast(1L)"
            ),
        );
        out.line_at(depth + 1, format_args!("}}"));
        out.line_at(depth, format_args!("}}"));
        out.line_at(depth, format_args!("CompositionLocalProvider("));
        out.line_at(
            depth + 1,
            format_args!(
                "LocalViewConfiguration provides nexaLongPressViewConfiguration{configuration_id}"
            ),
        );
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
    let gesture_consumed_name =
        (has_drag || has_pinch).then(|| format!("nexaPressableGestureConsumed{}", out.next_id()));
    if let Some(name) = &gesture_consumed_name {
        out.line_at(
            depth,
            format_args!("val {name} = remember {{ mutableStateOf(false) }}"),
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
            if let Some(name) = &gesture_consumed_name {
                out.line_at(depth + 5, format_args!("{name}.value = true"));
            }
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
        if let Some(name) = &gesture_consumed_name {
            out.line_at(depth + 5, format_args!("{name}.value = true"));
        }
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
        if let Some(name) = &gesture_consumed_name {
            out.line_at(depth + 5, format_args!("{name}.value = true"));
        }
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
    out.line_at(depth + 2, format_args!("role = Role.Button,"));
    out.text_at(depth + 2, format_args!("onClick = {{"));
    if actions.is_empty() && haptic.is_none() && gesture_consumed_name.is_none() {
        out.push_str(if has_combined_clickable {
            " },\n"
        } else {
            " }),\n"
        });
    } else {
        out.push('\n');
        if let Some(name) = &gesture_consumed_name {
            out.line_at(depth + 3, format_args!("if ({name}.value) {{"));
            out.line_at(depth + 4, format_args!("{name}.value = false"));
            out.line_at(depth + 3, format_args!("}} else {{"));
            if let Some(haptic) = haptic {
                render_haptic(*haptic, depth + 4, out);
            }
            render_actions(actions, depth + 4, out);
            out.line_at(depth + 3, format_args!("}}"));
        } else {
            if let Some(haptic) = haptic {
                render_haptic(*haptic, depth + 3, out);
            }
            render_actions(actions, depth + 3, out);
        }
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
    if *fill_max_size {
        out.line_at(
            depth + 1,
            format_args!("contentAlignment = Alignment.Center,"),
        );
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
        out.line_at(
            depth,
            format_args!(
                "androidx.compose.material3.MaterialExpressiveTheme(colorScheme = androidx.compose.material3.MaterialTheme.colorScheme) {{"
            ),
        );
        out.line_at(depth + 1, format_args!("DropdownMenu("));
        out.line_at(
            depth + 2,
            format_args!("expanded = nexaContextMenuExpanded.value,"),
        );
        out.line_at(
            depth + 2,
            format_args!("onDismissRequest = {{ nexaContextMenuExpanded.value = false }},"),
        );
        out.line_at(
            depth + 2,
            format_args!("shape = MenuDefaults.standaloneGroupShape,"),
        );
        out.line_at(
            depth + 2,
            format_args!("containerColor = MenuDefaults.groupStandardContainerColor,"),
        );
        out.line_at(depth + 2, format_args!("tonalElevation = 0.dp,"));
        out.line_at(depth + 2, format_args!("shadowElevation = 8.dp,"));
        out.line_at(depth + 1, format_args!(") {{"));
        let item_count = context_menu
            .iter()
            .filter(|node| matches!(node, Node::Button { .. }))
            .count();
        if item_count > 0 {
            render_context_menu(context_menu, item_count, depth + 2, out);
        }
        out.push('\n');
        indent(out, depth + 1);
        out.push('}');
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

fn render_context_menu(nodes: &[Node], item_count: usize, depth: usize, out: &mut SourceWriter) {
    let mut item_index = 0;
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
            format_args!(
                "shape = MenuDefaults.itemShape(index = {item_index}, count = {item_count}).shape,"
            ),
        );
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
        item_index += 1;
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
                let mut previous_values = Vec::with_capacity(animated_states.len());
                for state in animated_states {
                    let state = state_name(state);
                    let previous = format!("nexaAnimatedStateBefore{}", out.next_id());
                    previous_values.push((state.clone(), previous.clone()));
                    out.line_at(depth, format_args!("val {previous} = {state}"));
                    out.line_at(depth, format_args!("{state}AnimationSpec = {spec}"));
                }
                render_actions(actions, depth, out);
                for (state, previous) in previous_values {
                    // `animateFloatAsState` only invokes its finished listener
                    // when the target changes. Clear a one-shot spec here when
                    // this transaction leaves a value untouched, including
                    // branches that do not execute their assignment.
                    out.line_at(
                        depth,
                        format_args!("if ({state}.toBits() == {previous}.toBits()) {{"),
                    );
                    out.line_at(
                        depth + 1,
                        format_args!(
                            "{state}AnimationSpec = androidx.compose.animation.core.snap()"
                        ),
                    );
                    out.line_at(depth, format_args!("}}"));
                }
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

pub(crate) fn kotlin_animation(animation: AnimationSpec) -> String {
    match animation {
        AnimationSpec::Spring { response, damping } => {
            let physics = nexa_codegen::animation::spring_physics(response, damping);
            format!(
                "androidx.compose.animation.core.spring(dampingRatio = {damping}f, stiffness = {}f)",
                physics.stiffness
            )
        }
        AnimationSpec::Tween {
            duration,
            delay,
            easing,
        } => {
            let duration_millis = (duration * 1000.0).round() as u32;
            let delay_millis = (delay * 1000.0).round() as u32;
            let delay_parameter = if delay_millis > 0 {
                format!(", delayMillis = {delay_millis}")
            } else {
                String::new()
            };
            format!(
                "androidx.compose.animation.core.tween(durationMillis = {duration_millis}{delay_parameter}, easing = {})",
                kotlin_easing(easing)
            )
        }
        AnimationSpec::EaseIn => kotlin_tween("FastOutLinearInEasing"),
        AnimationSpec::EaseOut => kotlin_tween("LinearOutSlowInEasing"),
        AnimationSpec::EaseInOut => kotlin_tween("FastOutSlowInEasing"),
        AnimationSpec::Linear => kotlin_tween("LinearEasing"),
    }
}

fn kotlin_tween(easing: &str) -> String {
    format!(
        "androidx.compose.animation.core.tween(durationMillis = 300, easing = androidx.compose.animation.core.{easing})"
    )
}

fn kotlin_easing(easing: AnimationEasing) -> String {
    match easing {
        AnimationEasing::Linear => "androidx.compose.animation.core.LinearEasing".to_owned(),
        AnimationEasing::EaseIn => {
            "androidx.compose.animation.core.FastOutLinearInEasing".to_owned()
        }
        AnimationEasing::EaseOut => {
            "androidx.compose.animation.core.LinearOutSlowInEasing".to_owned()
        }
        AnimationEasing::EaseInOut => {
            "androidx.compose.animation.core.FastOutSlowInEasing".to_owned()
        }
        AnimationEasing::CubicBezier { x1, y1, x2, y2 } => {
            format!("androidx.compose.animation.core.CubicBezierEasing({x1}f, {y1}f, {x2}f, {y2}f)")
        }
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
    use nexa_ir::{Action, CollectionMutation, Expr, NumericType, SystemIcon, TaskExecutor, Type};

    use super::{
        render_actions, render_button, render_picker, render_progress_bar, render_progress_ring,
        render_slider, render_switch,
    };

    #[test]
    fn picker_calls_the_shared_native_component_primitive() {
        let mut output = SourceWriter::new();

        render_picker(
            &Expr::Array(vec![Expr::String("Low".to_owned())]),
            &[],
            "priority",
            Some(&SystemIcon::Shared("flag_filled".to_owned())),
            None,
            None,
            true,
            0,
            &mut output,
        );

        assert!(
            output.contains("NexaPickerPrimitive(items = listOf(\"Low\"), itemIcons = emptyList()")
        );
        assert!(output.contains("icon = Icons.Filled.Flag"));
        assert!(
            output
                .contains("label = null, tint = null, fillMaxWidth = true) { nexa_priority = it }")
        );
    }

    #[test]
    fn picker_generates_material_icons_for_each_menu_row() {
        let items = Expr::Array(vec![
            Expr::String("Due date".to_owned()),
            Expr::String("Priority".to_owned()),
            Expr::String("Title".to_owned()),
        ]);
        let item_icons = vec![
            SystemIcon::Shared("calendar".to_owned()),
            SystemIcon::Shared("flag_filled".to_owned()),
            SystemIcon::Shared("edit".to_owned()),
        ];
        let mut output = SourceWriter::new();

        render_picker(
            &items,
            &item_icons,
            "priority",
            None,
            Some(&Expr::String("Priority".to_owned())),
            None,
            true,
            0,
            &mut output,
        );

        assert!(output.contains(
            "itemIcons = listOf(Icons.Outlined.CalendarMonth, Icons.Filled.Flag, Icons.Filled.Edit)"
        ));
    }

    #[test]
    fn picker_renders_dynamic_tint_and_an_anchored_dropdown() {
        let items = Expr::Array(vec![Expr::String("None".to_owned())]);
        let tint = nexa_ir::ColorExpression::Dynamic(Box::new(Expr::State(
            "priorityTint".to_owned(),
            nexa_ir::Type::String,
        )));
        let mut output = SourceWriter::new();

        render_picker(
            &items,
            &[],
            "priority",
            Some(&SystemIcon::Shared("flag_filled".to_owned())),
            None,
            Some(&tint),
            true,
            0,
            &mut output,
        );

        assert!(output.contains("nexaColorFromHex(nexa_priorityTint)"));
        assert!(output.contains("NexaPickerPrimitive(items = listOf(\"None\"), itemIcons = emptyList(), selected = nexa_priority, icon = Icons.Filled.Flag, label = null, tint = nexaColorFromHex(nexa_priorityTint), fillMaxWidth = true)"));
    }

    #[test]
    fn renders_button_style_size_and_shape_as_compose_parameters() {
        let label = Expr::String("Skip".to_owned());
        let mut output = SourceWriter::new();

        render_button(
            &label,
            None,
            None,
            None,
            Some(nexa_ir::ButtonStyle::Borderless),
            Some(nexa_ir::ButtonSize::Large),
            Some(nexa_ir::ButtonShape::Capsule),
            None,
            &[],
            false,
            false,
            0,
            &mut output,
        );

        assert!(output.contains("NexaButtonPrimitive("));
        assert!(output.contains("label = \"Skip\""));
        assert!(output.contains("style = \"Borderless\""));
        assert!(output.contains("size = \"Large\""));
        assert!(output.contains(
            "shape = androidx.compose.foundation.shape.RoundedCornerShape(percent = 50)"
        ));
    }

    #[test]
    fn bordered_button_uses_shared_dynamic_accent_and_disabled_alpha() {
        let label = Expr::String("System".to_owned());
        let disabled = Expr::Bool(true);
        let mut output = SourceWriter::new();

        render_button(
            &label,
            None,
            None,
            Some(&disabled),
            Some(nexa_ir::ButtonStyle::Bordered),
            None,
            None,
            None,
            &[],
            false,
            false,
            0,
            &mut output,
        );

        assert!(output.contains("style = \"Bordered\""));
        assert!(output.contains("disabled = true"));
        assert!(output.contains("tint = null"));
    }

    #[test]
    fn default_button_padding_keeps_compose_labels_on_one_line_in_rows() {
        let label = Expr::String("Jump to Top".to_owned());
        let mut output = SourceWriter::new();

        render_button(
            &label,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            &[],
            false,
            false,
            0,
            &mut output,
        );

        assert!(output.contains("style = null"));
        assert!(output.contains("size = null"));
        assert!(output.contains("shape = null"));
    }

    #[test]
    fn icon_only_circle_button_has_a_square_unpadded_touch_target() {
        let label = Expr::LocalizedText {
            key: String::new(),
            value: Box::new(Expr::String(String::new())),
            comment: None,
        };
        let mut output = SourceWriter::new();

        render_button(
            &label,
            Some(&SystemIcon::Shared("add".to_owned())),
            None,
            None,
            None,
            None,
            Some(nexa_ir::ButtonShape::Circle),
            None,
            &[],
            false,
            false,
            0,
            &mut output,
        );

        assert!(output.contains("icon = Icons.Filled.Add"));
        assert!(output.contains("iconOnlyCircle = true"));
        assert!(output.contains("shape = androidx.compose.foundation.shape.CircleShape"));
    }

    #[test]
    fn switch_keeps_its_label_in_a_full_width_control_row() {
        let label = Expr::String("Open Links In App".to_owned());
        let mut output = SourceWriter::new();

        render_switch("openLinksInApp", &label, 0, &mut output);

        assert!(output.contains("NexaSwitchPrimitive("));
        assert!(output.contains("label = \"Open Links In App\""));
        assert!(output.contains("checked = nexa_openLinksInApp"));
        assert!(
            output.contains(
                "offTrackColor = if (MaterialTheme.colorScheme.background == Color.Black)"
            )
        );
    }

    #[test]
    fn renders_native_progress_indicators_with_bounded_float_values() {
        let progress = Expr::State(
            "progress".to_owned(),
            nexa_ir::Type::Numeric(nexa_ir::NumericType::Float64),
        );
        let mut output = SourceWriter::new();
        render_progress_bar(Some(&progress), None, 0, &mut output);
        render_progress_ring(Some(&progress), None, 0, &mut output);
        assert_eq!(
            output.as_str(),
            "NexaProgressBarPrimitive(progress = nexa_progress.toFloat(), tint = null, trackColor = null, thickness = null, cornerRadius = null)\nNexaProgressRingPrimitive(progress = nexa_progress.toFloat(), tint = null, trackColor = null, size = null, thickness = null)\n"
        );
    }

    #[test]
    fn omitted_progress_generates_the_native_indeterminate_bar_call() {
        let mut output = SourceWriter::new();
        render_progress_bar(None, None, 0, &mut output);

        assert_eq!(
            output.as_str(),
            "NexaProgressBarPrimitive(progress = null, tint = null, trackColor = null, thickness = null, cornerRadius = null)\n"
        );
    }

    #[test]
    fn omitted_ring_progress_generates_the_native_indeterminate_ring_call() {
        let mut output = SourceWriter::new();
        render_progress_ring(None, None, 0, &mut output);

        assert_eq!(
            output.as_str(),
            "NexaProgressRingPrimitive(progress = null, tint = null, trackColor = null, size = null, thickness = null)\n"
        );
    }

    #[test]
    fn renders_slider_as_a_compose_float_binding_with_static_steps() {
        let mut output = SourceWriter::new();
        render_slider("volume", false, 0.0, 1.0, 0.1, 0, &mut output);
        assert_eq!(
            output.as_str(),
            "NexaSliderPrimitive(value = nexa_volume.toFloat(), onValueChange = { nexa_volume = (0.0 + kotlin.math.round((it.toDouble() - 0.0) / 0.1) * 0.1) }, valueRange = 0f..1f, steps = 9)\n"
        );
    }

    #[test]
    fn rounds_stepped_slider_values_back_to_the_declared_double_range() {
        let mut output = SourceWriter::new();
        render_slider("zoom", false, -1.0, 1.0, 0.25, 0, &mut output);
        assert_eq!(
            output.as_str(),
            "NexaSliderPrimitive(value = nexa_zoom.toFloat(), onValueChange = { nexa_zoom = (-1.0 + kotlin.math.round((it.toDouble() - -1.0) / 0.25) * 0.25) }, valueRange = -1f..1f, steps = 7)\n"
        );
    }

    #[test]
    fn leaves_continuous_slider_values_unquantized() {
        let mut output = SourceWriter::new();
        render_slider("progress", false, 0.0, 1.0, 0.0, 0, &mut output);
        assert_eq!(
            output.as_str(),
            "NexaSliderPrimitive(value = nexa_progress.toFloat(), onValueChange = { nexa_progress = it.toDouble() }, valueRange = 0f..1f, steps = 0)\n"
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
