use nexa_codegen::names::state_name;
use nexa_ir::{Action, Module, Node};

use super::{
    components::render_children,
    expressions::expression,
    utils::{indent, swift_string},
};

pub(super) fn render_button(
    label: &nexa_ir::Expr,
    actions: &[Action],
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str(&format!("Button({}) {{", expression(label)));
    if actions.is_empty() {
        out.push_str(" }");
        return;
    }
    out.push('\n');
    render_actions(actions, depth + 1, out);
    indent(out, depth);
    out.push('}');
}

pub(super) fn render_switch(state: &str, label: &str, depth: usize, out: &mut String) {
    indent(out, depth);
    out.push_str(&format!(
        "Toggle({}, isOn: ${})",
        swift_string(label),
        state_name(state)
    ));
}

pub(super) fn render_pressable(
    disabled: bool,
    children: &[Node],
    actions: &[Action],
    module: &Module,
    depth: usize,
    out: &mut String,
) {
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
    render_children(children, module, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
    if disabled {
        out.push_str(".disabled(true)");
    }
    out.push_str(".buttonStyle(.plain)");
}

pub(super) fn render_actions(actions: &[Action], depth: usize, out: &mut String) {
    for action in actions {
        match action {
            Action::Assign { name, value } => {
                indent(out, depth);
                out.push_str(&format!("{} = {}\n", state_name(name), expression(value)));
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
        }
    }
}
