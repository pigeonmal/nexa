use nexa_codegen::SourceWriter;
use nexa_ir::{Alignment, AnimationSpec, LayoutKind, Node, ViewStyle};

use crate::generator::{
    colors,
    components::render_node,
    utils::{indent, number},
};

use super::RenderScope;

pub(crate) fn render_layout(
    kind: LayoutKind,
    spacing: f32,
    style: &ViewStyle,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
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
        render_node(child, scope.module, scope.features, depth + 1, out);
        if index + 1 < children.len() {
            out.push('\n');
        }
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
    append_style(out, depth, style);
}

fn append_style(out: &mut SourceWriter, depth: usize, style: &ViewStyle) {
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
    append_visual_effects(out, depth, style.opacity, &style.effects);
    if let Some(animation) = style.animation {
        let value = match animation {
            AnimationSpec::Spring { response, damping } => {
                format!(".spring(response: {response}, dampingFraction: {damping})")
            }
            AnimationSpec::EaseIn => ".easeIn".to_owned(),
            AnimationSpec::EaseOut => ".easeOut".to_owned(),
            AnimationSpec::EaseInOut => ".easeInOut".to_owned(),
            AnimationSpec::Linear => ".linear".to_owned(),
        };
        append_modifier(out, depth, &format!("animation({value})"));
    }
}

pub(crate) fn append_visual_effects(
    out: &mut SourceWriter,
    depth: usize,
    opacity: Option<f32>,
    effects: &nexa_ir::ViewEffects,
) {
    if let Some(opacity) = opacity {
        append_modifier(out, depth, &format!("opacity({})", number(opacity)));
    }
    if let Some(scale) = effects.scale {
        append_modifier(out, depth, &format!("scaleEffect({})", number(scale)));
    }
    if let Some(rotation) = effects.rotation {
        append_modifier(
            out,
            depth,
            &format!("rotationEffect(.degrees({}))", number(rotation)),
        );
    }
    if let Some(shadow) = effects.shadow {
        append_modifier(
            out,
            depth,
            &format!(
                "shadow(color: {}, radius: {}, x: {}, y: {})",
                colors::expression(shadow.color),
                number(shadow.radius),
                number(shadow.x),
                number(shadow.y)
            ),
        );
    }
    if let Some(blur) = effects.blur {
        append_modifier(out, depth, &format!("blur(radius: {})", number(blur)));
    }
    if let Some(radius) = effects.clip_rounded {
        append_modifier(
            out,
            depth,
            &format!(
                "clipShape(RoundedRectangle(cornerRadius: {}))",
                number(radius)
            ),
        );
    }
    if let Some(z_index) = effects.z_index {
        append_modifier(out, depth, &format!("zIndex({z_index})"));
    }
}

fn append_modifier(out: &mut SourceWriter, depth: usize, modifier: &str) {
    out.push('\n');
    indent(out, depth + 1);
    out.push('.');
    out.push_str(modifier);
}
