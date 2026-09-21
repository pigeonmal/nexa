use nexa_ir::{Alignment, LayoutKind, Module, Node, ViewStyle};

use super::{
    colors,
    components::render_node,
    utils::{indent, number},
};

pub(super) fn render_layout(
    kind: LayoutKind,
    spacing: f32,
    style: &ViewStyle,
    children: &[Node],
    module: &Module,
    depth: usize,
    out: &mut String,
) {
    let layout = match kind {
        LayoutKind::View | LayoutKind::Column => "VStack",
        LayoutKind::Row => "HStack",
    };
    indent(out, depth);
    let alignment = style.alignment.map(|alignment| match (kind, alignment) {
        (LayoutKind::Row, Alignment::Start) => ".top",
        (LayoutKind::Row, Alignment::Center) => ".center",
        (LayoutKind::Row, Alignment::End) => ".bottom",
        (_, Alignment::Start) => ".leading",
        (_, Alignment::Center) => ".center",
        (_, Alignment::End) => ".trailing",
    });
    match (alignment, spacing > 0.0) {
        (Some(alignment), true) => out.push_str(&format!(
            "{layout}(alignment: {alignment}, spacing: {}) {{\n",
            number(spacing)
        )),
        (Some(alignment), false) => {
            out.push_str(&format!("{layout}(alignment: {alignment}) {{\n"));
        }
        (None, true) => {
            out.push_str(&format!("{layout}(spacing: {}) {{\n", number(spacing)));
        }
        (None, false) => out.push_str(&format!("{layout} {{\n")),
    }
    for (index, child) in children.iter().enumerate() {
        render_node(child, module, depth + 1, out);
        if index + 1 < children.len() {
            out.push('\n');
        }
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
    append_style(out, depth, style);
}

fn append_style(out: &mut String, depth: usize, style: &ViewStyle) {
    if let (Some(width), Some(height)) = (style.width, style.height) {
        append_modifier(
            out,
            depth,
            &format!(
                "frame(width: {}, height: {})",
                number(width),
                number(height)
            ),
        );
    } else if let Some(width) = style.width {
        append_modifier(out, depth, &format!("frame(width: {})", number(width)));
    } else if let Some(height) = style.height {
        append_modifier(out, depth, &format!("frame(height: {})", number(height)));
    }
    if let Some(padding) = style.padding {
        append_modifier(out, depth, &format!("padding({})", number(padding)));
    }
    if let Some(color) = style.background {
        append_modifier(
            out,
            depth,
            &format!("background({})", colors::expression(color)),
        );
    }
    if let Some(radius) = style.corner_radius {
        append_modifier(
            out,
            depth,
            &format!(
                "clipShape(RoundedRectangle(cornerRadius: {}))",
                number(radius)
            ),
        );
    }
    if let Some(opacity) = style.opacity {
        append_modifier(out, depth, &format!("opacity({})", number(opacity)));
    }
}

fn append_modifier(out: &mut String, depth: usize, modifier: &str) {
    out.push('\n');
    indent(out, depth + 1);
    out.push('.');
    out.push_str(modifier);
}
