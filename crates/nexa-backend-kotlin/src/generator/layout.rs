use nexa_ir::{LayoutKind, Module, Node, ViewStyle};

use super::{
    colors,
    components::render_node,
    features::Features,
    utils::{indent, number, spaces},
};

pub(super) fn render_layout(
    kind: LayoutKind,
    spacing: f32,
    style: &ViewStyle,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    let layout = match kind {
        LayoutKind::View | LayoutKind::Column => "Column",
        LayoutKind::Row => "Row",
    };
    indent(out, depth);
    let arrangement = if spacing > 0.0 {
        Some(format!(
            "{} = Arrangement.spacedBy({}.dp)",
            match kind {
                LayoutKind::Row => "horizontalArrangement",
                LayoutKind::View | LayoutKind::Column => "verticalArrangement",
            },
            number(spacing)
        ))
    } else {
        None
    };
    if style_is_empty(style) {
        if let Some(arrangement) = arrangement {
            out.push_str(&format!("{layout}({arrangement}) {{\n"));
        } else {
            out.push_str(&format!("{layout} {{\n"));
        }
    } else {
        out.push_str(&format!("{layout}(\n"));
        indent(out, depth + 1);
        out.push_str("modifier = Modifier");
        render_modifiers(style, depth + 2, out);
        out.push_str(",\n");
        if let Some(arrangement) = arrangement {
            indent(out, depth + 1);
            out.push_str(&arrangement);
            out.push_str(",\n");
        }
        indent(out, depth);
        out.push_str(") {\n");
    }
    for (index, child) in children.iter().enumerate() {
        render_node(child, module, features, depth + 1, out);
        if index + 1 < children.len() {
            out.push('\n');
        }
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

fn style_is_empty(style: &ViewStyle) -> bool {
    style.padding.is_none()
        && style.width.is_none()
        && style.height.is_none()
        && style.background.is_none()
        && style.corner_radius.is_none()
        && style.opacity.is_none()
}

fn render_modifiers(style: &ViewStyle, depth: usize, out: &mut String) {
    if let Some(opacity) = style.opacity {
        out.push_str(&format!("\n{}.alpha({}f)", spaces(depth), number(opacity)));
    }
    if let Some(radius) = style.corner_radius {
        out.push_str(&format!(
            "\n{}.clip(RoundedCornerShape({}.dp))",
            spaces(depth),
            number(radius)
        ));
    }
    if let Some(color) = style.background {
        out.push_str(&format!(
            "\n{}.background({})",
            spaces(depth),
            colors::expression(color)
        ));
    }
    if let Some(padding) = style.padding {
        out.push_str(&format!(
            "\n{}.padding({}.dp)",
            spaces(depth),
            number(padding)
        ));
    }
    if let Some(width) = style.width {
        out.push_str(&format!("\n{}.width({}.dp)", spaces(depth), number(width)));
    }
    if let Some(height) = style.height {
        out.push_str(&format!(
            "\n{}.height({}.dp)",
            spaces(depth),
            number(height)
        ));
    }
}
