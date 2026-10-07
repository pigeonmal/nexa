use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{
    Action, AnimationSpec, CollectionMutation, ErrorCatchArm, Expr, HapticStyle, Module, Node,
    SystemIcon, TaskExecutor,
};

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
    if let Some(loading) = loading {
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
        out.line_at(depth + 1, format_args!("if {} {{", expression(loading)));
        indent(out, depth + 2);
        out.push_str("ProgressView()\n");
        indent(out, depth + 1);
        out.push_str("} else {\n");
        render_button_label(label, comment, icon, depth + 2, out);
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
        render_button_modifiers(style, size, shape, tint, glass, out);
        return;
    }
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
    render_button_label(label, comment, icon, depth + 1, out);
    indent(out, depth);
    out.push('}');
    if let Some(disabled) = disabled {
        out.push_str(&format!(".disabled({})", expression(disabled)));
    }
    render_button_modifiers(style, size, shape, tint, glass, out);
}

fn render_button_modifiers(
    style: Option<nexa_ir::ButtonStyle>,
    size: Option<nexa_ir::ButtonSize>,
    shape: Option<nexa_ir::ButtonShape>,
    tint: Option<&nexa_ir::ColorExpression>,
    glass: bool,
    out: &mut SourceWriter,
) {
    if let Some(style) = style {
        match style {
            nexa_ir::ButtonStyle::BorderedProminent => {
                out.push_str(".buttonStyle(.borderedProminent)")
            }
            nexa_ir::ButtonStyle::Bordered => out.push_str(".buttonStyle(.bordered)"),
            nexa_ir::ButtonStyle::Borderless => out.push_str(".buttonStyle(.borderless)"),
            nexa_ir::ButtonStyle::Plain => out.push_str(".buttonStyle(.plain)"),
        }
    } else {
        // Android's default Button is a filled Material button. Make Swift's
        // implicit style explicit so an omitted style has the same meaning.
        out.push_str(".buttonStyle(.borderedProminent)");
    }
    out.push_str(&format!(
        ".frame(minWidth: {}, minHeight: {})",
        nexa_codegen::design_system::BUTTON_MIN_WIDTH,
        nexa_codegen::design_system::BUTTON_MIN_TAP_TARGET,
    ));
    if let Some(size) = size {
        match size {
            nexa_ir::ButtonSize::Small => out.push_str(".controlSize(.small)"),
            nexa_ir::ButtonSize::Regular => out.push_str(".controlSize(.regular)"),
            nexa_ir::ButtonSize::Large => out.push_str(".controlSize(.large)"),
        }
    }
    if let Some(shape) = shape {
        match shape {
            nexa_ir::ButtonShape::Capsule => out.push_str(".nexaButtonShape(.capsule)"),
            nexa_ir::ButtonShape::Circle => out.push_str(".nexaButtonShape(.circle)"),
            nexa_ir::ButtonShape::Rounded(r) => {
                out.push_str(&format!(".nexaButtonShape(.roundedRectangle({}))", r))
            }
        }
    }
    if let Some(tint) = tint {
        out.push_str(&format!(
            ".tint({})",
            crate::generator::colors::expression_for_color(tint)
        ));
    } else if matches!(
        style,
        Some(nexa_ir::ButtonStyle::Borderless | nexa_ir::ButtonStyle::Plain)
    ) {
        out.push_str(".tint(Color.accentColor)");
    }
    if glass {
        let shape_str = match shape {
            Some(nexa_ir::ButtonShape::Circle) => ".circle",
            Some(nexa_ir::ButtonShape::Rounded(r)) => &format!(".rounded({})", r),
            _ => ".capsule",
        };
        if let Some(tint) = tint {
            out.push_str(&format!(
                ".nexaGlass(tint: {}, shape: {})",
                crate::generator::colors::expression_for_color(tint),
                shape_str
            ));
        } else {
            out.push_str(&format!(".nexaGlass(shape: {})", shape_str));
        }
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
        out.push_str(&format!(
            "Label {{ {} }} icon: {{ Image(systemName: {}) }}.font(.body)\n",
            crate::generator::expressions::localized_text_view(label, comment),
            swift_string(&icon.sf_symbol_name()),
        ));
    } else {
        out.push_str(&format!(
            "{}.font(.body)\n",
            crate::generator::expressions::localized_text_view(label, comment)
        ));
    }
}

pub(crate) fn render_switch(
    state: &str,
    label: &nexa_ir::Expr,
    comment: Option<&str>,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.text_at(
        depth,
        format_args!(
            "Toggle(isOn: ${}) {{ {} }}",
            state_name(state),
            crate::generator::expressions::localized_text_view(label, comment)
        ),
    );
}

pub(crate) fn render_slider(
    state: &str,
    min: f64,
    max: f64,
    step: f64,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(
        depth,
        format_args!(
            "Slider(value: ${}, in: {}...{}, step: {})",
            state_name(state),
            min,
            max,
            step
        ),
    );
}

pub(crate) fn render_progress_bar(progress: &nexa_ir::Expr, depth: usize, out: &mut SourceWriter) {
    out.line_at(
        depth,
        format_args!(
            "ProgressView(value: min(max({}, 0.0), 1.0), total: 1.0)",
            expression(progress)
        ),
    );
}

pub(crate) fn render_progress_ring(progress: &nexa_ir::Expr, depth: usize, out: &mut SourceWriter) {
    out.line_at(
        depth,
        format_args!(
            "ProgressView(value: min(max({}, 0.0), 1.0), total: 1.0).progressViewStyle(.circular)",
            expression(progress)
        ),
    );
}

pub(crate) fn render_segmented_control(
    items: &nexa_ir::Expr,
    state: &str,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(
        depth,
        format_args!("Picker(\"\", selection: ${}) {{", state_name(state)),
    );
    out.line_at(
        depth + 1,
        format_args!("ForEach({}, id: \\.self) {{ item in", expression(items)),
    );
    out.line_at(depth + 2, format_args!("Text(item).tag(item)"));
    out.line_at(depth + 1, format_args!("}}"));
    indent(out, depth);
    out.push_str("}.pickerStyle(.segmented).labelsHidden()");
}

pub(crate) fn render_picker(
    items: &nexa_ir::Expr,
    state: &str,
    icon: Option<&SystemIcon>,
    label: Option<&nexa_ir::Expr>,
    comment: Option<&str>,
    depth: usize,
    out: &mut SourceWriter,
) {
    if let Some(label) = label {
        let selected = state_name(state);
        out.line_at(depth, format_args!("Picker(selection: ${selected}) {{"));
        out.line_at(
            depth + 1,
            format_args!("ForEach({}, id: \\.self) {{ item in", expression(items)),
        );
        out.line_at(depth + 2, format_args!("Text(item).tag(item)"));
        out.line_at(depth + 1, format_args!("}}"));
        if let Some(icon) = icon {
            out.line_at(depth, format_args!("}} label: {{ Label {{"));
            out.line_at(
                depth + 1,
                format_args!(
                    "{}",
                    crate::generator::expressions::localized_text_view(label, comment)
                ),
            );
            out.line_at(
                depth,
                format_args!(
                    "}} icon: {{ Image(systemName: {}) }} }}",
                    swift_string(&icon.sf_symbol_name())
                ),
            );
        } else {
            out.line_at(
                depth,
                format_args!(
                    "}} label: {{ {} }}",
                    crate::generator::expressions::localized_text_view(label, comment)
                ),
            );
        }
        return;
    }
    if let Some(icon) = icon {
        let selected = state_name(state);
        out.line_at(depth, format_args!("Menu {{"));
        out.line_at(
            depth + 1,
            format_args!("ForEach({}, id: \\.self) {{ item in", expression(items)),
        );
        out.line_at(
            depth + 2,
            format_args!("Button {{ {selected} = item }} label: {{"),
        );
        out.line_at(depth + 3, format_args!("if item == {selected} {{"));
        out.line_at(
            depth + 4,
            format_args!("Label(item, systemImage: \"checkmark\")"),
        );
        out.line_at(depth + 3, format_args!("}} else {{"));
        out.line_at(depth + 4, format_args!("Text(item)"));
        out.line_at(depth + 3, format_args!("}}"));
        out.line_at(depth + 2, format_args!("}}"));
        out.line_at(depth + 1, format_args!("}}"));
        out.line_at(
            depth,
            format_args!(
                "}} label: {{ Image(systemName: {}) }}",
                swift_string(&icon.sf_symbol_name())
            ),
        );
        out.push_str(&format!(
            ".accessibilityLabel(\"Choose an option\").accessibilityValue({selected})"
        ));
        return;
    }
    out.line_at(
        depth,
        format_args!("Picker(\"\", selection: ${}) {{", state_name(state)),
    );
    out.line_at(
        depth + 1,
        format_args!("ForEach({}, id: \\.self) {{ item in", expression(items)),
    );
    out.line_at(depth + 2, format_args!("Text(item).tag(item)"));
    out.line_at(depth + 1, format_args!("}}"));
    indent(out, depth);
    out.push_str("}.pickerStyle(.menu).labelsHidden()");
}

pub(crate) fn render_date_picker(
    timestamp_state: &str,
    has_time_state: &str,
    depth: usize,
    out: &mut SourceWriter,
) {
    let timestamp = state_name(timestamp_state);
    let has_time = state_name(has_time_state);
    out.line_at(depth, format_args!("VStack(spacing: 8) {{"));
    out.line_at(
        depth + 1,
        format_args!("Toggle(\"Time\", isOn: ${has_time})"),
    );
    out.line_at(
        depth + 1,
        format_args!("DatePicker(\"Select Date\", selection: Binding("),
    );
    out.line_at(
        depth + 2,
        format_args!("get: {{ Date(timeIntervalSince1970: TimeInterval({timestamp}) / 1000) }},"),
    );
    out.line_at(
        depth + 2,
        format_args!("set: {{ {timestamp} = Int64(($0.timeIntervalSince1970 * 1000).rounded()) }}"),
    );
    out.line_at(
        depth + 1,
        format_args!("), displayedComponents: {has_time} ? [.date, .hourAndMinute] : [.date])"),
    );
    out.line_at(depth + 2, format_args!(".datePickerStyle(.graphical)"));
    out.line_at(depth + 1, format_args!("}}"));
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
    out.push_str(if double_tap_actions.is_empty() {
        "Button(action: {"
    } else {
        "Button(action: {}) {"
    });
    if !double_tap_actions.is_empty() {
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
        out.push_str(".gesture(\n");
        indent(out, depth + 1);
        out.push_str("TapGesture(count: 2)\n");
        indent(out, depth + 2);
        out.push_str(".onEnded {\n");
        if let Some(haptic) = haptic {
            render_haptic(*haptic, depth + 3, out);
        }
        render_actions(double_tap_actions, depth + 3, out);
        indent(out, depth + 2);
        out.push_str("}\n");
        indent(out, depth + 2);
        out.push_str(".exclusively(before: TapGesture(count: 1).onEnded {\n");
        if let Some(haptic) = haptic {
            render_haptic(*haptic, depth + 3, out);
        }
        render_actions(actions, depth + 3, out);
        indent(out, depth + 2);
        out.push_str("})\n");
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
        if !matches!(disabled, nexa_ir::Expr::Bool(true)) && !long_press_actions.is_empty() {
            out.push_str(".onLongPressGesture(minimumDuration: ");
            out.push_str(&long_press_duration_seconds(long_press_duration_ms));
            out.push_str(") {");
            out.push('\n');
            if let Some(haptic) = haptic {
                render_haptic(*haptic, depth + 1, out);
            }
            render_actions(long_press_actions, depth + 1, out);
            indent(out, depth);
            out.push('}');
        }
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
    if !matches!(disabled, nexa_ir::Expr::Bool(true)) && !long_press_actions.is_empty() {
        out.push_str(".onLongPressGesture(minimumDuration: ");
        out.push_str(&long_press_duration_seconds(long_press_duration_ms));
        out.push_str(") {");
        out.push('\n');
        if let Some(haptic) = haptic {
            render_haptic(*haptic, depth + 1, out);
        }
        render_actions(long_press_actions, depth + 1, out);
        indent(out, depth);
        out.push('}');
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
    for action in actions {
        match action {
            Action::Expression(value) => {
                out.line_at(depth, format_args!("{}", expression(value)));
            }
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
                render_actions(actions, depth + 1, out);
                indent(out, depth);
                out.push_str("}\n");
            }
            Action::NetworkStatusSubscribe { parameter, actions } => {
                out.text_at(depth, format_args!("NexaNetwork.onStatusChange {{ "));
                out.push_str(&state_name(parameter));
                out.push_str(" in\n");
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
                    indent(out, depth);
                    out.push_str(&format!("{handle} = "));
                } else {
                    indent(out, depth);
                }
                match executor {
                    TaskExecutor::Main => out.push_str("Task { @MainActor in\n"),
                    TaskExecutor::Background => out.push_str("Task.detached {\n"),
                }
                render_actions(actions, depth + 1, out);
                indent(out, depth);
                out.push_str("}\n");
            }
            Action::TaskCancel { handle } => {
                out.line_at(depth, format_args!("{}?.cancel()", state_name(handle)));
            }
            Action::WithAnimation {
                animation,
                animated_states,
                actions,
            } => {
                if animated_states.is_empty() {
                    render_actions(actions, depth, out);
                    continue;
                }
                indent(out, depth);
                out.push_str("withAnimation(");
                out.push_str(&swift_animation(*animation));
                out.push_str(") {\n");
                render_actions(actions, depth + 1, out);
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
                    format_args!("for {} in {} {{", state_name(name), expression(iterable)),
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
                        "for ({}, {}) in {} {{",
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
                out.line_at(depth, format_args!("while {} {{", expression(condition)));
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
                out.push_str("do {\n");
                render_actions(body, depth + 1, out);
                render_swift_error_catches(error_catches, catch_body.as_deref(), depth, out);
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

fn swift_animation(animation: AnimationSpec) -> String {
    match animation {
        AnimationSpec::Spring { response, damping } => {
            format!(".spring(response: {response}, dampingFraction: {damping})")
        }
        AnimationSpec::EaseIn => ".easeIn".to_owned(),
        AnimationSpec::EaseOut => ".easeOut".to_owned(),
        AnimationSpec::EaseInOut => ".easeInOut".to_owned(),
        AnimationSpec::Linear => ".linear".to_owned(),
    }
}

fn render_swift_error_catches(
    catches: &[ErrorCatchArm],
    catch_all: Option<&[Action]>,
    depth: usize,
    out: &mut SourceWriter,
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
            render_actions(&arm.body, depth + 3, out);
            // Swift does not implicitly terminate an enum switch case. Emit an
            // explicit break even when the handler has actions so generated
            // cases are always complete and never rely on fallthrough rules.
            indent(out, depth + 3);
            out.push_str("break\n");
        }
        if let Some(catch_all) = catch_all {
            indent(out, depth + 2);
            out.push_str("default:\n");
            render_actions(catch_all, depth + 3, out);
        }
        indent(out, depth + 1);
        out.push_str("}\n");
    }

    if catches.is_empty() || catch_all.is_some() {
        indent(out, depth);
        out.push_str("} catch {\n");
        if let Some(catch_all) = catch_all {
            render_actions(catch_all, depth + 1, out);
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
    use nexa_ir::{Action, CollectionMutation, Expr, NumericType, TaskExecutor, Type};

    use super::{render_actions, render_progress_bar, render_progress_ring, render_slider};

    #[test]
    fn renders_native_progress_indicators_with_bounded_values() {
        let progress = nexa_ir::Expr::State(
            "progress".to_owned(),
            nexa_ir::Type::Numeric(nexa_ir::NumericType::Float64),
        );
        let mut output = SourceWriter::new();
        render_progress_bar(&progress, 0, &mut output);
        render_progress_ring(&progress, 0, &mut output);
        assert_eq!(
            output.as_str(),
            "ProgressView(value: min(max(nexa_progress, 0.0), 1.0), total: 1.0)\nProgressView(value: min(max(nexa_progress, 0.0), 1.0), total: 1.0).progressViewStyle(.circular)\n"
        );
    }

    #[test]
    fn renders_slider_as_a_native_double_binding() {
        let mut output = SourceWriter::new();
        render_slider("volume", 0.0, 1.0, 0.1, 0, &mut output);
        assert_eq!(
            output.as_str(),
            "Slider(value: $nexa_volume, in: 0...1, step: 0.1)\n"
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
        assert!(output.contains("Task { @MainActor in\n    false\n}"));
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

        render_actions(&actions, 0, &mut output);

        assert!(output.contains("do {\n    (try await CameraPlugin.shared.capture())"));
        assert!(output.contains("} catch {\n    nexa_failed = true\n}"));
        assert!(!output.contains("try?"));
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

        render_actions(&actions, 0, &mut output);

        assert!(output.contains("} catch let error as PlayerError {"));
        assert!(output.contains("case .invalidUrl:"));
        assert!(output.contains("break\n"));
        assert!(output.contains("case let .decodingFailed(nexa_message):"));
        assert!(output.contains("nexa_loadError = nexa_message"));
        assert!(
            output.contains("fatalError(\"unexpected error escaped its typed plugin contract\")")
        );
    }
}
