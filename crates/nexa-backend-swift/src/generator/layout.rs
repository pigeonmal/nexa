use nexa_ir::{Alignment, AnimationSpec, LayoutKind, Module, Node, ViewStyle};

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
        LayoutKind::Column => "VStack",
        LayoutKind::Row => "HStack",
        LayoutKind::Stack => "ZStack",
    };
    indent(out, depth);
    let alignment = style.alignment.map(|alignment| match (kind, alignment) {
        (LayoutKind::Row, Alignment::Start) => ".top",
        (LayoutKind::Row, Alignment::Center) => ".center",
        (LayoutKind::Row, Alignment::End) => ".bottom",
        (LayoutKind::Stack, Alignment::Start) => ".topLeading",
        (LayoutKind::Stack, Alignment::Center) => ".center",
        (LayoutKind::Stack, Alignment::End) => ".bottomTrailing",
        (_, Alignment::Start) => ".leading",
        (_, Alignment::Center) => ".center",
        (_, Alignment::End) => ".trailing",
    });
    let has_spacing = spacing > 0.0 && !matches!(kind, LayoutKind::Stack);
    match (alignment, has_spacing) {
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
    let mut bounds = Vec::with_capacity(4);
    if let Some(min_width) = style.min_width {
        bounds.push(format!("minWidth: {}", number(min_width)));
    }
    if let Some(max_width) = style.max_width {
        bounds.push(format!("maxWidth: {}", number(max_width)));
    }
    if let Some(min_height) = style.min_height {
        bounds.push(format!("minHeight: {}", number(min_height)));
    }
    if let Some(max_height) = style.max_height {
        bounds.push(format!("maxHeight: {}", number(max_height)));
    }
    if !bounds.is_empty() {
        append_modifier(out, depth, &format!("frame({})", bounds.join(", ")));
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
    if let (Some(color), Some(width)) = (style.border_color, style.border_width) {
        let radius = style.corner_radius.unwrap_or(0.0);
        append_modifier(
            out,
            depth,
            &format!(
                "overlay(RoundedRectangle(cornerRadius: {}).stroke({}, lineWidth: {}))",
                number(radius),
                colors::expression(color),
                number(width)
            ),
        );
    }
    if let Some(opacity) = style.opacity {
        append_modifier(out, depth, &format!("opacity({})", number(opacity)));
    }
    if let Some(animation) = style.animation {
        let value = match animation {
            AnimationSpec::Spring => ".spring()",
            AnimationSpec::EaseIn => ".easeIn",
            AnimationSpec::EaseOut => ".easeOut",
            AnimationSpec::EaseInOut => ".easeInOut",
            AnimationSpec::Linear => ".linear",
        };
        append_modifier(out, depth, &format!("animation({value})"));
    }
}

fn append_modifier(out: &mut String, depth: usize, modifier: &str) {
    out.push('\n');
    indent(out, depth + 1);
    out.push('.');
    out.push_str(modifier);
}
