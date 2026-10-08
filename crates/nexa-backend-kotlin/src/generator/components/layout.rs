use nexa_codegen::SourceWriter;
use nexa_ir::{Alignment, AnimationSpec, LayoutKind, Node, ToolbarPlacement, ViewStyle};

use crate::generator::{
    colors,
    components::{render_children, render_node},
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
        features.uses_alignment || features.uses_column || features.uses_box,
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
    let default_alignment = match kind {
        // SwiftUI's VStack and ZStack center their children when alignment is omitted.
        // Spell that out in Compose, whose native defaults are Start and TopStart.
        LayoutKind::Column | LayoutKind::Stack => Some(Alignment::Center),
        LayoutKind::Row => None,
    };
    let alignment = style.alignment.or(default_alignment).map(|alignment| {
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

pub(crate) fn render_form_section(
    title: Option<&nexa_ir::Expr>,
    footer: Option<&nexa_ir::Expr>,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(
        depth,
        format_args!("Column(modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp)) {{"),
    );
    if let Some(title) = title {
        out.line_at(
            depth + 1,
            format_args!(
                "Text({}, modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp), style = androidx.compose.material3.MaterialTheme.typography.titleSmall)",
                crate::generator::engine::expressions::text_expression(title)
            ),
        );
    }
    for (index, child) in children.iter().enumerate() {
        out.line_at(
            depth + 1,
            format_args!("Box(modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp)) {{"),
        );
        render_node(child, scope.module, scope.features, depth + 2, out);
        out.line_at(depth + 1, format_args!("}}"));
        if index + 1 < children.len() {
            out.line_at(
                depth + 1,
                format_args!("HorizontalDivider(modifier = Modifier.padding(horizontal = 16.dp))"),
            );
        }
        if index + 1 < children.len() || footer.is_some() {
            out.push('\n');
        }
    }
    if let Some(footer) = footer {
        out.line_at(
            depth + 1,
            format_args!(
                "Text({}, modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp), style = androidx.compose.material3.MaterialTheme.typography.bodySmall)",
                crate::generator::engine::expressions::text_expression(footer)
            ),
        );
    }
    out.line_at(depth, format_args!("}}"));
}

pub(crate) fn render_form(
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(
        depth,
        format_args!(
            "Column(modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {{"
        ),
    );
    render_children(children, scope.module, scope.features, depth + 1, out);
    out.line_at(depth, format_args!("}}"));
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

#[cfg(test)]
mod tests {
    use nexa_codegen::SourceWriter;
    use nexa_ir::{
        Expr, KeyboardType, LayoutKind, Module, Node, TextStyle, ToolbarPlacement, ViewStyle,
    };

    use crate::generator::features::Features;

    use super::{RenderScope, render_form_section, render_layout};

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

        assert!(output.contains("TextField("));
        assert!(output.contains("modifier = Modifier.weight(1f)"));
        assert!(output.contains("Text(\"Trailing action\""));
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

        assert!(output.contains(
            "Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {"
        ));
        assert!(output.contains("Row(horizontalArrangement = Arrangement.spacedBy(8.dp)"));
        assert!(output.contains("Text(\"Priority\"") && output.contains("Text(\"Save\""));
        assert!(output.contains("Text(\"Task name\""));
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
            Node::Text {
                value: Expr::String("First setting".to_owned()),
                style: TextStyle::default(),
            },
            Node::Text {
                value: Expr::String("Second setting".to_owned()),
                style: TextStyle::default(),
            },
        ];
        let mut output = SourceWriter::new();

        render_form_section(None, None, &children, &scope, 0, &mut output);

        assert!(
            output.contains("Box(modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp))")
        );
        assert!(
            output.contains("HorizontalDivider(modifier = Modifier.padding(horizontal = 16.dp))")
        );
        assert_eq!(output.as_str().matches("HorizontalDivider(").count(), 1);
    }
}
