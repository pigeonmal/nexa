use nexa_codegen::SourceWriter;
use nexa_ir::{AccessibilityRole, Expr, Module, Node};

use crate::generator::{
    components::render_children,
    expressions::expression,
    features::Features,
    utils::{indent, swift_string},
};

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
    render_children(children, module, features, depth, out);
    let child_has_same_image_label = matches!((label, children),
        (Expr::String(label), [Node::Image { description, .. }]) if description == label
    );
    if children.len() > 1 {
        out.push('\n');
        indent(out, depth + 1);
        out.push_str(".accessibilityElement(children: .combine)");
    }
    if !child_has_same_image_label {
        out.push('\n');
        indent(out, depth + 1);
        let label = match label {
            Expr::String(value) => swift_string(value),
            value => format!("Text({})", expression(value)),
        };
        out.push_str(&format!(".accessibilityLabel({label})"));
    }
    if let Some(hint) = hint {
        out.push('\n');
        indent(out, depth + 1);
        let hint = match hint {
            Expr::String(value) => swift_string(value),
            value => format!("Text({})", expression(value)),
        };
        out.push_str(&format!(".accessibilityHint({hint})"));
    }
    if let Some(trait_name) = trait_name(role) {
        out.push('\n');
        out.text_at(
            depth + 1,
            format_args!(".accessibilityAddTraits({trait_name})"),
        );
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
