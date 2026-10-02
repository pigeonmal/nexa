use nexa_codegen::SourceWriter;
use nexa_ir::{Alignment, AnimationSpec, LayoutKind, Node, ViewStyle};

use crate::generator::{
    colors,
    components::render_node,
    features::Features,
    utils::{indent, number, spaces},
};

use crate::generator::engine::imports::ImportSet;

use super::RenderScope;

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    let uses_linear_gradient = features.facts.ui.linear_gradient;
    imports.add(
        uses_linear_gradient,
        "androidx.compose.foundation.layout.Box",
    );
    imports.add(
        uses_linear_gradient,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        uses_linear_gradient,
        "androidx.compose.foundation.layout.height",
    );
    imports.add(
        uses_linear_gradient,
        "androidx.compose.foundation.background",
    );
    imports.add(uses_linear_gradient, "androidx.compose.ui.Modifier");
    imports.add(uses_linear_gradient, "androidx.compose.ui.graphics.Brush");
    imports.add(uses_linear_gradient, "androidx.compose.ui.graphics.Color");
    imports.add(uses_linear_gradient, "androidx.compose.ui.unit.dp");
    imports.add(
        features.uses_background,
        "androidx.compose.foundation.background",
    );
    imports.add(features.uses_border, "androidx.compose.foundation.border");
    imports.add(
        features.uses_animation,
        "androidx.compose.animation.animateContentSize",
    );
    imports.add(
        features.uses_animation_ease_in,
        "androidx.compose.animation.core.FastOutLinearInEasing",
    );
    imports.add(
        features.uses_animation_ease_in_out,
        "androidx.compose.animation.core.FastOutSlowInEasing",
    );
    imports.add(
        features.uses_animation_linear,
        "androidx.compose.animation.core.LinearEasing",
    );
    imports.add(
        features.uses_animation_ease_out,
        "androidx.compose.animation.core.LinearOutSlowInEasing",
    );
    imports.add(
        features.uses_animation_spring,
        "androidx.compose.animation.core.spring",
    );
    imports.add(
        features.uses_animation_ease_in
            || features.uses_animation_ease_out
            || features.uses_animation_ease_in_out
            || features.uses_animation_linear,
        "androidx.compose.animation.core.tween",
    );
    imports.add(features.uses_box, "androidx.compose.foundation.layout.Box");
    imports.add(features.uses_alignment, "androidx.compose.ui.Alignment");
    imports.add(
        features.uses_column,
        "androidx.compose.foundation.layout.Column",
    );
    imports.add(features.uses_row, "androidx.compose.foundation.layout.Row");
    imports.add(
        features.uses_arrangement,
        "androidx.compose.foundation.layout.Arrangement",
    );
    imports.add(
        features.uses_spacer,
        "androidx.compose.foundation.layout.Spacer",
    );
    imports.add(
        features.uses_divider,
        "androidx.compose.material3.HorizontalDivider",
    );
    imports.add(
        features.uses_padding,
        "androidx.compose.foundation.layout.padding",
    );
    imports.add(
        features.uses_width,
        "androidx.compose.foundation.layout.width",
    );
    imports.add(
        features.uses_height,
        "androidx.compose.foundation.layout.height",
    );
    imports.add(
        features.uses_width_in,
        "androidx.compose.foundation.layout.widthIn",
    );
    imports.add(
        features.uses_height_in,
        "androidx.compose.foundation.layout.heightIn",
    );
    imports.add(features.uses_modifier, "androidx.compose.ui.Modifier");
    imports.add(features.uses_opacity, "androidx.compose.ui.draw.alpha");
    imports.add(features.uses_corner_radius, "androidx.compose.ui.draw.clip");
    imports.add(
        features.uses_corner_radius || features.uses_drop_shadow,
        "androidx.compose.foundation.shape.RoundedCornerShape",
    );
    imports.add(
        features.uses_graphics_layer,
        "androidx.compose.ui.graphics.graphicsLayer",
    );
    imports.add(
        features.uses_drop_shadow,
        "androidx.compose.ui.draw.dropShadow",
    );
    imports.add(
        features.uses_drop_shadow,
        "androidx.compose.ui.graphics.shadow.Shadow",
    );
    imports.add(
        features.uses_drop_shadow,
        "androidx.compose.ui.unit.DpOffset",
    );
    imports.add(features.uses_blur, "androidx.compose.ui.draw.blur");
    imports.add(features.uses_z_index, "androidx.compose.ui.zIndex");
    imports.add(features.uses_color, "androidx.compose.ui.graphics.Color");
    imports.add(features.uses_dp, "androidx.compose.ui.unit.dp");
    imports.add(features.uses_text_sp, "androidx.compose.ui.unit.sp");
    imports.add(
        features.uses_adaptive_color,
        "androidx.compose.foundation.isSystemInDarkTheme",
    );
}

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
        LayoutKind::Column => "Column",
        LayoutKind::Row => "Row",
        LayoutKind::Stack => "Box",
    };
    indent(out, depth);
    let arrangement = if spacing > 0.0 && !matches!(kind, LayoutKind::Stack) {
        Some(format!(
            "{} = Arrangement.spacedBy({}.dp)",
            match kind {
                LayoutKind::Row => "horizontalArrangement",
                LayoutKind::Column => "verticalArrangement",
                LayoutKind::Stack => unreachable!("stack has no arrangement"),
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
            (LayoutKind::Stack, Alignment::Start) => ("contentAlignment", "TopStart"),
            (LayoutKind::Stack, Alignment::Center) => ("contentAlignment", "Center"),
            (LayoutKind::Stack, Alignment::End) => ("contentAlignment", "BottomEnd"),
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
        render_node(child, scope.module, scope.features, depth + 1, out);
        if index + 1 < children.len() {
            out.push('\n');
        }
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

fn render_modifiers(style: &ViewStyle, depth: usize, out: &mut SourceWriter) {
    let effects = &style.effects;
    if let Some(opacity) = style.opacity {
        out.push_str(&format!("\n{}.alpha({}f)", spaces(depth), number(opacity)));
    }
    if effects.scale.is_some() || effects.rotation.is_some() {
        let scale = effects
            .scale
            .map(number)
            .unwrap_or_else(|| "1.0".to_owned());
        let rotation = effects
            .rotation
            .map(number)
            .unwrap_or_else(|| "0.0".to_owned());
        out.push_str(&format!(
            "\n{}.graphicsLayer(scaleX = {scale}f, scaleY = {scale}f, rotationZ = {rotation}f)",
            spaces(depth)
        ));
    }
    if let Some(blur) = effects.blur {
        out.push_str(&format!("\n{}.blur({}.dp)", spaces(depth), number(blur)));
    }
    if let Some(shadow) = effects.shadow {
        let radius = effects.clip_rounded.or(style.corner_radius).unwrap_or(0.0);
        out.push_str(&format!(
            "\n{}.dropShadow(shape = RoundedCornerShape({}.dp), shadow = Shadow(radius = {}.dp, color = {}, offset = DpOffset({}.dp, {}.dp)))",
            spaces(depth),
            number(radius),
            number(shadow.radius),
            colors::expression(shadow.color),
            number(shadow.x),
            number(shadow.y)
        ));
    }
    if let Some(radius) = effects.clip_rounded.or(style.corner_radius) {
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
    if let Some(z_index) = effects.z_index {
        out.push_str(&format!("\n{}.zIndex({}f)", spaces(depth), z_index));
    }
    if let Some(animation) = style.animation {
        let spec = match animation {
            AnimationSpec::Spring { response, damping } => {
                let stiffness = (100.0 / f64::from(response).powi(2)) as f32;
                format!("spring(dampingRatio = {damping}f, stiffness = {stiffness}f)")
            }
            AnimationSpec::EaseIn => "tween(easing = FastOutLinearInEasing)".to_owned(),
            AnimationSpec::EaseOut => "tween(easing = LinearOutSlowInEasing)".to_owned(),
            AnimationSpec::EaseInOut => "tween(easing = FastOutSlowInEasing)".to_owned(),
            AnimationSpec::Linear => "tween(easing = LinearEasing)".to_owned(),
        };
        out.push_str(&format!(
            "\n{}.animateContentSize(animationSpec = {spec})",
            spaces(depth)
        ));
    }
}
