use nexa_codegen::SourceWriter;
use nexa_ir::{AccessibilityRole, Expr, Node};

use crate::generator::{
    components::render_children, expressions::expression, features::Features, utils::indent,
};

use crate::generator::engine::imports::ImportSet;

use super::RenderScope;

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
        features.uses_switch || features.uses_date_picker || features.uses_accessibility,
        "androidx.compose.ui.semantics.contentDescription",
    );
    imports.add(
        features.uses_switch || features.uses_date_picker || features.uses_accessibility,
        "androidx.compose.ui.semantics.semantics",
    );
    imports.add(
        features.uses_accessibility,
        "androidx.compose.ui.semantics.stateDescription",
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_accessibility(
    label: &Expr,
    hint: Option<&Expr>,
    value: Option<&Expr>,
    role: AccessibilityRole,
    children: &[Node],
    scope: &RenderScope<'_>,
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
            format_args!(
                "contentDescription = {}",
                accessibility_description(label, hint)
            ),
        );
    } else if let Some(hint) = hint {
        out.line_at(
            depth + 2,
            format_args!("contentDescription = {}", expression(hint)),
        );
    }
    if let Some(value) = value {
        out.line_at(
            depth + 2,
            format_args!("stateDescription = {}", expression(value)),
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
    render_children(children, scope.module, scope.features, depth + 1, out);
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

/// Compose has no stable standalone accessibility-hint semantic in Nexa's
/// pinned BOM. Keep the hint attached to the accessible description instead
/// of misrepresenting it as the control's current state.
fn accessibility_description(label: &Expr, hint: Option<&Expr>) -> String {
    match (label, hint) {
        (Expr::String(label), Some(Expr::String(hint))) => {
            crate::generator::utils::kotlin_string(&format!("{label}, {hint}"))
        }
        (label, Some(hint)) => {
            format!("{} + \", \" + {}", expression(label), expression(hint))
        }
        (label, None) => expression(label),
    }
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
