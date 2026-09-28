use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Action, CollectionMutation, ErrorCatchArm, Expr, HapticStyle, Module, Node};

use crate::generator::{
    components::render_children,
    expressions::expression,
    features::Features,
    utils::{indent, kotlin_string},
};

use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.uses_haptic, "android.view.HapticFeedbackConstants");
    imports.add(
        features.uses_clickable || features.uses_link,
        "androidx.compose.foundation.clickable",
    );
    imports.add(
        features.uses_long_press,
        "androidx.compose.foundation.combinedClickable",
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
        features.uses_progress_ring,
        "androidx.compose.material3.CircularProgressIndicator",
    );
    imports.add(
        features.uses_button || features.uses_segmented_control || features.uses_picker,
        "androidx.compose.material3.Text",
    );
    imports.add(
        features.uses_tab_icon || features.uses_button_icon,
        "androidx.compose.material3.Icon",
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

pub(crate) fn render_button(
    label: &Expr,
    icon: Option<&str>,
    loading: Option<&Expr>,
    disabled: Option<&Expr>,
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
    out.push_str(") {\n");
    render_button_content(label, icon, depth + 1, out);
    indent(out, depth);
    out.push('}');
}

fn render_button_content(label: &Expr, icon: Option<&str>, depth: usize, out: &mut SourceWriter) {
    if let Some(icon) = icon {
        out.line_at(
            depth,
            format_args!(
                "Icon(painter = nexaDrawablePainter({}), contentDescription = null)",
                kotlin_string(icon)
            ),
        );
    }
    out.line_at(depth, format_args!("Text({})", expression(label)));
}

pub(crate) fn render_switch(state: &str, label: &str, depth: usize, out: &mut SourceWriter) {
    out.text_at(depth, format_args!("Switch(checked = {}, onCheckedChange = {{ {} = it }}, modifier = Modifier.semantics {{ contentDescription = {} }})",
        state_name(state),
        state_name(state),
        kotlin_string(label)
    ));
}

pub(crate) fn render_slider(
    state: &str,
    min: f64,
    max: f64,
    step: f64,
    depth: usize,
    out: &mut SourceWriter,
) {
    let intervals = ((max - min) / step).round() as i32;
    out.line_at(
        depth,
        format_args!(
            "Slider(value = {}.toFloat(), onValueChange = {{ {} = it.toDouble() }}, valueRange = {}f..{}f, steps = {})",
            state_name(state),
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

pub(crate) fn render_picker(items: &Expr, state: &str, depth: usize, out: &mut SourceWriter) {
    let selected = state_name(state);
    out.line_at(
        depth,
        format_args!(
            "nexaPickerMenu({}, {selected}, {{ {selected} = it }})",
            expression(items)
        ),
    );
}

pub(crate) fn render_picker_helper(out: &mut SourceWriter) {
    out.push_str(
        "@Composable\ninternal fun nexaPickerMenu(\n    items: List<String>,\n    selected: String,\n    onSelectionChanged: (String) -> Unit,\n) {\n    val expanded = remember { mutableStateOf(false) }\n    Box {\n        TextButton(onClick = { expanded.value = true }) {\n            Text(selected)\n        }\n        DropdownMenu(\n            expanded = expanded.value,\n            onDismissRequest = { expanded.value = false },\n        ) {\n            items.forEach { item ->\n                DropdownMenuItem(\n                    text = { Text(item) },\n                    onClick = {\n                        onSelectionChanged(item)\n                        expanded.value = false\n                    },\n                )\n            }\n        }\n    }\n}\n\n",
    );
}

pub(crate) fn render_pressable(
    disabled: &Expr,
    haptic: Option<HapticStyle>,
    children: &[Node],
    actions: &[Action],
    long_press_actions: &[Action],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    let modifier = if long_press_actions.is_empty() {
        "clickable"
    } else {
        "combinedClickable"
    };
    let has_long_press = !long_press_actions.is_empty();
    let enabled = format!("!({})", expression(disabled));
    out.push_str(&format!(
        "Box(\n{}    modifier = Modifier.{}(\n{}        enabled = {enabled},\n{}        role = Role.Button,\n{}        onClick = {{",
        "    ".repeat(depth),
        modifier,
        "    ".repeat(depth),
        "    ".repeat(depth),
        "    ".repeat(depth)
    ));
    if actions.is_empty() && haptic.is_none() {
        out.push_str(if has_long_press { " },\n" } else { " }),\n" });
    } else {
        out.push('\n');
        if let Some(haptic) = haptic {
            render_haptic(haptic, depth + 3, out);
        }
        render_actions(actions, depth + 3, out);
        indent(out, depth + 2);
        out.push_str(if has_long_press { "},\n" } else { "}),\n" });
    }
    if has_long_press {
        indent(out, depth + 2);
        out.push_str("onLongClick = {\n");
        if let Some(haptic) = haptic {
            render_haptic(haptic, depth + 3, out);
        }
        render_actions(long_press_actions, depth + 3, out);
        indent(out, depth + 2);
        out.push_str("},\n");
        indent(out, depth + 1);
        out.push_str("),\n");
    }
    indent(out, depth);
    out.push_str(") {\n");
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
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
    use nexa_ir::{Action, Expr, NumericType, Type};

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
        render_slider("volume", 0.0, 1.0, 0.1, 0, &mut output);
        assert_eq!(
            output.as_str(),
            "Slider(value = nexa_volume.toFloat(), onValueChange = { nexa_volume = it.toDouble() }, valueRange = 0f..1f, steps = 9)\n"
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
            "nexa_player.onProgressChanged = { nexa_position, nexa_duration ->\n    nexa_latest = nexa_position\n}\n"
        );
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
