use nexa_codegen::names::state_name;
use nexa_ir::{Action, CollectionMutation, Expr, Module, Node};

use super::{
    components::render_children,
    expressions::expression,
    features::Features,
    utils::{indent, kotlin_string},
};

pub(super) fn render_button(
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
    disabled: &Expr,
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
    if actions.is_empty() {
        out.push_str(if has_long_press { " },\n" } else { " }),\n" });
    } else {
        out.push('\n');
        render_actions(actions, depth + 3, out);
        indent(out, depth + 2);
        out.push_str(if has_long_press { "},\n" } else { "}),\n" });
    }
    if has_long_press {
        indent(out, depth + 2);
        out.push_str("onLongClick = {\n");
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

pub(super) fn render_actions(actions: &[Action], depth: usize, out: &mut String) {
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
