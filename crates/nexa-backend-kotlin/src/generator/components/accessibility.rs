use nexa_codegen::SourceWriter;
use nexa_ir::{AccessibilityRole, Expr, Node};

use crate::generator::engine::imports::ImportSet;
use crate::generator::{components::render_children, expressions::expression, features::Features};

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
    let child_has_same_image_label = matches!((label, children),
        (Expr::String(label), [Node::Image { description, .. }]) if description == label
    );
    out.line_at(depth, format_args!("NexaAccessibilityPrimitive("));
    out.line_at(depth + 1, format_args!("label = {},", expression(label)));
    out.line_at(
        depth + 1,
        format_args!(
            "hint = {},",
            hint.map(expression).unwrap_or_else(|| "null".to_owned())
        ),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "value = {},",
            value.map(expression).unwrap_or_else(|| "null".to_owned())
        ),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "role = {},",
            role_name(role)
                .map(crate::generator::utils::kotlin_string)
                .unwrap_or_else(|| "null".to_owned())
        ),
    );
    out.line_at(
        depth + 1,
        format_args!("isHeading = {},", matches!(role, AccessibilityRole::Header)),
    );
    out.line_at(
        depth + 1,
        format_args!("omitLabel = {child_has_same_image_label},"),
    );
    out.line_at(depth, format_args!(") {{"));
    render_children(children, scope.module, scope.features, depth + 1, out);
    out.push('\n');
    out.line_at(depth, format_args!("}}"));
}

fn role_name(role: AccessibilityRole) -> Option<&'static str> {
    match role {
        AccessibilityRole::None | AccessibilityRole::Link | AccessibilityRole::Header => None,
        AccessibilityRole::Button => Some("Button"),
        AccessibilityRole::Image => Some("Image"),
    }
}
