use nexa_codegen::names::state_name;
use nexa_ir::{Action, CollectionMutation, HapticStyle, Module, Node};

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
    render_children(children, module, depth + 1, out);
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

pub(crate) fn render_actions(actions: &[Action], depth: usize, out: &mut String) {
    for action in actions {
        match action {
            Action::Assign { name, value } => {
                indent(out, depth);
                out.push_str(&format!("{} = {}\n", state_name(name), expression(value)));
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
