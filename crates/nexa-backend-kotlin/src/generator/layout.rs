use nexa_ir::{Alignment, AnimationSpec, LayoutKind, Module, Node, ViewStyle};

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
        LayoutKind::Column => "Column",
        LayoutKind::Row => "Row",
    };
    indent(out, depth);
    let arrangement = if spacing > 0.0 {
        Some(format!(
            "{} = Arrangement.spacedBy({}.dp)",
            match kind {
                LayoutKind::Row => "horizontalArrangement",
                LayoutKind::Column => "verticalArrangement",
            },
            number(spacing)
        ))
    } else {
        None
    };
    let alignment = style.alignment.map(|alignment| {
        let (argument, value) = match (kind, alignment) {
            (LayoutKind::Row, Alignment::Start) => ("verticalAlignment", "Top"),
            (LayoutKind::Row, Alignment::Center) => ("verticalAlignment", "CenterVertically"),
            (LayoutKind::Row, Alignment::End) => ("verticalAlignment", "Bottom"),
            (LayoutKind::Column, Alignment::Start) => ("horizontalAlignment", "Start"),
            (LayoutKind::Column, Alignment::Center) => {
                ("horizontalAlignment", "CenterHorizontally")
            }
            (LayoutKind::Column, Alignment::End) => ("horizontalAlignment", "End"),
        };
        format!("{argument} = Alignment.{value}")
    });
    let has_modifier = style.has_modifiers();
    if !has_modifier && alignment.is_none() && arrangement.is_none() {
        out.push_str(&format!("{layout} {{\n"));
    } else {
        out.push_str(&format!("{layout}(\n"));
        if has_modifier {
            indent(out, depth + 1);
            out.push_str("modifier = Modifier");
            render_modifiers(style, depth + 2, out);
            out.push_str(",\n");
        }
        if let Some(alignment) = alignment {
            indent(out, depth + 1);
            out.push_str(&alignment);
            out.push_str(",\n");
        }
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
    if let (Some(color), Some(width)) = (style.border_color, style.border_width) {
        let radius = style.corner_radius.unwrap_or(0.0);
        out.push_str(&format!(
            "\n{}.border(width = {}.dp, color = {}, shape = RoundedCornerShape({}.dp))",
            spaces(depth),
            number(width),
            colors::expression(color),
            number(radius)
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
    let mut width_bounds = Vec::with_capacity(2);
    if let Some(min_width) = style.min_width {
        width_bounds.push(format!("min = {}.dp", number(min_width)));
    }
    if let Some(max_width) = style.max_width {
        width_bounds.push(format!("max = {}.dp", number(max_width)));
    }
    if !width_bounds.is_empty() {
        out.push_str(&format!(
            "\n{}.widthIn({})",
            spaces(depth),
            width_bounds.join(", ")
        ));
    }
    let mut height_bounds = Vec::with_capacity(2);
    if let Some(min_height) = style.min_height {
        height_bounds.push(format!("min = {}.dp", number(min_height)));
    }
    if let Some(max_height) = style.max_height {
        height_bounds.push(format!("max = {}.dp", number(max_height)));
    }
    if !height_bounds.is_empty() {
        out.push_str(&format!(
            "\n{}.heightIn({})",
            spaces(depth),
            height_bounds.join(", ")
        ));
    }
    if let Some(animation) = style.animation {
        let spec = match animation {
            AnimationSpec::Spring => "spring()",
            AnimationSpec::EaseIn => "tween(easing = FastOutLinearInEasing)",
            AnimationSpec::EaseOut => "tween(easing = LinearOutSlowInEasing)",
            AnimationSpec::EaseInOut => "tween(easing = FastOutSlowInEasing)",
            AnimationSpec::Linear => "tween(easing = LinearEasing)",
        };
        out.push_str(&format!(
            "\n{}.animateContentSize(animationSpec = {spec})",
            spaces(depth)
        ));
    }
}
