use nexa_codegen::names::state_name;
use nexa_ir::{Action, Expr, Module, Node};

use super::{
    components::render_node,
    expressions::expression,
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
    for (index, child) in children.iter().enumerate() {
        render_node(child, module, depth + 1, out);
        if index + 1 < children.len() {
            out.push('\n');
        }
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

fn render_actions(actions: &[Action], depth: usize, out: &mut String) {
    for action in actions {
        match action {
            Action::Assign { name, value } => {
                indent(out, depth);
                out.push_str(&format!("{} = {}\n", state_name(name), expression(value)));
            }
        }
    }
}
