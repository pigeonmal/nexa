use nexa_ir::{AccessibilityRole, Module, Node};

use super::{
    components::render_children,
    utils::{indent, swift_string},
};

pub(super) fn render_accessibility(
    label: &str,
    role: AccessibilityRole,
    children: &[Node],
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    render_children(children, module, depth, out);
    let child_has_same_image_label = matches!(
        children,
        [Node::Image { description, .. }] if description == label
    );
    if children.len() > 1 {
        out.push('\n');
        indent(out, depth + 1);
        out.push_str(".accessibilityElement(children: .combine)");
    }
    if !child_has_same_image_label {
        out.push('\n');
        indent(out, depth + 1);
        out.push_str(&format!(".accessibilityLabel({})", swift_string(label)));
    }
    if let Some(trait_name) = trait_name(role) {
        out.push('\n');
        indent(out, depth + 1);
        out.push_str(&format!(".accessibilityAddTraits({trait_name})"));
    }
}

fn trait_name(role: AccessibilityRole) -> Option<&'static str> {
    match role {
        AccessibilityRole::None => None,
        AccessibilityRole::Button => Some(".isButton"),
        AccessibilityRole::Link => Some(".isLink"),
        AccessibilityRole::Header => Some(".isHeader"),
        AccessibilityRole::Image => Some(".isImage"),
    }
}
