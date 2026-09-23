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
    imports.add(features.uses_button, "androidx.compose.material3.Text");
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
    out: &mut String,
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
        indent(out, depth + 1);
        out.push_str(&format!("if ({}) {{\n", expression(loading)));
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

fn render_button_content(label: &Expr, icon: Option<&str>, depth: usize, out: &mut String) {
    if let Some(icon) = icon {
        indent(out, depth);
        out.push_str(&format!(
            "Icon(painter = nexaDrawablePainter({}), contentDescription = null)\n",
            kotlin_string(icon)
        ));
    }
    indent(out, depth);
    out.push_str(&format!("Text({})\n", expression(label)));
}

pub(crate) fn render_switch(state: &str, label: &str, depth: usize, out: &mut String) {
    indent(out, depth);
    out.push_str(&format!(
        "Switch(checked = {}, onCheckedChange = {{ {} = it }}, modifier = Modifier.semantics {{ contentDescription = {} }})",
        state_name(state),
        state_name(state),
        kotlin_string(label)
    ));
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
    out: &mut String,
) {
    indent(out, depth);
    let modifier = if long_press_actions.is_empty() {
        "clickable"
    } else {
        "combinedClickable"
    };
    let has_long_press = !long_press_actions.is_empty();
    out.push_str(&format!(
        "Box(\n{}    modifier = Modifier.{}(\n{}        enabled = {},\n{}        role = Role.Button,\n{}        onClick = {{",
        "    ".repeat(depth),
        modifier,
        "    ".repeat(depth),
        format!("!({})", expression(disabled)),
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

fn render_haptic(style: HapticStyle, depth: usize, out: &mut String) {
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
    let mut out = String::from("{");
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
    out
}

pub(crate) fn render_actions(actions: &[Action], depth: usize, out: &mut String) {
    for action in actions {
        match action {
            Action::Expression(value) => {
                indent(out, depth);
                out.push_str(&format!("{}\n", expression(value)));
            }
            Action::Assign { name, value } => {
                indent(out, depth);
                out.push_str(&format!("{} = {}\n", state_name(name), expression(value)));
            }
            Action::NativePropertyAssign {
                receiver,
                property,
                value,
            } => {
                indent(out, depth);
                out.push_str(&format!(
                    "{}.{} = {}\n",
                    expression(receiver),
                    property,
                    expression(value)
                ));
            }
            Action::NativeEventSubscribe {
                receiver,
                property,
                parameters,
                actions,
            } => {
                indent(out, depth);
                out.push_str(&format!("{}.{} = {{", expression(receiver), property));
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
                }
            }
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                indent(out, depth);
                out.push_str(&format!("if ({}) {{\n", expression(condition)));
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
                indent(out, depth);
                out.push_str(&format!(
                    "for ({} in {}) {{\n",
                    state_name(name),
                    expression(iterable)
                ));
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
                indent(out, depth);
                out.push_str(&format!(
                    "for (({}, {}) in {}) {{\n",
                    state_name(key_name),
                    state_name(value_name),
                    expression(iterable)
                ));
                render_actions(body, depth + 1, out);
                indent(out, depth);
                out.push_str("}\n");
            }
            Action::While { condition, body } => {
                indent(out, depth);
                out.push_str(&format!("while ({}) {{\n", expression(condition)));
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
    out: &mut String,
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
                indent(out, depth + 3);
                out.push_str(&format!(
                    "val {} = error.{}\n",
                    state_name(binding),
                    property
                ));
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
    use nexa_ir::{Action, Expr, NumericType, Type};

    use super::render_actions;

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
        let mut output = String::new();

        render_actions(&actions, 0, &mut output);

        assert_eq!(
            output,
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
        let mut output = String::new();

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
        let mut output = String::new();

        render_actions(&actions, 0, &mut output);

        assert!(output.contains("catch (error: Exception)"));
        assert!(output.contains("PlayerError.invalidUrl -> {"));
        assert!(output.contains("is PlayerError.decodingFailed -> {"));
        assert!(output.contains("val nexa_message = error.message"));
        assert!(output.contains("nexa_loadError = nexa_message"));
        assert!(output.contains("else -> {\n            throw error"));
    }
}
