use nexa_codegen::SourceWriter;
use nexa_ir::{
    Alignment, AnimationEasing, AnimationSpec, Expr, LayoutKind, Node, ToolbarPlacement, ViewStyle,
};

use crate::generator::{
    colors,
    components::{render_node, render_node_in_form_row},
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
    imports.add(
        features.uses_alignment || features.uses_column || features.uses_box || features.uses_form,
        "androidx.compose.ui.Alignment",
    );
    imports.add(
        features.uses_column,
        "androidx.compose.foundation.layout.Column",
    );
    imports.add(
        features.uses_form,
        "androidx.compose.foundation.layout.fillMaxSize",
    );
    imports.add(features.uses_form, "androidx.compose.foundation.layout.Box");
    imports.add(
        features.uses_form,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(
        features.uses_form,
        "androidx.compose.foundation.rememberScrollState",
    );
    imports.add(
        features.uses_form,
        "androidx.compose.foundation.verticalScroll",
    );
    imports.add(features.uses_row, "androidx.compose.foundation.layout.Row");
    imports.add(
        features.uses_row,
        "androidx.compose.foundation.layout.fillMaxWidth",
    );
    imports.add(features.uses_row, "androidx.compose.ui.Modifier");
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
        features.uses_form,
        "androidx.compose.material3.HorizontalDivider",
    );
    imports.add(
        features.uses_form,
        "androidx.compose.material3.MaterialTheme",
    );
    imports.add(features.uses_form, "androidx.compose.material3.Surface");
    imports.add(features.uses_form, "androidx.compose.foundation.background");
    imports.add(
        features.uses_form,
        "androidx.compose.foundation.layout.Column",
    );
    imports.add(
        features.uses_form,
        "androidx.compose.foundation.shape.RoundedCornerShape",
    );
    imports.add(features.uses_form, "androidx.compose.ui.graphics.Color");
    imports.add(
        features.uses_padding,
        "androidx.compose.foundation.layout.padding",
    );
    imports.add(features.uses_form, "androidx.compose.ui.Modifier");
    imports.add(features.uses_form, "androidx.compose.ui.unit.dp");
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
        features.uses_height_in || features.uses_form,
        "androidx.compose.foundation.layout.heightIn",
    );
    imports.add(features.uses_modifier, "androidx.compose.ui.Modifier");
    imports.add(
        features.uses_opacity,
        "androidx.compose.ui.graphics.CompositingStrategy",
    );
    imports.add(features.uses_opacity, "androidx.compose.ui.draw.alpha");
    imports.add(
        features.uses_opacity || features.uses_graphics_layer,
        "androidx.compose.ui.graphics.graphicsLayer",
    );
    imports.add(
        features.uses_corner_radius || features.uses_glass,
        "androidx.compose.ui.draw.clip",
    );
    imports.add(
        features.uses_corner_radius || features.uses_drop_shadow || features.uses_glass,
        "androidx.compose.foundation.shape.RoundedCornerShape",
    );
    imports.add(
        features.uses_glass,
        "androidx.compose.foundation.shape.CircleShape",
    );
    imports.add(features.uses_glass, "androidx.compose.ui.graphics.Shape");
    imports.add(
        features.uses_glass,
        "androidx.compose.material3.MaterialTheme",
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
    imports.add(
        features.uses_text_sp || features.uses_text_node,
        "androidx.compose.ui.unit.sp",
    );
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
        LayoutKind::Column => "NexaColumnPrimitive",
        LayoutKind::Row => "NexaRowPrimitive",
        LayoutKind::Stack => "NexaStackPrimitive",
    };
    indent(out, depth);
    out.push_str(&format!("{layout}(\n"));
    if style.has_modifiers() {
        indent(out, depth + 1);
        out.push_str("modifier = Modifier");
        render_modifiers(style, depth + 2, out);
        out.push_str(",\n");
    }
    if !matches!(kind, LayoutKind::Stack) {
        indent(out, depth + 1);
        out.push_str(&format!("spacing = {}f,\n", number(spacing)));
    }
    if matches!(kind, LayoutKind::Row)
        && let Some(arrangement) = style.horizontal_arrangement
    {
        let arrangement = match arrangement {
            nexa_ir::HorizontalArrangement::Start => "Start",
            nexa_ir::HorizontalArrangement::Center => "Center",
            nexa_ir::HorizontalArrangement::End => "End",
        };
        indent(out, depth + 1);
        out.push_str(&format!(
            "horizontalArrangement = androidx.compose.foundation.layout.Arrangement.{arrangement},\n"
        ));
    }
    let alignment = match (kind, style.alignment.unwrap_or(Alignment::Center)) {
        (LayoutKind::Row, Alignment::Start) => ("verticalAlignment", "Top"),
        (LayoutKind::Row, Alignment::Center) => ("verticalAlignment", "CenterVertically"),
        (LayoutKind::Row, Alignment::End) => ("verticalAlignment", "Bottom"),
        (LayoutKind::Stack, Alignment::Start) => ("contentAlignment", "TopStart"),
        (LayoutKind::Stack, Alignment::Center) => ("contentAlignment", "Center"),
        (LayoutKind::Stack, Alignment::End) => ("contentAlignment", "BottomEnd"),
        (LayoutKind::Column, Alignment::Start) => ("horizontalAlignment", "Start"),
        (LayoutKind::Column, Alignment::Center) => ("horizontalAlignment", "CenterHorizontally"),
        (LayoutKind::Column, Alignment::End) => ("horizontalAlignment", "End"),
    };
    indent(out, depth + 1);
    out.push_str(&format!("{} = Alignment.{},\n", alignment.0, alignment.1));
    indent(out, depth);
    out.push_str(") {\n");
    let has_toolbars = children
        .iter()
        .any(|child| matches!(child, Node::Toolbar { .. }));
    if has_toolbars {
        render_toolbar_row(children, scope, depth + 1, out);
    }
    let mut rendered_content = 0;
    for child in children
        .iter()
        .filter(|child| !matches!(child, Node::Toolbar { .. }))
    {
        if matches!(kind, LayoutKind::Row) {
            super::node_renderer::render_node_in_row(
                child,
                scope.module,
                scope.features,
                depth + 1,
                out,
            );
        } else if matches!(kind, LayoutKind::Column)
            && matches!(child, Node::ContentUnavailable { .. })
        {
            out.line_at(
                depth + 1,
                format_args!("Box(modifier = Modifier.weight(1f).fillMaxWidth()) {{"),
            );
            render_node(child, scope.module, scope.features, depth + 2, out);
            out.line_at(depth + 1, format_args!("}}"));
        } else {
            render_node(child, scope.module, scope.features, depth + 1, out);
        }
        rendered_content += 1;
        if rendered_content < children.len() - usize::from(has_toolbars) {
            out.push('\n');
        }
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
}

fn render_toolbar_row(
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let toolbars = children.iter().filter_map(|node| match node {
        Node::Toolbar {
            placement,
            children,
        } => Some((*placement, children.as_slice())),
        _ => None,
    });
    let toolbars = toolbars.collect::<Vec<_>>();
    let has_leading = toolbars
        .iter()
        .any(|(placement, _)| *placement == ToolbarPlacement::Leading);
    let has_trailing = toolbars
        .iter()
        .any(|(placement, _)| *placement == ToolbarPlacement::Trailing);
    let arrangement = match (has_leading, has_trailing) {
        (true, true) => "SpaceBetween",
        (false, true) => "End",
        _ => "Start",
    };

    out.line_at(
        depth,
        format_args!("Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.{arrangement}, verticalAlignment = Alignment.CenterVertically) {{"),
    );
    for placement in [ToolbarPlacement::Leading, ToolbarPlacement::Trailing] {
        if !toolbars
            .iter()
            .any(|(toolbar_placement, _)| *toolbar_placement == placement)
        {
            continue;
        }
        let group_arrangement = "Arrangement.spacedBy(8.dp)";
        out.line_at(
            depth + 1,
            format_args!("Row(horizontalArrangement = {group_arrangement}, verticalAlignment = Alignment.CenterVertically) {{"),
        );
        let mut rendered_items = 0;
        for (_, toolbar_children) in toolbars
            .iter()
            .filter(|(toolbar_placement, _)| *toolbar_placement == placement)
        {
            for child in *toolbar_children {
                if rendered_items > 0 {
                    out.push('\n');
                }
                render_node(child, scope.module, scope.features, depth + 2, out);
                rendered_items += 1;
            }
        }
        out.push('\n');
        indent(out, depth + 1);
        out.push_str("}\n");
    }
    out.line_at(depth, format_args!("}}"));
}

pub(crate) struct FormSectionRenderOptions<'a> {
    pub(crate) title: Option<&'a Expr>,
    pub(crate) footer: Option<&'a Expr>,
    pub(crate) show_dividers: bool,
    pub(crate) first_section: bool,
    pub(crate) background: Option<nexa_ir::ColorValue>,
}

pub(crate) fn render_form_section(
    options: FormSectionRenderOptions<'_>,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let title = options
        .title
        .map(crate::generator::engine::expressions::text_expression)
        .unwrap_or_else(|| "null".to_owned());
    let footer = options
        .footer
        .map(crate::generator::engine::expressions::text_expression)
        .unwrap_or_else(|| "null".to_owned());
    out.line_at(depth, format_args!("NexaFormSectionPrimitive("));
    out.line_at(depth + 1, format_args!("title = {title},"));
    out.line_at(depth + 1, format_args!("footer = {footer},"));
    out.line_at(
        depth + 1,
        format_args!("firstSection = {},", options.first_section),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "background = {},",
            options
                .background
                .map(crate::generator::engine::colors::expression)
                .unwrap_or_else(|| "null".to_owned())
        ),
    );
    out.line_at(depth, format_args!(") {{"));
    for (index, child) in children.iter().enumerate() {
        render_form_row(child, 0, scope, depth + 1, out);
        if options.show_dividers && index + 1 < children.len() {
            out.line_at(depth + 1, format_args!("NexaFormDividerPrimitive()"));
        }
    }
    out.line_at(depth, format_args!("}}"));
}

fn render_form_row(
    node: &Node,
    horizontal_inset: u8,
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(
        depth,
        format_args!(
            "NexaFormRowPrimitive(minHeight = {}f, horizontalInset = {}f) {{",
            nexa_codegen::design_system::ANDROID_FORM_ROW_MIN_HEIGHT,
            horizontal_inset,
        ),
    );
    render_node_in_form_row(node, scope.module, scope.features, depth + 1, out);
    out.line_at(depth, format_args!("}}"));
}

pub(crate) fn render_form(
    show_dividers: bool,
    background: Option<nexa_ir::ColorValue>,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    let background = background
        .map(crate::generator::engine::colors::expression)
        .unwrap_or_else(|| "null".to_owned());
    out.line_at(
        depth,
        format_args!("NexaFormPrimitive(background = {background}) {{"),
    );
    let mut first_section = true;
    for (index, child) in children.iter().enumerate() {
        if let Node::FormSection {
            title,
            footer,
            show_dividers: section_show_dividers,
            background: section_background,
            children,
        } = child
        {
            render_form_section(
                FormSectionRenderOptions {
                    title: title.as_ref(),
                    footer: footer.as_ref(),
                    show_dividers: section_show_dividers.unwrap_or(show_dividers),
                    first_section,
                    background: *section_background,
                },
                children,
                scope,
                depth + 1,
                out,
            );
            first_section = false;
        } else {
            render_form_row(
                child,
                nexa_codegen::design_system::FORM_ROW_HORIZONTAL_INSET,
                scope,
                depth + 1,
                out,
            );
            let next_is_section = children
                .get(index + 1)
                .is_some_and(|next| matches!(next, Node::FormSection { .. }));
            if show_dividers && index + 1 < children.len() && !next_is_section {
                out.line_at(depth + 1, format_args!("NexaFormDividerPrimitive()"));
            }
        }
        if index + 1 < children.len() {
            out.push('\n');
        }
    }
    out.line_at(depth, format_args!("}}"));
}

fn render_modifiers(style: &ViewStyle, depth: usize, out: &mut SourceWriter) {
    let effects = &style.effects;
    if let Some(opacity) = style.opacity {
        out.push_str(&format!(
            "\n{}.graphicsLayer(alpha = {}f, compositingStrategy = CompositingStrategy.Offscreen)",
            spaces(depth),
            number(opacity)
        ));
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
    if let Some(glass) = &effects.glass {
        // Android represents glass with a solid Material 3 surface. Place it
        // outside padding and sizing so it covers the full layout, like SwiftUI.
        let shape = match glass.shape {
            nexa_ir::GlassShape::Circle => "CircleShape".to_owned(),
            nexa_ir::GlassShape::Capsule => "RoundedCornerShape(percent = 50)".to_owned(),
            nexa_ir::GlassShape::Rounded(radius) => {
                format!("RoundedCornerShape({}.dp)", number(radius))
            }
        };
        let tint = glass
            .tint
            .map(colors::expression)
            .map(|tint| format!("{tint}.copy(alpha = 1f)"))
            .or_else(|| style.background.map(colors::expression))
            .unwrap_or_else(|| "MaterialTheme.colorScheme.surface".to_owned());
        out.push_str(&format!(
            "\n{}.let {{ val surfaceShape: Shape = {shape}; val surfaceColor = {tint}; it.clip(surfaceShape).background(surfaceColor, surfaceShape) }}",
            spaces(depth),
        ));
    }
    if let Some(animation) = style.animation {
        let spec = match animation {
            AnimationSpec::Spring { response, damping } => {
                let physics = nexa_codegen::animation::spring_physics(response, damping);
                format!(
                    "spring(dampingRatio = {damping}f, stiffness = {}f)",
                    physics.stiffness
                )
            }
            AnimationSpec::Tween {
                duration,
                delay,
                easing,
            } => {
                let duration_millis = (duration * 1000.0).round() as u32;
                let delay_millis = (delay * 1000.0).round() as u32;
                let delay_parameter = if delay_millis > 0 {
                    format!(", delayMillis = {delay_millis}")
                } else {
                    String::new()
                };
                format!(
                    "tween(durationMillis = {duration_millis}{delay_parameter}, easing = {})",
                    easing_name(easing)
                )
            }
            AnimationSpec::EaseIn => {
                "tween(durationMillis = 300, easing = FastOutLinearInEasing)".to_owned()
            }
            AnimationSpec::EaseOut => {
                "tween(durationMillis = 300, easing = LinearOutSlowInEasing)".to_owned()
            }
            AnimationSpec::EaseInOut => {
                "tween(durationMillis = 300, easing = FastOutSlowInEasing)".to_owned()
            }
            AnimationSpec::Linear => {
                "tween(durationMillis = 300, easing = LinearEasing)".to_owned()
            }
        };
        out.push_str(&format!(
            "\n{}.animateContentSize(animationSpec = {spec})",
            spaces(depth)
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
}

fn easing_name(easing: AnimationEasing) -> String {
    match easing {
        AnimationEasing::Linear => "LinearEasing".to_owned(),
        AnimationEasing::EaseIn => "FastOutLinearInEasing".to_owned(),
        AnimationEasing::EaseOut => "LinearOutSlowInEasing".to_owned(),
        AnimationEasing::EaseInOut => "FastOutSlowInEasing".to_owned(),
        AnimationEasing::CubicBezier { x1, y1, x2, y2 } => {
            format!("androidx.compose.animation.core.CubicBezierEasing({x1}f, {y1}f, {x2}f, {y2}f)")
        }
    }
}

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{
        Color, ColorValue, Expr, GlassShape, KeyboardType, LayoutKind, Module, Node, SystemIcon,
        TextStyle, ToolbarPlacement, ViewGlass, ViewStyle,
    };

    use crate::generator::features::Features;

    use super::{
        FormSectionRenderOptions, RenderScope, render_form_section, render_layout, render_modifiers,
    };

    #[test]
    fn content_size_animation_wraps_padding_and_size_constraints() {
        let style = ViewStyle {
            animation: Some(nexa_ir::AnimationSpec::EaseInOut),
            padding: Some(8.0),
            width: Some(200.0),
            min_height: Some(48.0),
            ..ViewStyle::default()
        };
        let mut output = SourceWriter::new();

        render_modifiers(&style, 0, &mut output);

        let generated = output.finish();
        let animation = generated
            .find(".animateContentSize(")
            .expect("emit the content-size animation");
        for sizing_modifier in [".padding(8.dp)", ".width(200.dp)", ".heightIn(min = 48.dp)"] {
            let sizing = generated
                .find(sizing_modifier)
                .unwrap_or_else(|| panic!("emit {sizing_modifier}"));
            assert!(animation < sizing, "animation must wrap {sizing_modifier}");
        }
    }

    #[test]
    fn spring_content_animation_uses_shared_physics() {
        let style = ViewStyle {
            animation: Some(nexa_ir::AnimationSpec::Spring {
                response: 0.35,
                damping: 0.8,
            }),
            ..ViewStyle::default()
        };
        let mut output = SourceWriter::new();

        render_modifiers(&style, 0, &mut output);

        assert!(output
            .finish()
            .contains(".animateContentSize(animationSpec = spring(dampingRatio = 0.8f, stiffness = 816.32654f))"));
    }

    #[test]
    fn glass_maps_to_a_borderless_material_surface_on_android() {
        let style = ViewStyle {
            padding: Some(8.0),
            width: Some(220.0),
            height: Some(48.0),
            effects: nexa_ir::ViewEffects {
                glass: Some(ViewGlass {
                    tint: Some(ColorValue::Static(Color {
                        red: 221,
                        green: 235,
                        blue: 255,
                        alpha: 255,
                    })),
                    shape: GlassShape::Capsule,
                }),
                ..nexa_ir::ViewEffects::default()
            },
            ..ViewStyle::default()
        };
        let mut output = SourceWriter::new();

        render_modifiers(&style, 0, &mut output);

        let generated = output.finish();
        assert!(generated.contains("val surfaceShape: Shape = RoundedCornerShape(percent = 50)"));
        assert!(generated.contains("Color(0xFFDDEBFF).copy(alpha = 1f)"));
        assert!(generated.contains("it.clip(surfaceShape).background(surfaceColor, surfaceShape)"));
        let glass = generated
            .find("val surfaceShape")
            .expect("emit glass surface");
        let padding = generated.find(".padding(8.dp)").expect("emit row padding");
        let width = generated.find(".width(220.dp)").expect("emit row width");
        let height = generated.find(".height(48.dp)").expect("emit row height");
        assert!(glass < padding && padding < width && width < height);
        assert!(!generated.contains(".shadow("));
        assert!(!generated.contains(".border("));
        assert!(!generated.contains("copy(alpha = 0.05f)"));
        assert!(!generated.contains("Color.White.copy"));
    }

    #[test]
    fn untinted_glass_preserves_an_explicit_android_background() {
        let style = ViewStyle {
            background: Some(ColorValue::Static(Color {
                red: 242,
                green: 242,
                blue: 247,
                alpha: 255,
            })),
            effects: nexa_ir::ViewEffects {
                glass: Some(ViewGlass {
                    tint: None,
                    shape: GlassShape::Rounded(8.0),
                }),
                ..nexa_ir::ViewEffects::default()
            },
            ..ViewStyle::default()
        };
        let mut output = SourceWriter::new();

        render_modifiers(&style, 0, &mut output);

        let generated = output.finish();
        assert!(generated.contains("val surfaceColor = Color(0xFFF2F2F7)"));
        assert!(generated.contains("it.clip(surfaceShape).background(surfaceColor, surfaceShape)"));
    }

    #[test]
    fn width_bounded_columns_preserve_intrinsic_child_width() {
        let module = Module {
            app_name: "ColumnTextBounds".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: Vec::new(),
            globals: Vec::new(),
            screens: Vec::new(),
            widgets: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        let features = Features::default();
        let scope = RenderScope {
            module: &module,
            features: &features,
        };
        let children = [Node::Text {
            value: Expr::String("Wrapped text stays centered".to_owned()),
            style: TextStyle::default(),
        }];
        let style = ViewStyle {
            min_width: Some(210.0),
            max_width: Some(260.0),
            ..ViewStyle::default()
        };
        let mut output = SourceWriter::new();

        render_layout(
            LayoutKind::Column,
            0.0,
            &style,
            &children,
            &scope,
            0,
            &mut output,
        );

        let generated = output.finish();
        assert!(generated.contains("horizontalAlignment = Alignment.CenterHorizontally"));
        assert!(!generated.contains("fillTextWidth"));
    }

    #[test]
    fn opacity_uses_an_offscreen_group_for_consistent_compositing() {
        let style = ViewStyle {
            opacity: Some(0.45),
            ..ViewStyle::default()
        };
        let mut output = SourceWriter::new();

        render_modifiers(&style, 0, &mut output);

        assert!(output.finish().contains(
            ".graphicsLayer(alpha = 0.45f, compositingStrategy = CompositingStrategy.Offscreen)"
        ));
    }

    #[test]
    fn row_text_input_takes_remaining_width_instead_of_clipping_trailing_controls() {
        let module = Module {
            app_name: "RowInputParity".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: Vec::new(),
            globals: Vec::new(),
            screens: Vec::new(),
            widgets: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        let features = Features::default();
        let scope = RenderScope {
            module: &module,
            features: &features,
        };
        let input = Node::TextInput {
            state: "newComment".to_owned(),
            placeholder: "Add comment...".to_owned(),
            comment: None,
            keyboard: KeyboardType::Text,
            secure: false,
            multiline: false,
            autofill: None,
            return_key: None,
            autocorrect: None,
            capitalization: None,
            focused: None,
            max_length: None,
            font: None,
            min_lines: None,
            max_lines: None,
            searchable: false,
            actions: Vec::new(),
            on_change: None,
        };
        let children = [
            input,
            Node::Text {
                value: Expr::String("Trailing action".to_owned()),
                style: TextStyle::default(),
            },
        ];
        let mut output = SourceWriter::new();

        render_layout(
            LayoutKind::Row,
            0.0,
            &ViewStyle::default(),
            &children,
            &scope,
            0,
            &mut output,
        );

        assert!(output.contains("NexaTextInputPrimitive("));
        assert!(output.contains("searchable = false"));
        assert!(output.contains("modifier = Modifier.weight(1f)"));
        assert!(output.contains("NexaTextPrimitive(text = \"Trailing action\""));
    }

    #[test]
    fn column_toolbars_share_a_leading_and_trailing_action_row() {
        let module = Module {
            app_name: "ToolbarParity".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: Vec::new(),
            globals: Vec::new(),
            screens: Vec::new(),
            widgets: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        let features = Features::default();
        let scope = RenderScope {
            module: &module,
            features: &features,
        };
        let children = vec![
            Node::Toolbar {
                placement: ToolbarPlacement::Leading,
                children: vec![Node::Text {
                    value: Expr::String("Priority".to_owned()),
                    style: TextStyle::default(),
                }],
            },
            Node::Toolbar {
                placement: ToolbarPlacement::Trailing,
                children: vec![Node::Text {
                    value: Expr::String("Save".to_owned()),
                    style: TextStyle::default(),
                }],
            },
            Node::Text {
                value: Expr::String("Task name".to_owned()),
                style: TextStyle::default(),
            },
        ];
        let mut output = SourceWriter::new();

        render_layout(
            LayoutKind::Column,
            0.0,
            &ViewStyle::default(),
            &children,
            &scope,
            0,
            &mut output,
        );

        assert!(output.contains("NexaColumnPrimitive("));
        assert!(output.contains(
            "Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {"
        ));
        assert!(output.contains("Row(horizontalArrangement = Arrangement.spacedBy(8.dp)"));
        assert!(
            output.contains("NexaTextPrimitive(text = \"Priority\"")
                && output.contains("NexaTextPrimitive(text = \"Save\"")
        );
        assert!(output.contains("NexaTextPrimitive(text = \"Task name\""));
        assert_eq!(
            output
                .as_str()
                .matches("Row(modifier = Modifier.fillMaxWidth()")
                .count(),
            1
        );
    }

    #[test]
    fn form_section_insets_rows_and_divides_adjacent_controls() {
        let module = Module {
            app_name: "FormParity".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: Vec::new(),
            globals: Vec::new(),
            screens: Vec::new(),
            widgets: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        let features = Features::default();
        let scope = RenderScope {
            module: &module,
            features: &features,
        };
        let children = [
            Node::Button {
                label: Expr::String("Send Feedback".to_owned()),
                icon: None,
                loading: None,
                disabled: None,
                style: Some(nexa_ir::ButtonStyle::Plain),
                size: None,
                shape: None,
                tint: None,
                glass: false,
                actions: Vec::new(),
            },
            Node::Text {
                value: Expr::String("First setting".to_owned()),
                style: TextStyle::default(),
            },
            Node::Text {
                value: Expr::String("Second setting".to_owned()),
                style: TextStyle::default(),
            },
            Node::Picker {
                items: Expr::Array(vec![Expr::String("10 minutes before".to_owned())]),
                state: "reminderChoice".to_owned(),
                icon: Some(SystemIcon::Shared("notifications".to_owned())),
                item_icons: Vec::new(),
                label: Some(Expr::String("Automatic\nReminders".to_owned())),
                tint: None,
            },
        ];
        let mut output = SourceWriter::new();
        let footer = Expr::String("ToDoList v1.0.0 (1)".to_owned());

        render_form_section(
            FormSectionRenderOptions {
                title: None,
                footer: Some(&footer),
                show_dividers: true,
                first_section: false,
                background: None,
            },
            &children,
            &scope,
            0,
            &mut output,
        );

        assert!(output.contains("NexaFormSectionPrimitive("));
        assert!(output.contains("title = null,"));
        assert!(output.contains("footer = \"ToDoList v1.0.0 (1)\","));
        assert!(output.contains("NexaFormRowPrimitive(minHeight = 56f, horizontalInset = 0f) {"));
        assert!(output.contains("fullWidth = true,"));
        assert!(output.contains("leadingContent = true,"));
        assert_eq!(
            output
                .as_str()
                .matches("NexaFormDividerPrimitive()")
                .count(),
            children.len() - 1
        );
        assert_eq!(
            output
                .as_str()
                .matches("NexaFormRowPrimitive(minHeight = 56f")
                .count(),
            children.len()
        );
        assert!(!output.contains("heightIn(min = 44.dp)"));
        assert!(!output.contains("heightIn(min = 58.dp)"));
    }
}
