use nexa_ir::{AccessibilityRole, Module, Node};

use super::{
    components::render_children,
    features::Features,
    utils::{indent, kotlin_string},
};

pub(super) fn render_accessibility(
    label: &str,
    role: AccessibilityRole,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    indent(out, depth);
    out.push_str("Box(\n");
    indent(out, depth + 1);
    out.push_str("modifier = Modifier.semantics(mergeDescendants = true) {\n");
    let child_has_same_image_label = matches!(
        children,
        [Node::Image { description, .. }] if description == label
    );
    if !child_has_same_image_label {
        indent(out, depth + 2);
        out.push_str(&format!("contentDescription = {}\n", kotlin_string(label)));
    }
    if matches!(role, AccessibilityRole::Header) {
        indent(out, depth + 2);
        out.push_str("heading()\n");
    } else if let Some(role_name) = role_name(role) {
        indent(out, depth + 2);
        out.push_str(&format!("role = Role.{role_name}\n"));
    }
    indent(out, depth + 1);
    out.push_str("},\n");
    indent(out, depth);
    out.push_str(") {\n");
    render_children(children, module, features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

fn role_name(role: AccessibilityRole) -> Option<&'static str> {
    match role {
        AccessibilityRole::None => None,
        AccessibilityRole::Button => Some("Button"),
        AccessibilityRole::Link => None,
        AccessibilityRole::Header => None,
        AccessibilityRole::Image => Some("Image"),
    }
}
