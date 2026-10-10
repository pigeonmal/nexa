use nexa_codegen::SourceWriter;
use nexa_ir::{AccessibilityRole, Expr, Node};

use crate::generator::{components::render_children, expressions::localized_text_view};

use super::RenderScope;

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
    out.line_at(
        depth + 1,
        format_args!("label: {},", localized_text_view(label, None)),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "hint: {},",
            hint.map(|value| localized_text_view(value, None))
                .unwrap_or_else(|| "nil".to_owned())
        ),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "value: {},",
            value
                .map(|value| localized_text_view(value, None))
                .unwrap_or_else(|| "nil".to_owned())
        ),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "role: {},",
            crate::generator::utils::swift_string(role_name(role))
        ),
    );
    out.line_at(
        depth + 1,
        format_args!("omitLabel: {child_has_same_image_label},"),
    );
    out.line_at(depth, format_args!(") {{"));
    render_children(children, scope.module, scope.features, depth + 1, out);
    out.push('\n');
    out.line_at(depth, format_args!("}}"));
}

fn role_name(role: AccessibilityRole) -> &'static str {
    match role {
        AccessibilityRole::None => "None",
        AccessibilityRole::Button => "Button",
        AccessibilityRole::Link => "Link",
        AccessibilityRole::Header => "Header",
        AccessibilityRole::Image => "Image",
    }
}
