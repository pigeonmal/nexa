use nexa_codegen::SourceWriter;
use nexa_ir::{AccessibilityRole, Expr, Module, Node};

use crate::generator::{
    components::render_children, expressions::expression, features::Features, utils::indent,
};

use crate::generator::engine::imports::ImportSet;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_pressable || features.uses_accessibility_role,
        "androidx.compose.ui.semantics.Role",
    );
    imports.add(
        features.uses_accessibility_role,
        "androidx.compose.ui.semantics.role",
    );
    imports.add(
        features.uses_accessibility_heading,
        "androidx.compose.ui.semantics.heading",
    );
    imports.add(
        features.uses_accessibility_hint,
        "androidx.compose.ui.semantics.stateDescription",
    );
    imports.add(
        features.uses_switch || features.uses_accessibility,
        "androidx.compose.ui.semantics.contentDescription",
    );
    imports.add(
        features.uses_switch || features.uses_accessibility,
        "androidx.compose.ui.semantics.semantics",
    );
}

pub(crate) fn render_accessibility(
    label: &Expr,
    hint: Option<&Expr>,
    role: AccessibilityRole,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    indent(out, depth);
    out.push_str("Box(\n");
    indent(out, depth + 1);
    out.push_str("modifier = Modifier.semantics(mergeDescendants = true) {\n");
    let child_has_same_image_label = matches!((label, children),
        (Expr::String(label), [Node::Image { description, .. }]) if description == label
    );
    if !child_has_same_image_label {
        out.line_at(
            depth + 2,
            format_args!("contentDescription = {}", expression(label)),
        );
    }
    if let Some(hint) = hint {
        out.line_at(
            depth + 2,
            format_args!("stateDescription = {}", expression(hint)),
        );
    }
    if matches!(role, AccessibilityRole::Header) {
        indent(out, depth + 2);
        out.push_str("heading()\n");
    } else if let Some(role_name) = role_name(role) {
        out.line_at(depth + 2, format_args!("role = Role.{role_name}"));
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
