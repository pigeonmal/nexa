use nexa_codegen::names::state_name;
use nexa_ir::{Action, Expr, Module, Node};

use super::{
    components::render_children,
    expressions::expression,
    features::Features,
    utils::{indent, kotlin_string},
};

pub(super) fn render_button(label: &Expr, actions: &[Action], depth: usize, out: &mut String) {
    indent(out, depth);
    out.push_str("Button(onClick = {");
    if actions.is_empty() {
        out.push_str(" }) {\n");
    } else {
        out.push('\n');
        render_actions(actions, depth + 1, out);
        indent(out, depth);
        out.push_str("}) {\n");
    }
    indent(out, depth + 1);
    out.push_str(&format!("Text({})\n", expression(label)));
    indent(out, depth);
    out.push('}');
}

pub(super) fn render_switch(state: &str, label: &str, depth: usize, out: &mut String) {
    indent(out, depth);
    out.push_str(&format!(
        "Switch(checked = {}, onCheckedChange = {{ {} = it }}, modifier = Modifier.semantics {{ contentDescription = {} }})",
        state_name(state),
        state_name(state),
        kotlin_string(label)
    ));
}

pub(super) fn render_pressable(
    disabled: bool,
    children: &[Node],
    actions: &[Action],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str(&format!(
        "Box(\n{}    modifier = Modifier.clickable(\n{}        enabled = {},\n{}        role = Role.Button,\n{}        onClick = {{",
        "    ".repeat(depth),
        "    ".repeat(depth),
        !disabled,
        "    ".repeat(depth),
        "    ".repeat(depth)
    ));
    if actions.is_empty() {
        out.push_str(" }),\n");
    } else {
        out.push('\n');
        render_actions(actions, depth + 3, out);
        indent(out, depth + 2);
        out.push_str("}),\n");
    }
    indent(out, depth + 1);
    out.push_str(") {\n");
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
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
        }
    }
}
