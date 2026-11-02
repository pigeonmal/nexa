use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{
    Action, AnimationEasing, AnimationSpec, CollectionMutation, ErrorCatchArm, Expr, HapticStyle,
    Module, Node, ScreenId, SystemIcon, TaskExecutor, Type,
};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static KNOWN_SCREENS: RefCell<HashMap<String, ScreenId>> = RefCell::new(HashMap::new());
}

pub(crate) fn register_known_screens(screens: &[nexa_ir::Screen]) {
    KNOWN_SCREENS.with(|cell| {
        let mut map = cell.borrow_mut();
        map.clear();
        for screen in screens {
            map.insert(screen.name.clone(), screen.id);
            map.insert(format!("screen_{}", screen.id.0), screen.id);
            map.insert(format!("screen{}", screen.id.0), screen.id);
        }
    });
}

pub(crate) fn resolve_screen_id(key: &str) -> Option<ScreenId> {
    KNOWN_SCREENS.with(|cell| {
        let map = cell.borrow();
        if let Some(id) = map.get(key) {
            return Some(*id);
        }
        key.strip_prefix("screen_")
            .or_else(|| key.strip_prefix("screen"))
            .and_then(|idx| idx.parse::<usize>().ok())
            .map(ScreenId)
    })
}

use crate::generator::{
    components::render_children,
    expressions::expression,
    utils::{indent, swift_string},
};

use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;
use crate::generator::engine::types::swift_type;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.uses_haptic, "UIKit");
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_button(
    label: &nexa_ir::Expr,
    comment: Option<&str>,
    accessibility_label: Option<&nexa_ir::Expr>,
    icon: Option<&SystemIcon>,
    loading: Option<&nexa_ir::Expr>,
    disabled: Option<&nexa_ir::Expr>,
    style: Option<nexa_ir::ButtonStyle>,
    size: Option<nexa_ir::ButtonSize>,
    shape: Option<nexa_ir::ButtonShape>,
    tint: Option<&nexa_ir::ColorExpression>,
    glass: bool,
    actions: &[Action],
    depth: usize,
    out: &mut SourceWriter,
) {
    let style = match style {
        Some(nexa_ir::ButtonStyle::Bordered) => ".bordered",
        Some(nexa_ir::ButtonStyle::Borderless) => ".borderless",
        Some(nexa_ir::ButtonStyle::Plain) => ".plain",
        Some(nexa_ir::ButtonStyle::BorderedProminent) | None => ".borderedProminent",
    };
    let size = match size {
        Some(nexa_ir::ButtonSize::Small) => ".small",
        Some(nexa_ir::ButtonSize::Regular) => ".regular",
        Some(nexa_ir::ButtonSize::Large) => ".large",
        None => "nil",
    };
    let resolved_shape = shape.unwrap_or(nexa_codegen::design_system::DEFAULT_BUTTON_SHAPE);
    let icon_only_circle = icon.is_some()
        && resolved_shape == nexa_ir::ButtonShape::Circle
        && is_empty_button_label(label);
    let shape = match resolved_shape {
        nexa_ir::ButtonShape::Circle => ".circle".to_owned(),
        nexa_ir::ButtonShape::Capsule => ".capsule".to_owned(),
        nexa_ir::ButtonShape::Rounded(radius) => format!(".rounded({radius})"),
    };
    let tint = tint.map_or_else(
        || "nil".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let loading = loading.map_or_else(|| "false".to_owned(), expression);
    let disabled = disabled.map_or_else(|| "false".to_owned(), expression);
    let accessibility_label = accessibility_label.map_or_else(|| expression(label), expression);

    out.line_at(depth, format_args!("NexaButtonPrimitive("));
    out.line_at(depth + 1, format_args!("style: {style},"));
    out.line_at(depth + 1, format_args!("size: {size},"));
    out.line_at(depth + 1, format_args!("shape: {shape},"));
    out.line_at(depth + 1, format_args!("tint: {tint},"));
    out.line_at(depth + 1, format_args!("loading: {loading},"));
    out.line_at(depth + 1, format_args!("disabled: {disabled},"));
    out.line_at(
        depth + 1,
        format_args!("accessibilityLabel: {accessibility_label},"),
    );
    out.line_at(depth + 1, format_args!("glass: {glass},"));
    out.line_at(
        depth + 1,
        format_args!("iconOnlyCircle: {icon_only_circle},"),
    );
    out.line_at(depth + 1, format_args!("action: {{"));
    render_actions(actions, depth + 2, out);
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth + 1, format_args!("label: {{"));
    render_button_label(label, comment, icon, depth + 2, out);
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth, format_args!(")"));
}

pub(crate) fn render_confirmation_action_button(
    button: &Node,
    depth: usize,
    out: &mut SourceWriter,
) {
    let Node::Button {
        label,
        icon,
        accessibility_label,
        loading,
        disabled,
        actions,
        ..
    } = button
    else {
        return;
    };

    indent(out, depth);
    out.push_str("Button(action: {");
    if actions.is_empty() {
        out.push_str(" }) {");
    } else {
        out.push('\n');
        render_actions(actions, depth + 1, out);
        indent(out, depth);
        out.push_str("}) {");
    }
    out.push('\n');
    if let Some(loading) = loading {
        out.line_at(depth + 1, format_args!("if {} {{", expression(loading)));
        indent(out, depth + 2);
        out.push_str("ProgressView()\n");
        indent(out, depth + 1);
        out.push_str("} else {\n");
        render_button_label(label, None, icon.as_ref(), depth + 2, out);
        indent(out, depth + 1);
        out.push('}');
        out.push('\n');
        indent(out, depth);
        out.push('}');
        out.push_str(".disabled(");
        out.push_str(&expression(loading));
        if let Some(disabled) = disabled {
            out.push_str(" || ");
            out.push_str(&expression(disabled));
        }
        out.push(')');
        if let Some(accessibility_label) = accessibility_label {
            out.push_str(&format!(
                ".accessibilityLabel(Text({}))",
                expression(accessibility_label)
            ));
        }
        return;
    }
    render_button_label(label, None, icon.as_ref(), depth + 1, out);
    indent(out, depth);
    out.push('}');
    if let Some(disabled) = disabled {
        out.push_str(&format!(".disabled({})", expression(disabled)));
    }
    if let Some(accessibility_label) = accessibility_label {
        out.push_str(&format!(
            ".accessibilityLabel(Text({}))",
            expression(accessibility_label)
        ));
    }
}

pub(crate) fn render_swipe_action_button(button: &Node, depth: usize, out: &mut SourceWriter) {
    let Node::Button {
        label,
        icon,
        accessibility_label,
        loading,
        disabled,
        tint,
        actions,
        ..
    } = button
    else {
        return;
    };

    out.line_at(depth, format_args!("Button(action: {{"));
    render_actions(actions, depth + 1, out);
    out.line_at(depth, format_args!("}}) {{"));
    if let Some(loading) = loading {
        out.line_at(depth + 1, format_args!("if {} {{", expression(loading)));
        out.line_at(depth + 2, format_args!("ProgressView()"));
        out.line_at(depth + 1, format_args!("}} else {{"));
        render_button_label(label, None, icon.as_ref(), depth + 2, out);
        out.line_at(depth + 1, format_args!("}}"));
    } else {
        render_button_label(label, None, icon.as_ref(), depth + 1, out);
    }
    indent(out, depth);
    out.push('}');

    if disabled.is_some() || loading.is_some() {
        let disabled = disabled
            .as_ref()
            .map_or_else(|| "false".to_owned(), expression);
        let loading = loading
            .as_ref()
            .map_or_else(|| "false".to_owned(), expression);
        out.push_str(&format!(".disabled(({disabled}) || ({loading}))"));
    }
    if let Some(tint) = tint {
        out.push_str(&format!(
            ".tint({})",
            crate::generator::colors::expression_for_color(tint)
        ));
    }
    if let Some(accessibility_label) = accessibility_label {
        out.push_str(&format!(
            ".accessibilityLabel(Text({}))",
            expression(accessibility_label)
        ));
    }
}

fn render_button_label(
    label: &nexa_ir::Expr,
    comment: Option<&str>,
    icon: Option<&SystemIcon>,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    if let Some(icon) = icon {
        if icon.font_glyph_parts().is_some() {
            let font_icon = format!(
                "NexaIconView(icon: {}, description: \"\", size: CGFloat({}))",
                super::system_icons::spec(icon),
                nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE,
            );
            if comment.is_none() && is_empty_button_label(label) {
                out.push_str(&format!("{font_icon}\n"));
            } else {
                out.push_str(&format!(
                    "Label {{ {} }} icon: {{ {font_icon} }}.font(.system(size: {}))\n",
                    crate::generator::expressions::localized_text_view(label, comment),
                    nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE,
                ));
            }
            return;
        }
        if comment.is_none() && is_empty_button_label(label) {
            out.push_str(&format!(
                "Image(systemName: {}).font(.system(size: {}))\n",
                swift_string(&icon.sf_symbol_name()),
                nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE,
            ));
        } else if comment.is_none() && !has_localized_comment(label) {
            out.push_str(&format!(
                "Label({}, systemImage: {}).font(.system(size: {}))\n",
                crate::generator::engine::expressions::text_expression(label),
                swift_string(&icon.sf_symbol_name()),
                nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE,
            ));
        } else {
            out.push_str(&format!(
                "Label {{ {} }} icon: {{ Image(systemName: {}) }}.font(.system(size: {}))\n",
                crate::generator::expressions::localized_text_view(label, comment),
                swift_string(&icon.sf_symbol_name()),
                nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE,
            ));
        }
    } else {
        out.push_str(&format!(
            "{}.font(.system(size: {}))\n",
            crate::generator::expressions::localized_text_view(label, comment),
            nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE,
        ));
    }
}

fn is_empty_button_label(label: &nexa_ir::Expr) -> bool {
    match label {
        nexa_ir::Expr::String(text) => text.is_empty(),
        nexa_ir::Expr::LocalizedText {
            value,
            comment: None,
            ..
        } => matches!(value.as_ref(), nexa_ir::Expr::String(text) if text.is_empty()),
        _ => false,
    }
}

fn has_localized_comment(label: &nexa_ir::Expr) -> bool {
    matches!(
        label,
        nexa_ir::Expr::LocalizedText {
            comment: Some(_),
            ..
        }
    )
}

pub(crate) fn render_switch(
    state: &str,
    label: &nexa_ir::Expr,
    on_change: Option<&nexa_ir::SwitchChange>,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.text_at(
        depth,
        format_args!(
            "NexaSwitchPrimitive(label: {}, isOn: ${}, onChange: ",
            crate::generator::expressions::localized_text_view(label, None),
            state_name(state),
        ),
    );
    if let Some(change) = on_change {
        out.push_str(&format!("{{ {} in\n", state_name(&change.parameter)));
        render_actions(&change.actions, depth + 1, out);
        indent(out, depth);
        out.push_str("})");
    } else {
        out.push_str("nil)");
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_slider(
    state: &str,
    min: &nexa_ir::Expr,
    max: &nexa_ir::Expr,
    step: f64,
    tint: Option<&nexa_ir::ColorExpression>,
    track_color: Option<&nexa_ir::ColorExpression>,
    thumb_color: Option<&nexa_ir::ColorExpression>,
    track_height: Option<f32>,
    thumb_size: Option<f32>,
    hit_slop: Option<f32>,
    on_change: Option<&nexa_ir::SliderChange>,
    on_editing_changed: Option<&nexa_ir::SliderEditingChanged>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let color = |value: Option<&nexa_ir::ColorExpression>| {
        value.map_or_else(
            || "nil".to_owned(),
            crate::generator::colors::expression_for_color,
        )
    };
    let optional_number =
        |value: Option<f32>| value.map_or_else(|| "nil".to_owned(), |number| number.to_string());
    out.line_at(depth, format_args!("NexaSliderPrimitive("));
    out.line_at(
        depth + 1,
        format_args!(
            "value: ${}, minValue: {}, maxValue: {}, step: {},",
            state_name(state),
            expression(min),
            expression(max),
            step
        ),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "tint: {}, trackColor: {}, thumbColor: {},",
            color(tint),
            color(track_color),
            color(thumb_color)
        ),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "trackHeight: {}, thumbSize: {}, hitSlop: {},",
            optional_number(track_height),
            optional_number(thumb_size),
            optional_number(hit_slop)
        ),
    );
    out.line_at(depth + 1, format_args!("onChange: "));
    if let Some(change) = on_change {
        out.line_at(
            depth + 1,
            format_args!("{{ {} in", state_name(&change.parameter)),
        );
        render_actions(&change.actions, depth + 2, out);
        out.line_at(depth + 1, format_args!("}},"));
    } else {
        out.line_at(depth + 1, format_args!("nil,"));
    }
    out.line_at(depth + 1, format_args!("onEditingChanged: "));
    if let Some(change) = on_editing_changed {
        out.line_at(
            depth + 1,
            format_args!("{{ {} in", state_name(&change.parameter)),
        );
        render_actions(&change.actions, depth + 2, out);
        out.line_at(depth + 1, format_args!("}},"));
    } else {
        out.line_at(depth + 1, format_args!("nil,"));
    }
    out.line_at(depth, format_args!(")"));
}

pub(crate) fn render_progress_bar(
    progress: Option<&nexa_ir::Expr>,
    style: Option<&nexa_ir::ProgressBarStyle>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let tint = style.and_then(|style| style.tint.as_ref());
    let track_color = style.and_then(|style| style.track_color.as_ref());
    let thickness = style.and_then(|style| style.thickness);
    let corner_radius = style.and_then(|style| style.corner_radius);
    let tint = tint.map_or_else(
        || "nil".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let track_color = track_color.map_or_else(
        || "nil".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let thickness = thickness.map_or_else(|| "nil".to_owned(), |value| value.to_string());
    let corner_radius = corner_radius.map_or_else(|| "nil".to_owned(), |value| value.to_string());
    out.line_at(
        depth,
        format_args!(
            "NexaProgressBarPrimitive(progress: {}, tint: {}, trackColor: {}, thickness: {}, cornerRadius: {})",
            progress.map_or_else(|| "nil".to_owned(), expression), tint, track_color, thickness, corner_radius
        ),
    );
}

pub(crate) fn render_progress_ring(
    progress: Option<&nexa_ir::Expr>,
    style: Option<&nexa_ir::ProgressRingStyle>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let tint = style.and_then(|style| style.tint.as_ref()).map_or_else(
        || "nil".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let track_color = style
        .and_then(|style| style.track_color.as_ref())
        .map_or_else(
            || "nil".to_owned(),
            crate::generator::colors::expression_for_color,
        );
    let size = style
        .and_then(|style| style.size)
        .map_or_else(|| "nil".to_owned(), |value| value.to_string());
    let thickness = style
        .and_then(|style| style.thickness)
        .map_or_else(|| "nil".to_owned(), |value| value.to_string());
    out.line_at(
        depth,
        format_args!(
            "NexaProgressRingPrimitive(progress: {}, tint: {}, trackColor: {}, size: {}, thickness: {})",
            progress.map_or_else(|| "nil".to_owned(), expression), tint, track_color, size, thickness
        ),
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
    items: &nexa_ir::Expr,
    state: &str,
    animation: Option<AnimationSpec>,
    style: SegmentedControlStyle<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let animation = animation.map_or_else(|| "nil".to_owned(), swift_animation);
    let color = |value: Option<&nexa_ir::ColorExpression>| {
        value.map_or_else(
            || "nil".to_owned(),
            crate::generator::colors::expression_for_color,
        )
    };
    let tint = color(style.tint);
    let background = color(style.background);
    let text_color = color(style.text_color);
    let selected_text_color = color(style.selected_text_color);
    let font_size = style
        .font_size
        .map_or_else(|| "nil".to_owned(), |value| format!("{value}"));
    let font_weight = style.font_weight.map_or_else(
        || "nil".to_owned(),
        |weight| {
            match weight {
                nexa_ir::FontWeight::Normal => ".regular",
                nexa_ir::FontWeight::Medium => ".medium",
                nexa_ir::FontWeight::Semibold => ".semibold",
                nexa_ir::FontWeight::Bold => ".bold",
            }
            .to_owned()
        },
    );
    let height = style
        .height
        .map_or_else(|| "nil".to_owned(), |value| format!("{value}"));
    let corner_radius = style
        .corner_radius
        .map_or_else(|| "nil".to_owned(), |value| format!("{value}"));
    let border_color = color(style.border_color);
    let border_width = style
        .border_width
        .map_or_else(|| "nil".to_owned(), |value| format!("{value}"));
    out.line_at(
        depth,
        format_args!(
            "NexaSegmentedControlPrimitive(items: {}, selection: ${}, animation: {}, tint: {}, background: {}, textColor: {}, selectedTextColor: {}, fontSize: {}, fontWeight: {}, height: {}, cornerRadius: {}, borderColor: {}, borderWidth: {})",
            expression(items),
            state_name(state),
            animation,
            tint,
            background,
            text_color,
            selected_text_color,
            font_size,
            font_weight,
            height,
            corner_radius,
            border_color,
            border_width,
        ),
    );
}

#[derive(Default)]
pub(crate) struct PickerOptions<'a> {
    pub icon: Option<&'a SystemIcon>,
    pub item_icons: &'a [SystemIcon],
    pub label: Option<&'a nexa_ir::Expr>,
    pub tint: Option<&'a nexa_ir::ColorExpression>,
    pub comment: Option<&'a str>,
}

pub(crate) fn render_picker(
    items: &nexa_ir::Expr,
    state: &str,
    options: PickerOptions<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let PickerOptions {
        icon,
        item_icons,
        label,
        tint,
        comment,
    } = options;
    let selected = state_name(state);
    let icon = icon.map_or_else(|| "nil".to_owned(), super::system_icons::spec);
    let item_icons = item_icons
        .iter()
        .map(super::system_icons::spec)
        .collect::<Vec<_>>()
        .join(", ");
    let tint = tint.map_or_else(
        || "nil".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let has_label = label.is_some();
    out.line_at(
        depth,
        format_args!(
            "NexaPickerPrimitive(selection: ${selected}, items: {}, itemIcons: [{item_icons}], icon: {icon}, tint: {tint}, hasLabel: {has_label}) {{",
            expression(items),
        ),
    );
    if let Some(label) = label {
        out.line_at(
            depth + 1,
            format_args!(
                "{}",
                crate::generator::expressions::localized_text_view(label, comment)
            ),
        );
    } else {
        out.line_at(depth + 1, format_args!("EmptyView()"));
    }
    out.line_at(depth, format_args!("}}"));
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
            .map_or_else(|| "nil".to_owned(), |name| format!("${}", state_name(name)))
    };
    let optional_expr = |value: Option<&Expr>| value.map_or_else(|| "nil".to_owned(), expression);
    let color = |value: Option<&nexa_ir::ColorExpression>| {
        value.map_or_else(
            || "nil".to_owned(),
            crate::generator::colors::expression_for_color,
        )
    };
    let optional_float =
        |value: Option<f32>| value.map_or_else(|| "nil".to_owned(), |v| format!("{v}"));

    out.line_at(depth, format_args!("NexaDatePickerPrimitive("));
    out.line_at(
        depth + 1,
        format_args!("timestamp: {},", binding(timestamp_state)),
    );
    out.line_at(
        depth + 1,
        format_args!("startTimestamp: {},", binding(start_timestamp_state)),
    );
    out.line_at(
        depth + 1,
        format_args!("endTimestamp: {},", binding(end_timestamp_state)),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "pickerType: {},",
            crate::generator::engine::utils::swift_string(kind)
        ),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "style: {},",
            crate::generator::engine::utils::swift_string(style)
        ),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "selection: {},",
            crate::generator::engine::utils::swift_string(selection)
        ),
    );
    out.line_at(
        depth + 1,
        format_args!("minimum: {},", optional_expr(minimum.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!("maximum: {},", optional_expr(maximum.as_ref())),
    );
    out.line_at(depth + 1, format_args!("tint: {},", color(tint.as_ref())));
    out.line_at(
        depth + 1,
        format_args!("textColor: {},", color(text_color.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!("backgroundColor: {},", color(background_color.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "pickerBackgroundColor: {},",
            color(picker_background_color.as_ref())
        ),
    );
    out.line_at(
        depth + 1,
        format_args!("borderColor: {},", color(border_color.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!("borderWidth: {},", optional_float(*border_width)),
    );
    out.line_at(
        depth + 1,
        format_args!("cornerRadius: {},", optional_float(*corner_radius)),
    );
    out.line_at(
        depth + 1,
        format_args!("localeIdentifier: {},", optional_expr(locale.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!("timeZoneIdentifier: {},", optional_expr(time_zone.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!("is24Hour: {},", optional_expr(is_24_hour.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!("enabled: {},", optional_expr(enabled.as_ref())),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "accessibilityLabel: {},",
            optional_expr(accessibility_label.as_ref())
        ),
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
    let root_depth = depth;
    let has_long_press = !long_press_actions.is_empty();
    let has_drag = drag_parameters.len() == 4;
    let has_pinch = pinch_parameter.is_some();
    let has_drag_velocity = has_drag
        && nexa_ir::walk::actions_reference_state(
            drag_actions,
            &[drag_parameters[2].as_str(), drag_parameters[3].as_str()],
        );
    if has_drag || has_pinch {
        indent(out, root_depth);
        out.push_str("NexaDragGestureView(enabled: !( ");
        out.push_str(&expression(disabled));
        out.push_str("), trackVelocity: ");
        out.push_str(if has_drag_velocity { "true" } else { "false" });
        out.push_str(", onDrag: ");
        if has_drag {
            out.push_str("{ ");
            out.push_str(
                &drag_parameters
                    .iter()
                    .map(|name| state_name(name))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push_str(" in\n");
            render_actions(drag_actions, root_depth + 1, out);
            indent(out, root_depth);
            out.push('}');
        } else {
            out.push_str("nil");
        }
        out.push_str(", onPinch: ");
        if let Some(parameter) = pinch_parameter {
            out.push_str("{ ");
            out.push_str(&state_name(parameter));
            out.push_str(" in\n");
            render_actions(pinch_actions, root_depth + 1, out);
            indent(out, root_depth);
            out.push('}');
        } else {
            out.push_str("nil");
        }
        out.push_str(") {\n");
    }
    let depth = root_depth + usize::from(has_drag || has_pinch);
    indent(out, depth);
    out.push_str(if double_tap_actions.is_empty() && !has_long_press {
        "Button(action: {"
    } else {
        "Button(action: {}) {"
    });
    if !double_tap_actions.is_empty() || has_long_press {
        out.push('\n');
        render_children(children, module, features, depth + 1, out);
        out.push('\n');
        indent(out, depth);
        out.push('}');
        out.push_str(".buttonStyle(.plain)");
        if *fill_max_size {
            out.push_str(
                ".frame(maxWidth: .infinity, maxHeight: .infinity).contentShape(Rectangle())",
            );
        }
        if !matches!(disabled, nexa_ir::Expr::Bool(false)) {
            out.push_str(".disabled(");
            out.push_str(&expression(disabled));
            out.push(')');
        }
        out.push_str(if has_long_press {
            ".highPriorityGesture(\n"
        } else {
            ".gesture(\n"
        });
        if has_long_press {
            indent(out, depth + 1);
            out.push_str("LongPressGesture(minimumDuration: ");
            out.push_str(&long_press_duration_seconds(long_press_duration_ms));
            out.push_str(")\n");
            indent(out, depth + 2);
            out.push_str(".onEnded { _ in\n");
            render_pressable_gesture_actions(disabled, *haptic, long_press_actions, depth + 3, out);
            indent(out, depth + 2);
            out.push_str("}\n");
            indent(out, depth + 2);
            out.push_str(".exclusively(before:\n");
        }
        if !double_tap_actions.is_empty() {
            indent(out, depth + 1 + usize::from(has_long_press));
            out.push_str("TapGesture(count: 2)\n");
            indent(out, depth + 2 + usize::from(has_long_press));
            out.push_str(".onEnded {\n");
            render_pressable_gesture_actions(
                disabled,
                *haptic,
                double_tap_actions,
                depth + 3 + usize::from(has_long_press),
                out,
            );
            indent(out, depth + 2 + usize::from(has_long_press));
            out.push_str("}\n");
            indent(out, depth + 2 + usize::from(has_long_press));
            out.push_str(".exclusively(before:\n");
        }
        let single_tap_depth =
            depth + 1 + usize::from(has_long_press || !double_tap_actions.is_empty());
        indent(out, single_tap_depth);
        out.push_str("TapGesture(count: 1)\n");
        indent(out, single_tap_depth + 1);
        out.push_str(".onEnded {\n");
        render_pressable_gesture_actions(disabled, *haptic, actions, single_tap_depth + 2, out);
        indent(out, single_tap_depth + 1);
        out.push_str("}\n");
        if !double_tap_actions.is_empty() {
            indent(out, depth + 2 + usize::from(has_long_press));
            out.push_str(")\n");
        }
        if has_long_press {
            indent(out, depth + 2);
            out.push_str(")\n");
        }
        indent(out, depth);
        out.push(')');
        out.push_str(".accessibilityAction(.default) {");
        if let Some(haptic) = haptic {
            out.push('\n');
            render_haptic(*haptic, depth + 1, out);
        }
        if !actions.is_empty() {
            out.push('\n');
            render_actions(actions, depth + 1, out);
            indent(out, depth);
        }
        out.push('}');
        out.push_str(".accessibilityAddTraits(.isButton)");
        render_context_menu(context_menu, module, features, depth, out);
        if has_drag || has_pinch {
            out.push('\n');
            indent(out, root_depth);
            out.push('}');
        }
        return;
    }
    if actions.is_empty() {
        if let Some(haptic) = haptic {
            out.push('\n');
            render_haptic(*haptic, depth + 1, out);
            indent(out, depth);
            out.push_str("}) {");
        } else {
            out.push_str(" }) {");
        }
    } else {
        out.push('\n');
        if let Some(haptic) = haptic {
            render_haptic(*haptic, depth + 1, out);
        }
        render_actions(actions, depth + 1, out);
        indent(out, depth);
        out.push_str("}) {");
    }
    out.push('\n');
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
    if !matches!(disabled, nexa_ir::Expr::Bool(false)) {
        out.push_str(".disabled(");
        out.push_str(&expression(disabled));
        out.push(')');
    }
    out.push_str(".buttonStyle(.plain)");
    if *fill_max_size {
        out.push_str(".frame(maxWidth: .infinity, maxHeight: .infinity).contentShape(Rectangle())");
    }
    render_context_menu(context_menu, module, features, depth, out);
    if has_drag || has_pinch {
        out.push('\n');
        indent(out, root_depth);
        out.push('}');
    }
}

fn render_context_menu(
    context_menu: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    if context_menu.is_empty() {
        return;
    }
    out.push_str(".contextMenu {\n");
    render_children(context_menu, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

fn long_press_duration_seconds(duration_ms: &Expr) -> String {
    format!("(Double(max(1, {})) / 1000.0)", expression(duration_ms))
}

fn render_pressable_gesture_actions(
    disabled: &Expr,
    haptic: Option<HapticStyle>,
    actions: &[Action],
    depth: usize,
    out: &mut SourceWriter,
) {
    if matches!(disabled, Expr::Bool(true)) {
        return;
    }

    let needs_disabled_guard = !matches!(disabled, Expr::Bool(false));
    if needs_disabled_guard {
        out.line_at(depth, format_args!("if !({}) {{", expression(disabled)));
    }
    let action_depth = depth + usize::from(needs_disabled_guard);
    if let Some(haptic) = haptic {
        render_haptic(haptic, action_depth, out);
    }
    render_actions(actions, action_depth, out);
    if needs_disabled_guard {
        out.line_at(depth, format_args!("}}"));
    }
}

fn render_haptic(style: HapticStyle, depth: usize, out: &mut SourceWriter) {
    indent(out, depth);
    let style = match style {
        HapticStyle::Light => "light",
        HapticStyle::Medium => "medium",
        HapticStyle::Heavy => "heavy",
    };
    out.push_str(&format!(
        "UIImpactFeedbackGenerator(style: .{style}).impactOccurred()\n"
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
        out.push_str(" in");
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
    render_actions_with_cancellation(actions, depth, out, false, CancellationMode::Consume);
}

pub(crate) fn render_actions_in_task_scope(
    actions: &[Action],
    depth: usize,
    out: &mut SourceWriter,
) {
    render_actions_with_cancellation(actions, depth, out, true, CancellationMode::Consume);
}

/// Renders a function body with cancellation handling that matches its declared
/// error contract. A nonthrowing Nexa function returns a typed fallback when
/// cancellation reaches a local `try` boundary.
pub(crate) fn render_function_actions(
    actions: &[Action],
    depth: usize,
    out: &mut SourceWriter,
    is_throwing: bool,
    return_type: &Type,
) {
    let cancellation_return = cancellation_return_statement(return_type);
    render_actions_with_cancellation(
        actions,
        depth,
        out,
        false,
        if is_throwing {
            CancellationMode::Propagate
        } else {
            CancellationMode::Return(&cancellation_return)
        },
    );
}

pub(crate) fn render_task_body(
    actions: &[Action],
    depth: usize,
    out: &mut SourceWriter,
    track_handles: bool,
) {
    if task_actions_may_throw(actions) {
        out.line_at(depth, format_args!("do {{"));
        render_actions_with_task_scope(actions, depth + 1, out, track_handles);
        out.line_at(depth, format_args!("}} catch is CancellationError {{"));
        out.line_at(depth + 1, format_args!("return"));
        out.line_at(depth, format_args!("}} catch {{"));
        out.line_at(
            depth + 1,
            format_args!("fatalError(\"unhandled error escaped a typed Nexa task\")"),
        );
        out.line_at(depth, format_args!("}}"));
    } else {
        render_actions_with_task_scope(actions, depth, out, track_handles);
    }
}

fn task_actions_may_throw(actions: &[Action]) -> bool {
    actions.iter().any(|action| match action {
        Action::Expression(expression)
        | Action::Let {
            value: expression, ..
        }
        | Action::Return { value: expression }
        | Action::Assign {
            value: expression, ..
        } => expression_may_throw(expression),
        Action::NavigationPush { arguments, .. } | Action::CollectionMutation { arguments, .. } => {
            arguments.iter().any(expression_may_throw)
        }
        Action::NativePropertyAssign {
            receiver, value, ..
        } => expression_may_throw(receiver) || expression_may_throw(value),
        Action::NativeEventSubscribe { receiver, .. } => expression_may_throw(receiver),
        Action::NetworkStatusSubscribe { .. } | Action::TaskLaunch { .. } => false,
        Action::WithAnimation { actions, .. } => task_actions_may_throw(actions),
        Action::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expression_may_throw(condition)
                || task_actions_may_throw(then_branch)
                || else_branch.as_deref().is_some_and(task_actions_may_throw)
        }
        Action::For { iterable, body, .. } | Action::ForMap { iterable, body, .. } => {
            expression_may_throw(iterable) || task_actions_may_throw(body)
        }
        Action::While { condition, body } => {
            expression_may_throw(condition) || task_actions_may_throw(body)
        }
        Action::TryCatch {
            error_catches,
            catch_body,
            ..
        } => {
            error_catches
                .iter()
                .any(|catch| task_actions_may_throw(&catch.body))
                || catch_body.as_deref().is_some_and(task_actions_may_throw)
        }
        Action::NavigationBack | Action::TaskCancel { .. } | Action::Break | Action::Continue => {
            false
        }
        Action::NavigationCommand(cmd) => match cmd.as_ref() {
            nexa_ir::NavigationCommand::Push { arguments, .. }
            | nexa_ir::NavigationCommand::ReplaceTop { arguments, .. }
            | nexa_ir::NavigationCommand::Reset { arguments, .. } => {
                arguments.iter().any(expression_may_throw)
            }
            _ => false,
        },
    })
}

fn expression_may_throw(expression: &Expr) -> bool {
    let mut may_throw = false;
    nexa_ir::walk::walk_expression(expression, &mut |nested| {
        may_throw |= nested.is_throwing_call() || matches!(nested, Expr::Try { .. });
    });
    may_throw
}

fn expression_returns_void(expression: &Expr) -> bool {
    match expression {
        Expr::Located { expression, .. } => expression_returns_void(expression),
        Expr::Call { return_type, .. } | Expr::NativeCall { return_type, .. } => {
            *return_type == Type::Void
        }
        Expr::Await(value) | Expr::TryAwait(value) => expression_returns_void(value),
        Expr::Try { value_type, .. } => *value_type == Type::Void,
        Expr::TimeCall {
            method: nexa_ir::TimeMethod::Sleep,
            ..
        }
        | Expr::LogCall { .. } => true,
        _ => false,
    }
}

fn actions_terminate(actions: &[Action]) -> bool {
    match actions.last() {
        Some(Action::Return { .. }) => true,
        Some(Action::If {
            then_branch,
            else_branch: Some(else_branch),
            ..
        }) => actions_terminate(then_branch) && actions_terminate(else_branch),
        _ => false,
    }
}

fn render_actions_with_task_scope(
    actions: &[Action],
    depth: usize,
    out: &mut SourceWriter,
    track_handles: bool,
) {
    render_actions_with_cancellation(
        actions,
        depth,
        out,
        track_handles,
        CancellationMode::Return("return"),
    );
}

#[derive(Clone, Copy)]
enum CancellationMode<'a> {
    Propagate,
    Consume,
    Return(&'a str),
}

fn cancellation_return_statement(ty: &Type) -> String {
    match ty {
        Type::Void => "return".to_owned(),
        Type::String => "return \"\"".to_owned(),
        Type::Bytes => "return Data()".to_owned(),
        Type::Bool => "return false".to_owned(),
        Type::Numeric(_) => "return 0".to_owned(),
        Type::Optional(_) => "return nil".to_owned(),
        Type::Array(_) | Type::Set(_) => "return []".to_owned(),
        Type::Map(_, _) => "return [:]".to_owned(),
        Type::Pair(first, second) => format!(
            "return ({}, {})",
            cancellation_default_value(first),
            cancellation_default_value(second)
        ),
        Type::Triple(first, second, third) => format!(
            "return ({}, {}, {})",
            cancellation_default_value(first),
            cancellation_default_value(second),
            cancellation_default_value(third)
        ),
        Type::Struct { name, fields } => {
            let arguments = fields
                .iter()
                .map(|(field, ty)| {
                    format!(
                        "{}: {}",
                        nexa_codegen::names::struct_field_name(field),
                        cancellation_default_value(ty)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "return {}({arguments})",
                nexa_codegen::names::struct_name(name)
            )
        }
        Type::Class {
            name,
            fields,
            constructor_parameter_count,
        } => {
            let arguments = fields
                .iter()
                .take(*constructor_parameter_count)
                .map(|(_, ty)| cancellation_default_value(ty))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "return {}({arguments})",
                nexa_codegen::names::struct_name(name)
            )
        }
        Type::Result(_, _)
        | Type::Signal(_)
        | Type::ComponentRef { .. }
        | Type::ComponentRefError(_)
        | Type::Enum(_)
        | Type::TypeParam(_)
        | Type::Plugin { .. }
        | Type::TaskHandle
        | Type::NetworkResponse => {
            "fatalError(\"cancelled while producing a value without a default\")".to_owned()
        }
    }
}

fn cancellation_default_value(ty: &Type) -> String {
    match ty {
        Type::Void => "()".to_owned(),
        Type::String => "\"\"".to_owned(),
        Type::Bytes => "Data()".to_owned(),
        Type::Bool => "false".to_owned(),
        Type::Numeric(_) => "0".to_owned(),
        Type::Optional(_) => "nil".to_owned(),
        Type::Array(_) | Type::Set(_) => "[]".to_owned(),
        Type::Map(_, _) => "[:]".to_owned(),
        Type::Pair(first, second) => format!(
            "({}, {})",
            cancellation_default_value(first),
            cancellation_default_value(second)
        ),
        Type::Triple(first, second, third) => format!(
            "({}, {}, {})",
            cancellation_default_value(first),
            cancellation_default_value(second),
            cancellation_default_value(third)
        ),
        Type::Struct { name, fields } => {
            let arguments = fields
                .iter()
                .map(|(field, ty)| {
                    format!(
                        "{}: {}",
                        nexa_codegen::names::struct_field_name(field),
                        cancellation_default_value(ty)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({arguments})", nexa_codegen::names::struct_name(name))
        }
        Type::Class {
            name,
            fields,
            constructor_parameter_count,
        } => {
            let arguments = fields
                .iter()
                .take(*constructor_parameter_count)
                .map(|(_, ty)| cancellation_default_value(ty))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({arguments})", nexa_codegen::names::struct_name(name))
        }
        Type::Result(_, _)
        | Type::Signal(_)
        | Type::ComponentRef { .. }
        | Type::ComponentRefError(_)
        | Type::Enum(_)
        | Type::TypeParam(_)
        | Type::Plugin { .. }
        | Type::TaskHandle
        | Type::NetworkResponse => "fatalError(\"no cancellation default\")".to_owned(),
    }
}

fn render_actions_with_cancellation(
    actions: &[Action],
    depth: usize,
    out: &mut SourceWriter,
    track_handles: bool,
    cancellation_mode: CancellationMode,
) {
    for action in actions {
        match action {
            Action::Expression(value) => {
                let rendered = expression(value);
                if expression_returns_void(value) {
                    out.line_at(depth, format_args!("{rendered}"));
                } else {
                    out.line_at(depth, format_args!("_ = {rendered}"));
                }
            }
            Action::NavigationPush {
                destination,
                arguments,
            } => {
                out.line_at(
                    depth,
                    format_args!(
                        "nexaNavigationPath.wrappedValue.append({})",
                        super::navigation::action_route_value(*destination, arguments)
                    ),
                );
            }
            Action::NavigationBack => {
                out.line_at(
                    depth,
                    format_args!("if !nexaNavigationPath.wrappedValue.isEmpty {{ nexaNavigationPath.wrappedValue.removeLast() }}"),
                );
            }
            Action::NavigationCommand(cmd) => match cmd.as_ref() {
                nexa_ir::NavigationCommand::Push {
                    route, arguments, ..
                } => {
                    let dest = route.as_str();
                    let screen_id = resolve_screen_id(dest);
                    if let Some(screen_id) = screen_id {
                        out.line_at(
                            depth,
                            format_args!(
                                "nexaNavigationPath.wrappedValue.append({})",
                                super::navigation::action_route_value(screen_id, arguments)
                            ),
                        );
                    } else {
                        let mut values = vec!["UUID()".to_owned()];
                        values.extend(arguments.iter().map(expression));
                        out.line_at(
                            depth,
                            format_args!(
                                "nexaNavigationPath.wrappedValue.append(NexaNavigationRoute.{dest}({}))",
                                values.join(", ")
                            ),
                        );
                    }
                }
                nexa_ir::NavigationCommand::Pop { .. } => {
                    out.line_at(
                        depth,
                        format_args!("if !nexaNavigationPath.wrappedValue.isEmpty {{ nexaNavigationPath.wrappedValue.removeLast() }}"),
                    );
                }
                nexa_ir::NavigationCommand::PopTo {
                    entry, inclusive, ..
                } => {
                    let entry_str = entry.as_str();
                    if *inclusive {
                        out.line_at(
                            depth,
                            format_args!("if let idx = nexaNavigationPath.wrappedValue.firstIndex(where: {{ \"\\($0.identity)\" == \"{entry_str}\" }}) {{ nexaNavigationPath.wrappedValue.removeSubrange(idx...) }}"),
                        );
                    } else {
                        out.line_at(
                            depth,
                            format_args!("if let idx = nexaNavigationPath.wrappedValue.firstIndex(where: {{ \"\\($0.identity)\" == \"{entry_str}\" }}), idx + 1 < nexaNavigationPath.wrappedValue.count {{ nexaNavigationPath.wrappedValue.removeSubrange((idx + 1)...) }}"),
                        );
                    }
                }
                nexa_ir::NavigationCommand::ReplaceTop {
                    route, arguments, ..
                } => {
                    let dest = route.as_str();
                    let screen_id = resolve_screen_id(dest);
                    out.line_at(
                        depth,
                        format_args!("if !nexaNavigationPath.wrappedValue.isEmpty {{ nexaNavigationPath.wrappedValue.removeLast() }}"),
                    );
                    if let Some(screen_id) = screen_id {
                        out.line_at(
                            depth,
                            format_args!(
                                "nexaNavigationPath.wrappedValue.append({})",
                                super::navigation::action_route_value(screen_id, arguments)
                            ),
                        );
                    } else {
                        let mut values = vec!["UUID()".to_owned()];
                        values.extend(arguments.iter().map(expression));
                        out.line_at(
                            depth,
                            format_args!(
                                "nexaNavigationPath.wrappedValue.append(NexaNavigationRoute.{dest}({}))",
                                values.join(", ")
                            ),
                        );
                    }
                }
                nexa_ir::NavigationCommand::Reset {
                    route, arguments, ..
                } => {
                    let dest = route.as_str();
                    let screen_id = resolve_screen_id(dest);
                    out.line_at(
                        depth,
                        format_args!("nexaNavigationPath.wrappedValue.removeAll()"),
                    );
                    if let Some(screen_id) = screen_id {
                        out.line_at(
                            depth,
                            format_args!(
                                "nexaNavigationPath.wrappedValue.append({})",
                                super::navigation::action_route_value(screen_id, arguments)
                            ),
                        );
                    } else {
                        let mut values = vec!["UUID()".to_owned()];
                        values.extend(arguments.iter().map(expression));
                        out.line_at(
                            depth,
                            format_args!(
                                "nexaNavigationPath.wrappedValue.append(NexaNavigationRoute.{dest}({}))",
                                values.join(", ")
                            ),
                        );
                    }
                }
                nexa_ir::NavigationCommand::SelectTab { navigator } => {
                    let nav = navigator.as_str();
                    out.line_at(
                        depth,
                        format_args!("NotificationCenter.default.post(name: NSNotification.Name(\"NexaSelectTab\"), object: \"{nav}\")"),
                    );
                }
                nexa_ir::NavigationCommand::OpenExternalRoute { uri } => {
                    out.line_at(
                        depth,
                        format_args!("if let url = URL(string: \"{uri}\") {{ UIApplication.shared.open(url) }}"),
                    );
                }
            },
            Action::Let { name, ty, value } => {
                out.line_at(
                    depth,
                    format_args!(
                        "let {}: {} = {}",
                        state_name(name),
                        swift_type(ty),
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
                    out.push_str(" in");
                }
                out.push('\n');
                render_actions_with_cancellation(
                    actions,
                    depth + 1,
                    out,
                    track_handles,
                    CancellationMode::Consume,
                );
                indent(out, depth);
                out.push_str("}\n");
            }
            Action::NetworkStatusSubscribe { parameter, actions } => {
                out.text_at(depth, format_args!("NexaNetwork.onStatusChange {{ "));
                out.push_str(&state_name(parameter));
                out.push_str(" in\n");
                render_actions_with_cancellation(
                    actions,
                    depth + 1,
                    out,
                    track_handles,
                    CancellationMode::Consume,
                );
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
                    indent(out, depth);
                    out.push_str(&format!("{handle} = "));
                    if track_handles {
                        out.push_str("nexaTaskScope.track(");
                    }
                } else {
                    indent(out, depth);
                    out.push_str("_ = nexaTaskScope.track(");
                }
                match executor {
                    TaskExecutor::Main => out.push_str("Task { @MainActor in\n"),
                    TaskExecutor::Background => out.push_str("Task.detached {\n"),
                }
                render_task_body(actions, depth + 1, out, track_handles);
                indent(out, depth);
                out.push('}');
                if handle.is_none() || track_handles {
                    out.push_str(")\n");
                } else {
                    out.push('\n');
                }
            }
            Action::TaskCancel { handle } => {
                out.line_at(depth, format_args!("{}?.cancel()", state_name(handle)));
            }
            Action::WithAnimation {
                animation,
                animated_states: _,
                actions,
            } => {
                indent(out, depth);
                out.push_str("withAnimation(");
                out.push_str(&swift_animation(*animation));
                out.push_str(") {\n");
                render_actions_with_cancellation(
                    actions,
                    depth + 1,
                    out,
                    track_handles,
                    CancellationMode::Consume,
                );
                indent(out, depth);
                out.push_str("}\n");
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
                        out.push_str(&format!("{state}.append({})\n", rendered[0]));
                    }
                    CollectionMutation::ArrayRemoveAt => {
                        out.push_str(&format!("{state}.remove(at: Int({}))\n", rendered[0]));
                    }
                    CollectionMutation::ArrayMove => {
                        let suffix = out.next_id();
                        let from = format!("nexaMoveFrom{suffix}");
                        let to = format!("nexaMoveTo{suffix}");
                        let moved_value = format!("nexaMovedValue{suffix}");
                        let nested_indent = "    ".repeat(depth + 1);
                        let line_indent = "    ".repeat(depth);
                        out.push_str(&format!(
                            "let {from} = Int({})\n{line_indent}let {to} = Int({})\n{line_indent}if {state}.indices.contains({from}) && {state}.indices.contains({to}) && {from} != {to} {{\n{nested_indent}let {moved_value} = {state}.remove(at: {from})\n{nested_indent}{state}.insert({moved_value}, at: {to})\n{line_indent}}}\n",
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
                            "let {from} = Int({})\n{line_indent}let {to} = Int({})\n{line_indent}let {original} = {}\n{line_indent}var {reordered} = {original}\n{line_indent}if {from} >= 0 && {from} < {reordered}.count && {to} >= 0 && {to} < {reordered}.count && {from} != {to} {{\n{nested_indent}let {moved} = {reordered}.remove(at: {from})\n{nested_indent}{reordered}.insert({moved}, at: {to})\n{line_indent}}}\n{line_indent}var {cursor} = 0\n{line_indent}var {slots}: [Int] = []\n{line_indent}for nexaBackingIndex{suffix} in {state}.indices where {cursor} < {original}.count {{\n{nested_indent}if {state}[nexaBackingIndex{suffix}] == {original}[{cursor}] {{\n{inner_indent}{slots}.append(nexaBackingIndex{suffix})\n{inner_indent}{cursor} += 1\n{nested_indent}}}\n{line_indent}}}\n{line_indent}if {cursor} == {original}.count {{\n{nested_indent}for nexaSubsetIndex{suffix} in {original}.indices {{\n{inner_indent}{state}[{slots}[nexaSubsetIndex{suffix}]] = {reordered}[nexaSubsetIndex{suffix}]\n{nested_indent}}}\n{line_indent}}}\n",
                            rendered[0], rendered[1], rendered[2],
                        ));
                    }
                    CollectionMutation::SetInsert => {
                        out.push_str(&format!("{state}.insert({})\n", rendered[0]));
                    }
                    CollectionMutation::SetRemove => {
                        out.push_str(&format!("{state}.remove({})\n", rendered[0]));
                    }
                    CollectionMutation::MapSet => {
                        out.push_str(&format!("{state}[{}] = {}\n", rendered[0], rendered[1]));
                    }
                    CollectionMutation::MapRemove => {
                        out.push_str(&format!("{state}.removeValue(forKey: {})\n", rendered[0]));
                    }
                    CollectionMutation::MapClear => {
                        out.push_str(&format!("{state}.removeAll(keepingCapacity: true)\n"));
                    }
                    CollectionMutation::Replace => {
                        out.push_str(&format!("{state} = {}\n", rendered[0]));
                    }
                }
            }
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                out.line_at(depth, format_args!("if {} {{", expression(condition)));
                render_actions_with_cancellation(
                    then_branch,
                    depth + 1,
                    out,
                    track_handles,
                    cancellation_mode,
                );
                if let Some(else_branch) = else_branch {
                    indent(out, depth);
                    out.push_str("} else {\n");
                    render_actions_with_cancellation(
                        else_branch,
                        depth + 1,
                        out,
                        track_handles,
                        cancellation_mode,
                    );
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
                    format_args!("for {} in {} {{", state_name(name), expression(iterable)),
                );
                render_actions_with_cancellation(
                    body,
                    depth + 1,
                    out,
                    track_handles,
                    cancellation_mode,
                );
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
                        "for ({}, {}) in {} {{",
                        state_name(key_name),
                        state_name(value_name),
                        expression(iterable)
                    ),
                );
                render_actions_with_cancellation(
                    body,
                    depth + 1,
                    out,
                    track_handles,
                    cancellation_mode,
                );
                indent(out, depth);
                out.push_str("}\n");
            }
            Action::While { condition, body } => {
                out.line_at(depth, format_args!("while {} {{", expression(condition)));
                render_actions_with_cancellation(
                    body,
                    depth + 1,
                    out,
                    track_handles,
                    cancellation_mode,
                );
                indent(out, depth);
                out.push_str("}\n");
            }
            Action::TryCatch {
                body,
                error_catches,
                catch_body,
            } => {
                indent(out, depth);
                out.push_str("do {\n");
                render_actions_with_cancellation(
                    body,
                    depth + 1,
                    out,
                    track_handles,
                    cancellation_mode,
                );
                render_swift_error_catches(
                    error_catches,
                    catch_body.as_deref(),
                    depth,
                    out,
                    track_handles,
                    cancellation_mode,
                );
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

pub(crate) fn swift_animation(animation: AnimationSpec) -> String {
    match animation {
        AnimationSpec::Spring { response, damping } => {
            let physics = nexa_codegen::animation::spring_physics(response, damping);
            format!(
                ".interpolatingSpring(mass: 1, stiffness: {}, damping: {}, initialVelocity: 0)",
                physics.stiffness, physics.damping_coefficient
            )
        }
        AnimationSpec::Tween {
            duration,
            delay,
            easing,
        } => swift_tween(duration, easing, delay),
        AnimationSpec::EaseIn => swift_tween(0.3, AnimationEasing::EaseIn, 0.0),
        AnimationSpec::EaseOut => swift_tween(0.3, AnimationEasing::EaseOut, 0.0),
        AnimationSpec::EaseInOut => swift_tween(0.3, AnimationEasing::EaseInOut, 0.0),
        AnimationSpec::Linear => swift_tween(0.3, AnimationEasing::Linear, 0.0),
    }
}

fn swift_tween(duration: f64, easing: AnimationEasing, delay: f64) -> String {
    let mut animation = match easing {
        AnimationEasing::Linear => format!(".linear(duration: {duration})"),
        AnimationEasing::EaseIn => {
            format!(".timingCurve(0.4, 0.0, 1.0, 1.0, duration: {duration})")
        }
        AnimationEasing::EaseOut => {
            format!(".timingCurve(0.0, 0.0, 0.2, 1.0, duration: {duration})")
        }
        AnimationEasing::EaseInOut => {
            format!(".timingCurve(0.4, 0.0, 0.2, 1.0, duration: {duration})")
        }
        AnimationEasing::CubicBezier { x1, y1, x2, y2 } => {
            format!(".timingCurve({x1}, {y1}, {x2}, {y2}, duration: {duration})")
        }
    };
    if delay > 0.0 {
        animation.push_str(&format!(".delay({delay})"));
    }
    animation
}

fn render_swift_error_catches(
    catches: &[ErrorCatchArm],
    catch_all: Option<&[Action]>,
    depth: usize,
    out: &mut SourceWriter,
    track_handles: bool,
    cancellation_mode: CancellationMode,
) {
    let mut groups: Vec<(&str, &str, Vec<&ErrorCatchArm>)> = Vec::new();
    for arm in catches {
        if let Some((_, _, arms)) = groups.iter_mut().find(|(namespace, error_type, _)| {
            *namespace == arm.namespace && *error_type == arm.error_type
        }) {
            arms.push(arm);
        } else {
            groups.push((&arm.namespace, &arm.error_type, vec![arm]));
        }
    }

    for (_, error_type, arms) in groups {
        out.line_at(depth, format_args!("}} catch let error as {error_type} {{"));
        indent(out, depth + 1);
        out.push_str("switch error {\n");
        for arm in arms {
            indent(out, depth + 2);
            if arm.parameters.is_empty() {
                out.push_str(&format!("case .{}:\n", arm.variant));
            } else {
                let bindings = arm
                    .parameters
                    .iter()
                    .map(|(name, _, _)| state_name(name))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("case let .{}({bindings}):\n", arm.variant));
            }
            render_actions_with_cancellation(
                &arm.body,
                depth + 3,
                out,
                track_handles,
                cancellation_mode,
            );
            // Swift does not implicitly terminate an enum switch case. Emit a
            // break unless this arm already returns from its enclosing function.
            if !actions_terminate(&arm.body) {
                indent(out, depth + 3);
                out.push_str("break\n");
            }
        }
        if let Some(catch_all) = catch_all {
            indent(out, depth + 2);
            out.push_str("default:\n");
            render_actions_with_cancellation(
                catch_all,
                depth + 3,
                out,
                track_handles,
                cancellation_mode,
            );
        }
        indent(out, depth + 1);
        out.push_str("}\n");
    }

    indent(out, depth);
    out.push_str("} catch is CancellationError {\n");
    match cancellation_mode {
        CancellationMode::Propagate => {
            indent(out, depth + 1);
            out.push_str("throw CancellationError()\n");
        }
        CancellationMode::Consume => {}
        CancellationMode::Return(statement) => {
            out.line_at(depth + 1, format_args!("{statement}"));
        }
    }

    if catches.is_empty() || catch_all.is_some() {
        indent(out, depth);
        out.push_str("} catch {\n");
        if let Some(catch_all) = catch_all {
            render_actions_with_cancellation(
                catch_all,
                depth + 1,
                out,
                track_handles,
                cancellation_mode,
            );
        }
        indent(out, depth);
        out.push_str("}\n");
    } else {
        indent(out, depth);
        out.push_str("} catch {\n");
        indent(out, depth + 1);
        out.push_str("fatalError(\"unexpected error escaped its typed plugin contract\")\n");
        indent(out, depth);
        out.push_str("}\n");
    }
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{
        Action, CollectionMutation, ColorExpression, Expr, NumericType, SystemIcon, TaskExecutor,
        Type,
    };

    use super::{
        PickerOptions, render_actions, render_button, render_function_actions, render_picker,
        render_progress_bar, render_progress_ring, render_slider, render_task_body,
    };

    fn float_expr(raw: &str) -> Expr {
        Expr::Number {
            raw: raw.to_owned(),
            ty: NumericType::Float64,
        }
    }

    #[test]
    fn icon_picker_uses_the_selected_dynamic_tint() {
        let items = Expr::Array(vec![Expr::String("High".to_owned())]);
        let tint = ColorExpression::Dynamic(Box::new(Expr::State(
            "priorityTint".to_owned(),
            Type::String,
        )));
        let mut output = SourceWriter::new();

        render_picker(
            &items,
            "priority",
            PickerOptions {
                icon: Some(&SystemIcon::Shared("flag_filled".to_owned())),
                item_icons: &[],
                label: None,
                tint: Some(&tint),
                comment: None,
            },
            0,
            &mut output,
        );

        assert!(output.contains(
            "NexaPickerPrimitive(selection: $nexa_priority, items: [\"High\"], itemIcons: [], icon: NexaIconSpec(symbol: \"flag.fill\"), tint: nexaColor(hex: nexa_priorityTint), hasLabel: false) {"
        ));
        assert!(output.contains("EmptyView()"));
    }

    #[test]
    fn picker_generates_icons_for_each_menu_row() {
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
            "priority",
            PickerOptions {
                icon: None,
                item_icons: &item_icons,
                label: Some(&Expr::String("Priority".to_owned())),
                tint: None,
                comment: None,
            },
            0,
            &mut output,
        );

        assert!(output.contains("itemIcons: [NexaIconSpec(symbol: \"calendar\"), NexaIconSpec(symbol: \"flag.fill\"), NexaIconSpec(symbol: \"pencil\")]"));
    }

    #[test]
    fn borderless_button_uses_shared_label_metrics_and_tap_target() {
        let mut output = SourceWriter::new();
        render_button(
            &Expr::String("Skip".to_owned()),
            None,
            None,
            None,
            None,
            None,
            Some(nexa_ir::ButtonStyle::Borderless),
            None,
            None,
            None,
            false,
            &[],
            0,
            &mut output,
        );

        assert!(output.contains("NexaButtonPrimitive("));
        assert!(output.contains("style: .borderless"));
        assert!(output.contains("size: nil"));
        assert!(output.contains("disabled: false"));
        assert!(output.contains("action: {"));
        assert!(output.contains(".font(.system(size: 17))"));
        assert!(output.contains("label: {"));
    }

    #[test]
    fn icon_only_button_uses_a_direct_image_label() {
        let mut output = SourceWriter::new();
        render_button(
            &Expr::LocalizedText {
                key: String::new(),
                value: Box::new(Expr::String(String::new())),
                comment: None,
            },
            None,
            None,
            Some(&SystemIcon::Shared("close".to_owned())),
            None,
            None,
            Some(nexa_ir::ButtonStyle::Plain),
            None,
            None,
            None,
            false,
            &[],
            0,
            &mut output,
        );

        assert!(output.contains("Image(systemName: \"xmark\").font(.system(size: 17))"));
        assert!(!output.contains("Label { Text(\"\") }"));
    }

    #[test]
    fn button_accessibility_label_is_emitted_for_icon_only_actions() {
        let mut output = SourceWriter::new();
        render_button(
            &Expr::String(String::new()),
            None,
            Some(&Expr::String("Add".to_owned())),
            Some(&SystemIcon::Shared("add".to_owned())),
            None,
            None,
            Some(nexa_ir::ButtonStyle::BorderedProminent),
            None,
            Some(nexa_ir::ButtonShape::Circle),
            None,
            false,
            &[],
            0,
            &mut output,
        );

        assert!(output.contains("accessibilityLabel: \"Add\""));
    }

    #[test]
    fn button_text_is_the_default_accessibility_label() {
        let mut output = SourceWriter::new();
        render_button(
            &Expr::String("Save changes".to_owned()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            false,
            &[],
            0,
            &mut output,
        );

        assert!(output.contains("accessibilityLabel: \"Save changes\""));
    }

    #[test]
    fn icon_button_uses_the_native_string_label_initializer() {
        let mut output = SourceWriter::new();
        render_button(
            &Expr::Call {
                name: "taskReminderButtonLabel".to_owned(),
                arguments: Vec::new(),
                return_type: Type::String,
                is_async: false,
                is_throwing: false,
                is_constructor: false,
            },
            None,
            None,
            Some(&SystemIcon::Shared("notifications".to_owned())),
            None,
            None,
            Some(nexa_ir::ButtonStyle::Bordered),
            None,
            None,
            None,
            false,
            &[],
            0,
            &mut output,
        );

        assert!(output.contains(
            "Label(nexa_fn_taskReminderButtonLabel(), systemImage: \"bell.fill\").font(.system(size: 17))"
        ));
        assert!(!output.contains("Label { Text(nexa_fn_taskReminderButtonLabel()) }"));
    }

    #[test]
    fn bordered_button_uses_an_outline_and_shared_content_padding() {
        let mut output = SourceWriter::new();
        render_button(
            &Expr::String("Tomorrow".to_owned()),
            None,
            None,
            None,
            None,
            None,
            Some(nexa_ir::ButtonStyle::Bordered),
            None,
            Some(nexa_ir::ButtonShape::Capsule),
            None,
            false,
            &[],
            0,
            &mut output,
        );

        assert!(output.contains("style: .bordered"));
        assert!(output.contains("shape: .capsule"));
    }

    #[test]
    fn large_button_uses_the_shared_large_control_height() {
        let mut output = SourceWriter::new();
        render_button(
            &Expr::String("Enable Notifications".to_owned()),
            None,
            None,
            None,
            None,
            None,
            Some(nexa_ir::ButtonStyle::BorderedProminent),
            Some(nexa_ir::ButtonSize::Large),
            None,
            None,
            false,
            &[],
            0,
            &mut output,
        );

        assert!(output.contains("size: .large"));
        assert!(output.contains("style: .borderedProminent"));
        assert!(output.contains("NexaButtonPrimitive("));
        assert!(output.contains(".font(.system(size: 17))"));
    }

    #[test]
    fn renders_native_progress_indicators_with_bounded_values() {
        let progress = nexa_ir::Expr::State(
            "progress".to_owned(),
            nexa_ir::Type::Numeric(nexa_ir::NumericType::Float64),
        );
        let mut output = SourceWriter::new();
        render_progress_bar(Some(&progress), None, 0, &mut output);
        render_progress_ring(Some(&progress), None, 0, &mut output);
        assert_eq!(
            output.as_str(),
            "NexaProgressBarPrimitive(progress: nexa_progress, tint: nil, trackColor: nil, thickness: nil, cornerRadius: nil)\nNexaProgressRingPrimitive(progress: nexa_progress, tint: nil, trackColor: nil, size: nil, thickness: nil)\n"
        );
    }

    #[test]
    fn omitted_progress_generates_the_native_indeterminate_bar_call() {
        let mut output = SourceWriter::new();
        render_progress_bar(None, None, 0, &mut output);

        assert_eq!(
            output.as_str(),
            "NexaProgressBarPrimitive(progress: nil, tint: nil, trackColor: nil, thickness: nil, cornerRadius: nil)\n"
        );
    }

    #[test]
    fn omitted_ring_progress_generates_the_native_indeterminate_ring_call() {
        let mut output = SourceWriter::new();
        render_progress_ring(None, None, 0, &mut output);

        assert_eq!(
            output.as_str(),
            "NexaProgressRingPrimitive(progress: nil, tint: nil, trackColor: nil, size: nil, thickness: nil)\n"
        );
    }

    #[test]
    fn renders_slider_as_a_native_double_binding() {
        let mut output = SourceWriter::new();
        let min = float_expr("0.0");
        let max = float_expr("1.0");
        render_slider(
            "volume",
            &min,
            &max,
            0.1,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            0,
            &mut output,
        );
        assert_eq!(
            output.as_str(),
            "NexaSliderPrimitive(\n    value: $nexa_volume, minValue: Double(0.0), maxValue: Double(1.0), step: 0.1,\n    tint: nil, trackColor: nil, thumbColor: nil,\n    trackHeight: nil, thumbSize: nil, hitSlop: nil,\n    onChange: \n    nil,\n    onEditingChanged: \n    nil,\n)\n"
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

        assert!(output.contains("let nexaMoveFrom"));
        assert!(output.contains("nexa_items.indices.contains(nexaMoveFrom"));
        assert!(output.contains("nexa_items.remove(at: nexaMoveFrom"));
        assert!(output.contains("nexa_items.insert(nexaMovedValue"));
        assert!(output.contains(", at: nexaMoveTo"));
    }

    #[test]
    fn renders_map_clear_without_replacing_the_state_value() {
        let actions = [Action::CollectionMutation {
            name: "quantities".to_owned(),
            operation: CollectionMutation::MapClear,
            arguments: Vec::new(),
        }];
        let mut output = SourceWriter::new();

        render_actions(&actions, 0, &mut output);

        assert_eq!(
            output.as_str(),
            "nexa_quantities.removeAll(keepingCapacity: true)\n"
        );
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
            "nexa_player.onProgressChanged = { nexa_position, nexa_duration in\n    nexa_latest = nexa_position\n}\n"
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

        assert!(output.contains("NexaNetwork.onStatusChange { nexa_online in"));
        assert!(output.contains("nexa_connected = nexa_online"));
    }

    #[test]
    fn renders_task_launch_and_cancellation_on_the_selected_executor() {
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

        assert!(
            output.contains("nexa_refreshTask?.cancel()\nnexa_refreshTask = Task { @MainActor in")
        );
        assert!(output.contains("nexa_finished = true"));
        assert!(output.contains("nexa_workerTask = Task.detached {"));
        assert!(output.contains("_ = nexaTaskScope.track(Task { @MainActor in"));
        assert!(output.contains("_ = false"));
        assert!(!output.contains("unhandled error escaped a typed Nexa task"));
        assert!(output.ends_with("nexa_refreshTask?.cancel()\n"));
    }

    #[test]
    fn renders_explicit_throwing_call_recovery_without_optional_fallbacks() {
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

        render_function_actions(&actions, 0, &mut output, true, &Type::Void);

        assert!(output.contains("do {\n    (try await CameraPlugin.shared.capture())"));
        assert!(output.contains(
            "} catch is CancellationError {\n    throw CancellationError()\n} catch {\n    nexa_failed = true\n}"
        ));
        assert!(!output.contains("try?"));

        let mut task_output = SourceWriter::new();
        render_task_body(&actions, 0, &mut task_output, false);
        assert!(!task_output.contains("throw CancellationError()"));
        assert!(task_output.contains("} catch is CancellationError {\n    return"));
        assert!(task_output.contains("CameraPlugin.shared.capture()"));
        assert!(!task_output.contains("fatalError(\"unhandled error escaped a typed Nexa task\")"));
    }

    #[test]
    fn nonthrowing_function_returns_a_typed_default_on_cancellation() {
        let actions = [Action::TryCatch {
            body: vec![Action::Expression(Expr::TryAwait(Box::new(
                Expr::NativeCall {
                    receiver: None,
                    namespace: "Network".to_owned(),
                    name: "refresh".to_owned(),
                    arguments: Vec::new(),
                    codecs: Vec::new(),
                    return_type: Type::Void,
                    source_span: None,
                    is_async: true,
                    is_throwing: true,
                },
            )))],
            error_catches: Vec::new(),
            catch_body: Some(vec![Action::Expression(Expr::Bool(false))]),
        }];
        let mut output = SourceWriter::new();

        render_function_actions(&actions, 0, &mut output, false, &Type::Bool);

        assert!(output.contains("} catch is CancellationError {\n    return false\n} catch {"));
        assert!(!output.contains("throw CancellationError()"));
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
                    body: Vec::new(),
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

        render_function_actions(&actions, 0, &mut output, true, &Type::Void);

        assert!(output.contains("} catch let error as PlayerError {"));
        assert!(output.contains("case .invalidUrl:"));
        assert!(output.contains("break\n"));
        assert!(output.contains("case let .decodingFailed(nexa_message):"));
        assert!(output.contains("nexa_loadError = nexa_message"));
        assert!(output.contains("catch is CancellationError"));
        assert!(
            output.contains("fatalError(\"unexpected error escaped its typed plugin contract\")")
        );
    }
}
