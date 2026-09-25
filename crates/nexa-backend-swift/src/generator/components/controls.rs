use nexa_codegen::names::state_name;
use nexa_ir::{Action, CollectionMutation, ErrorCatchArm, HapticStyle, Module, Node};

use crate::generator::{
    components::render_children,
    expressions::expression,
    utils::{indent, swift_string},
};

use crate::generator::engine::features::Features;
use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(features.uses_haptic, "UIKit");
}

pub(crate) fn render_button(
    label: &nexa_ir::Expr,
    icon: Option<&str>,
    loading: Option<&nexa_ir::Expr>,
    disabled: Option<&nexa_ir::Expr>,
    actions: &[Action],
    depth: usize,
    out: &mut String,
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
        indent(out, depth + 1);
        out.push_str(&format!("if {} {{\n", expression(loading)));
        indent(out, depth + 2);
        out.push_str("ProgressView()\n");
        indent(out, depth + 1);
        out.push_str("} else {\n");
        render_button_label(label, icon, depth + 2, out);
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
        return;
    }
    indent(out, depth);
    if icon.is_none() {
        out.push_str(&format!("Button({}) {{", expression(label)));
        if actions.is_empty() {
            out.push_str(" }");
        } else {
            out.push('\n');
            render_actions(actions, depth + 1, out);
            indent(out, depth);
            out.push('}');
        }
    } else {
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
        render_button_label(label, icon, depth + 1, out);
        out.push('\n');
        indent(out, depth);
        out.push('}');
    }
    if let Some(disabled) = disabled {
        out.push_str(&format!(".disabled({})", expression(disabled)));
    }
}

fn render_button_label(label: &nexa_ir::Expr, icon: Option<&str>, depth: usize, out: &mut String) {
    indent(out, depth);
    if let Some(icon) = icon {
        out.push_str(&format!(
            "Label({}, systemImage: {})\n",
            expression(label),
            swift_string(icon)
        ));
    } else {
        out.push_str(&format!("Text({})\n", expression(label)));
    }
}

pub(crate) fn render_switch(state: &str, label: &str, depth: usize, out: &mut String) {
    indent(out, depth);
    out.push_str(&format!(
        "Toggle({}, isOn: ${})",
        swift_string(label),
        state_name(state)
    ));
}

pub(crate) fn render_pressable(
    disabled: &nexa_ir::Expr,
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
    out.push_str("Button(action: {");
    if actions.is_empty() {
        if let Some(haptic) = haptic {
            out.push('\n');
            render_haptic(haptic, depth + 1, out);
            indent(out, depth);
            out.push_str("}) {");
        } else {
            out.push_str(" }) {");
        }
    } else {
        out.push('\n');
        if let Some(haptic) = haptic {
            render_haptic(haptic, depth + 1, out);
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
    if !matches!(disabled, nexa_ir::Expr::Bool(true)) && !long_press_actions.is_empty() {
        out.push_str(".onLongPressGesture {");
        out.push('\n');
        if let Some(haptic) = haptic {
            render_haptic(haptic, depth + 1, out);
        }
        render_actions(long_press_actions, depth + 1, out);
        indent(out, depth);
        out.push('}');
    }
}

fn render_haptic(style: HapticStyle, depth: usize, out: &mut String) {
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
                    out.push_str(" in");
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
                        out.push_str(&format!("{state}.append({})\n", rendered[0]));
                    }
                    CollectionMutation::ArrayRemoveAt => {
                        out.push_str(&format!("{state}.remove(at: Int({}))\n", rendered[0]));
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
                }
            }
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                indent(out, depth);
                out.push_str(&format!("if {} {{\n", expression(condition)));
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
                    "for {} in {} {{\n",
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
                    "for ({}, {}) in {} {{\n",
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
                out.push_str(&format!("while {} {{\n", expression(condition)));
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

fn render_swift_error_catches(
    catches: &[ErrorCatchArm],
    catch_all: Option<&[Action]>,
    depth: usize,
    out: &mut String,
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
        indent(out, depth);
        out.push_str(&format!("}} catch let error as {error_type} {{\n"));
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
            "nexa_player.onProgressChanged = { nexa_position, nexa_duration in\n    nexa_latest = nexa_position\n}\n"
        );
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

        assert!(output.contains("do {\n    try await CameraPlugin.shared.capture()"));
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

        assert!(output.contains("} catch let error as PlayerError {"));
        assert!(output.contains("case .invalidUrl:"));
        assert!(output.contains("case let .decodingFailed(nexa_message):"));
        assert!(output.contains("nexa_loadError = nexa_message"));
        assert!(
            output.contains("fatalError(\"unexpected error escaped its typed plugin contract\")")
        );
    }
}
