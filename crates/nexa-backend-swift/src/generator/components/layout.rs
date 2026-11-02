use std::collections::HashSet;

use nexa_codegen::SourceWriter;
use nexa_ir::Expr;
use nexa_ir::{Alignment, Dimension, LayoutKind, Module, Node, ViewStyle};
use nexa_ir::{AnimationEasing, AnimationSpec};

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
    indent(out, depth);
    let (layout, alignment, text_alignment, frame_alignment) = match kind {
        LayoutKind::Column => (
            "NexaColumnPrimitive",
            match style.alignment.unwrap_or(Alignment::Center) {
                Alignment::Start => ".leading",
                Alignment::Center => ".center",
                Alignment::End => ".trailing",
            },
            match style.alignment.unwrap_or(Alignment::Center) {
                Alignment::Start => ".leading",
                Alignment::Center => ".center",
                Alignment::End => ".trailing",
            },
            match style.alignment.unwrap_or(Alignment::Center) {
                Alignment::Start => ".topLeading",
                Alignment::Center => ".top",
                Alignment::End => ".topTrailing",
            },
        ),
        LayoutKind::Row => (
            "NexaRowPrimitive",
            match style.alignment.unwrap_or(Alignment::Center) {
                Alignment::Start => ".top",
                Alignment::Center => ".center",
                Alignment::End => ".bottom",
            },
            ".center",
            match style.alignment.unwrap_or(Alignment::Center) {
                Alignment::Start => ".topLeading",
                Alignment::Center => ".leading",
                Alignment::End => ".bottomLeading",
            },
        ),
        LayoutKind::Stack => (
            "NexaStackPrimitive",
            match style.alignment.unwrap_or(Alignment::Center) {
                Alignment::Start => ".topLeading",
                Alignment::Center => ".center",
                Alignment::End => ".bottomTrailing",
            },
            ".center",
            match style.alignment.unwrap_or(Alignment::Center) {
                Alignment::Start => ".topLeading",
                Alignment::Center => ".center",
                Alignment::End => ".bottomTrailing",
            },
        ),
    };
    if matches!(kind, LayoutKind::Stack) {
        out.push_str(&format!("{layout}(alignment: {alignment}) {{\n"));
    } else if matches!(kind, LayoutKind::Column) {
        out.push_str(&format!(
            "{layout}(alignment: {alignment}, textAlignment: {text_alignment}, spacing: {}) {{\n",
            number(spacing)
        ));
    } else if let Some(horizontal_arrangement) = style.horizontal_arrangement {
        let arrangement = match horizontal_arrangement {
            nexa_ir::HorizontalArrangement::Start => ".start",
            nexa_ir::HorizontalArrangement::Center => ".center",
            nexa_ir::HorizontalArrangement::End => ".end",
        };
        out.push_str(&format!(
            "{layout}(alignment: {alignment}, spacing: {}, horizontalArrangement: {arrangement}) {{\n",
            number(spacing)
        ));
    } else {
        out.push_str(&format!(
            "{layout}(alignment: {alignment}, spacing: {}) {{\n",
            number(spacing)
        ));
    }
    let content_children = children
        .iter()
        .filter(|child| !matches!(child, Node::Toolbar { .. } | Node::BottomSheet { .. }))
        .collect::<Vec<_>>();
    let bottom_sheets = children
        .iter()
        .filter(|child| matches!(child, Node::BottomSheet { .. }))
        .collect::<Vec<_>>();
    let overlay_start = if matches!(kind, LayoutKind::Stack) {
        stack_overlay_start(&content_children, scope.module)
    } else {
        None
    };
    let base_child_count = overlay_start.unwrap_or(content_children.len());
    for (index, child) in content_children.iter().take(base_child_count).enumerate() {
        if matches!(kind, LayoutKind::Stack) {
            indent(out, depth + 1);
            out.push_str(&format!("NexaStackLayer(order: {index}) {{\n"));
            render_node(child, scope.module, scope.features, depth + 2, out);
            out.push('\n');
            indent(out, depth + 1);
            out.push('}');
        } else {
            render_node(child, scope.module, scope.features, depth + 1, out);
        }
        if index + 1 < content_children.len() {
            out.push('\n');
        }
    }
    out.push('\n');
    indent(out, depth);
    out.push('}');
    if let Some(overlay_start) = overlay_start {
        // SwiftUI sizes an overlay to the intrinsic size of its base view.
        // A UIKit-backed FastList can report a small intrinsic size, which
        // then pulls later Stack children (toolbars, tab bars, etc.) into the
        // middle of the screen. Expand the stack to its parent proposal before
        // attaching those overlays so they keep the full-screen layout.
        out.line_at(
            depth,
            format_args!(".frame(maxWidth: .infinity, maxHeight: .infinity)"),
        );
        for (index, child) in content_children.iter().enumerate().skip(overlay_start) {
            out.push('\n');
            out.line_at(depth, format_args!(".overlay(alignment: {alignment}) {{"));
            out.line_at(depth + 1, format_args!("NexaStackLayer(order: {index}) {{"));
            render_node(child, scope.module, scope.features, depth + 2, out);
            out.push('\n');
            out.line_at(depth + 1, format_args!("}}"));
            out.line_at(depth, format_args!("}}"));
        }
    }
    append_style(out, depth, style, frame_alignment);

    if !bottom_sheets.is_empty() {
        out.push('\n');
        out.line_at(depth, format_args!(".background {{"));
        out.line_at(depth + 1, format_args!("ZStack {{"));
        for (index, bottom_sheet) in bottom_sheets.iter().enumerate() {
            render_node(bottom_sheet, scope.module, scope.features, depth + 2, out);
            if index + 1 < bottom_sheets.len() {
                out.push('\n');
            }
        }
        out.push('\n');
        out.line_at(depth + 1, format_args!("}}"));
        out.line_at(depth, format_args!("}}"));
    }

    let toolbars = children
        .iter()
        .filter(|child| matches!(child, Node::Toolbar { .. }))
        .collect::<Vec<_>>();
    if !toolbars.is_empty() {
        out.push('\n');
        out.line_at(depth, format_args!(".toolbar {{"));
        for toolbar in toolbars {
            super::bottom_bar::render_navigation_toolbar_items(
                toolbar,
                scope.module,
                scope.features,
                depth + 1,
                out,
            );
        }
        out.line_at(depth, format_args!("}}"));
    }
}

/// A UIKit-backed FastList can cover the native hit-test surface of later
/// SwiftUI siblings even when the siblings draw above it in a ZStack. Keep
/// those later Stack layers as SwiftUI overlays so controls stay interactive.
fn stack_overlay_start(children: &[&Node], module: &Module) -> Option<usize> {
    let mut has_fast_list_below = false;
    for (index, child) in children.iter().enumerate() {
        if has_fast_list_below {
            return Some(index);
        }
        let mut visited_components = HashSet::new();
        has_fast_list_below |= node_contains_fast_list(child, module, &mut visited_components);
    }
    None
}

fn node_contains_fast_list(
    node: &Node,
    module: &Module,
    visited_components: &mut HashSet<String>,
) -> bool {
    let mut found = false;
    nexa_ir::walk::walk_ir(
        std::slice::from_ref(node),
        &mut |candidate| {
            if matches!(candidate, Node::FastList { .. }) {
                found = true;
            } else if let Node::ComponentCall { name, .. } = candidate
                && visited_components.insert(name.clone())
            {
                if let Some(component) = module.components.iter().find(|item| item.name == *name) {
                    found |=
                        node_list_contains_fast_list(&component.body, module, visited_components);
                }
                visited_components.remove(name);
            }
        },
        &mut |_| {},
    );
    found
}

fn node_list_contains_fast_list(
    nodes: &[Node],
    module: &Module,
    visited_components: &mut HashSet<String>,
) -> bool {
    let mut found = false;
    nexa_ir::walk::walk_ir(
        nodes,
        &mut |node| {
            if matches!(node, Node::FastList { .. }) {
                found = true;
            } else if let Node::ComponentCall { name, .. } = node
                && visited_components.insert(name.clone())
            {
                if let Some(component) = module.components.iter().find(|item| item.name == *name) {
                    found |=
                        node_list_contains_fast_list(&component.body, module, visited_components);
                }
                visited_components.remove(name);
            }
        },
        &mut |_| {},
    );
    found
}

pub(crate) struct FormSectionRenderOptions<'a> {
    pub(crate) title: Option<&'a Expr>,
    pub(crate) footer: Option<&'a Expr>,
    pub(crate) comment: Option<&'a str>,
    pub(crate) show_dividers: bool,
    pub(crate) is_first_section: bool,
    pub(crate) background: Option<nexa_ir::ColorValue>,
}

pub(crate) fn render_form_section(
    options: FormSectionRenderOptions<'_>,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(depth, format_args!("NexaFormSectionPrimitive("));
    out.line_at(
        depth + 1,
        format_args!(
            "background: {},",
            options
                .background
                .map(crate::generator::engine::colors::expression)
                .unwrap_or_else(|| "nil".to_owned())
        ),
    );
    out.line_at(
        depth + 1,
        format_args!("isFirstSection: {},", options.is_first_section),
    );
    out.line_at(depth + 1, format_args!("content: {{"));
    for (index, child) in children.iter().enumerate() {
        render_form_row(child, options.show_dividers, scope, depth + 2, out);
        if index + 1 < children.len() {
            out.push('\n');
        }
    }
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth + 1, format_args!("header: {{"));
    if let Some(title) = options.title {
        out.line_at(
            depth + 2,
            format_args!(
                "{}",
                crate::generator::expressions::localized_text_view(title, options.comment)
            ),
        );
    } else {
        out.line_at(depth + 2, format_args!("EmptyView()"));
    }
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth + 1, format_args!("footer: {{"));
    if let Some(footer) = options.footer {
        out.line_at(
            depth + 2,
            format_args!(
                "{}",
                crate::generator::expressions::localized_text_view(footer, options.comment)
            ),
        );
    } else {
        out.line_at(depth + 2, format_args!("EmptyView()"));
    }
    out.line_at(depth + 1, format_args!("}}"));
    out.line_at(depth, format_args!(")"));
}

pub(crate) fn render_form_row(
    node: &Node,
    show_dividers: bool,
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(
        depth,
        format_args!(
            "NexaFormRowPrimitive(minHeight: {}, horizontalInset: {}, showDividers: {}) {{",
            nexa_codegen::design_system::FORM_ROW_MIN_HEIGHT,
            nexa_codegen::design_system::FORM_ROW_HORIZONTAL_INSET,
            show_dividers,
        ),
    );
    render_node(node, scope.module, scope.features, depth + 1, out);
    out.line_at(depth, format_args!("}}"));
}

fn append_style(out: &mut SourceWriter, depth: usize, style: &ViewStyle, frame_alignment: &str) {
    append_size_modifiers(out, depth, style, frame_alignment);
    // SwiftUI frames can draw oversized children outside their reported bounds.
    // Compose measures layout children within the remaining constrained height.
    if style.max_height.is_some() {
        append_modifier(out, depth, "clipped()");
    }
    // Keep the animated measurement inside the fill and border modifiers.
    // This makes the visible surface follow the interpolated size, matching
    // Compose's animateContentSize modifier order.
    if let Some(animation) = style.animation {
        let value = match animation {
            AnimationSpec::Spring { response, damping } => {
                let physics = nexa_codegen::animation::spring_physics(response, damping);
                format!(
                    ".interpolatingSpring(mass: 1, stiffness: {}, damping: {}, initialVelocity: 0)",
                    number(physics.stiffness),
                    number(physics.damping_coefficient),
                )
            }
            AnimationSpec::Tween {
                duration,
                delay,
                easing,
            } => swift_tween(duration, easing, delay),
            // Keep SwiftUI's layout animation at Compose's shared 300 ms default.
            // Match Compose's Material easing curves rather than SwiftUI's
            // similarly named but different default curves.
            AnimationSpec::EaseIn => swift_tween(0.3, AnimationEasing::EaseIn, 0.0),
            AnimationSpec::EaseOut => swift_tween(0.3, AnimationEasing::EaseOut, 0.0),
            AnimationSpec::EaseInOut => swift_tween(0.3, AnimationEasing::EaseInOut, 0.0),
            AnimationSpec::Linear => swift_tween(0.3, AnimationEasing::Linear, 0.0),
        };
        append_modifier(
            out,
            depth,
            &format!("modifier(NexaAnimateContentSizeModifier(animation: {value}))"),
        );
    }
    if let Some(color) = style.background {
        append_modifier(
            out,
            depth,
            &format!("background({})", colors::expression(color)),
        );
    }
    if style.corner_radius.is_some()
        || style.corner_radius_expression.is_some()
        || style.corner_radius_bottom.is_some()
    {
        append_modifier(
            out,
            depth,
            &format!("clipShape({})", swift_corner_shape(style)),
        );
    }
    if let (Some(color), Some(width)) = (style.border_color, style.border_width) {
        append_modifier(
            out,
            depth,
            &format!(
                "overlay({}.stroke({}, lineWidth: {}))",
                swift_corner_shape(style),
                colors::expression(color),
                number(width)
            ),
        );
    }
    append_visual_effects(out, depth, style.opacity, &style.effects);
}

fn swift_corner_shape(style: &ViewStyle) -> String {
    let top = style
        .corner_radius_expression
        .as_ref()
        .map(|expression| crate::generator::engine::expressions::expression(expression))
        .or_else(|| style.corner_radius.map(number))
        .unwrap_or_else(|| "0".to_owned());
    if let Some(bottom) = style.corner_radius_bottom {
        return format!(
            "UnevenRoundedRectangle(cornerRadii: RectangleCornerRadii(topLeading: {top}, bottomLeading: {}, bottomTrailing: {}, topTrailing: {top}))",
            number(bottom),
            number(bottom),
        );
    }
    format!("RoundedRectangle(cornerRadius: {top})")
}

fn swift_tween(duration: f64, easing: AnimationEasing, delay: f64) -> String {
    let mut animation = match easing {
        AnimationEasing::Linear => format!(".linear(duration: {duration})"),
        AnimationEasing::EaseIn => {
            format!(".timingCurve(0.4, 0.0, 1.0, 1.0, duration: {duration})")
        }
        AnimationEasing::EaseOut => {
            format!(".timingCurve(0.0, 0.0, 0.2, 1.0, duration: {duration})")
        }
        AnimationEasing::EaseInOut => {
            format!(".timingCurve(0.4, 0.0, 0.2, 1.0, duration: {duration})")
        }
        AnimationEasing::CubicBezier { x1, y1, x2, y2 } => {
            format!(".timingCurve({x1}, {y1}, {x2}, {y2}, duration: {duration})")
        }
    };
    if delay > 0.0 {
        animation.push_str(&format!(".delay({delay})"));
    }
    animation
}

/// Keep explicit dimensions outside flexible frames so SwiftUI does not expand
/// a fixed size to the available `maxWidth` or `maxHeight` proposal.
pub(crate) fn append_size_modifiers(
    out: &mut SourceWriter,
    depth: usize,
    style: &ViewStyle,
    frame_alignment: &str,
) {
    let mut bounds = Vec::with_capacity(6);
    append_swift_dimension_bound(&mut bounds, "minWidth", style.min_width, true, true);
    append_swift_dimension_bound(&mut bounds, "maxWidth", style.max_width, true, false);
    append_swift_dimension_bound(&mut bounds, "minHeight", style.min_height, false, true);
    append_swift_dimension_bound(&mut bounds, "maxHeight", style.max_height, false, false);
    if matches!(style.width, Some(Dimension::Fill)) && style.min_width.is_none() {
        bounds.push("minWidth: 0".to_owned());
    }
    if matches!(style.width, Some(Dimension::Fill)) && style.max_width.is_none() {
        bounds.push("maxWidth: .infinity".to_owned());
    }
    if matches!(style.height, Some(Dimension::Fill)) && style.min_height.is_none() {
        bounds.push("minHeight: 0".to_owned());
    }
    if matches!(style.height, Some(Dimension::Fill)) && style.max_height.is_none() {
        bounds.push("maxHeight: .infinity".to_owned());
    }
    if !bounds.is_empty() {
        append_modifier(
            out,
            depth,
            &format!("frame({}, alignment: {frame_alignment})", bounds.join(", ")),
        );
    }
    let frame_width = style
        .width
        .and_then(|value| swift_dimension_value(value, true));
    let frame_height = style
        .height
        .and_then(|value| swift_dimension_value(value, false));
    if let (Some(width), Some(height)) = (frame_width.as_deref(), frame_height.as_deref()) {
        append_modifier(
            out,
            depth,
            &format!("frame(width: {width}, height: {height}, alignment: {frame_alignment})"),
        );
    } else if let Some(width) = frame_width.as_deref() {
        append_modifier(
            out,
            depth,
            &format!("frame(width: {width}, alignment: {frame_alignment})"),
        );
    } else if let Some(height) = frame_height.as_deref() {
        append_modifier(
            out,
            depth,
            &format!("frame(height: {height}, alignment: {frame_alignment})"),
        );
    }
    let relative_args = [
        ("width", style.width),
        ("height", style.height),
        ("minWidth", style.min_width),
        ("maxWidth", style.max_width),
        ("minHeight", style.min_height),
        ("maxHeight", style.max_height),
    ]
    .into_iter()
    .filter_map(|(name, dimension)| match dimension {
        Some(Dimension::ParentFraction(fraction)) => Some(format!("{name}: {}", number(fraction))),
        _ => None,
    })
    .collect::<Vec<_>>();
    if !relative_args.is_empty() {
        append_modifier(
            out,
            depth,
            &format!(
                "modifier(NexaParentRelativeDimensionModifier({}))",
                relative_args.join(", ")
            ),
        );
    }
}

fn append_swift_dimension_bound(
    bounds: &mut Vec<String>,
    name: &str,
    dimension: Option<Dimension>,
    width_axis: bool,
    minimum: bool,
) {
    let Some(dimension) = dimension else { return };
    if matches!(dimension, Dimension::ParentFraction(_)) {
        return;
    }
    let value = match dimension {
        Dimension::Fixed(value) => number(value),
        Dimension::Fill => {
            if minimum {
                "0".to_owned()
            } else {
                ".infinity".to_owned()
            }
        }
        Dimension::ScreenWidthFraction(fraction) if width_axis => format!(
            "nexaWindowMetrics.viewportSize.width * {}",
            number(fraction)
        ),
        Dimension::ScreenHeightFraction(fraction) if !width_axis => format!(
            "nexaWindowMetrics.viewportSize.height * {}",
            number(fraction)
        ),
        _ => return,
    };
    bounds.push(format!("{name}: {value}"));
}

fn swift_dimension_value(dimension: Dimension, width_axis: bool) -> Option<String> {
    match dimension {
        Dimension::Fixed(value) => Some(number(value)),
        Dimension::ScreenWidthFraction(fraction) if width_axis => Some(format!(
            "nexaWindowMetrics.viewportSize.width * {}",
            number(fraction)
        )),
        Dimension::ScreenHeightFraction(fraction) if !width_axis => Some(format!(
            "nexaWindowMetrics.viewportSize.height * {}",
            number(fraction)
        )),
        Dimension::Fill | Dimension::ParentFraction(_) => None,
        _ => None,
    }
}

pub(crate) fn append_visual_effects(
    out: &mut SourceWriter,
    depth: usize,
    opacity: Option<f32>,
    effects: &nexa_ir::ViewEffects,
) {
    if let Some(opacity) = opacity {
        append_modifier(
            out,
            depth,
            &format!("compositingGroup().opacity({})", number(opacity)),
        );
    }
    if let Some(opacity) = &effects.opacity_expression {
        append_modifier(
            out,
            depth,
            &format!(
                "compositingGroup().opacity(min(1.0, max(0.0, {})))",
                crate::generator::engine::expressions::expression(opacity)
            ),
        );
    }
    if effects.scale_y.is_some() || effects.scale_y_expression.is_some() {
        let scale_x = effects
            .scale_expression
            .as_ref()
            .map(|expression| crate::generator::engine::expressions::expression(expression))
            .or_else(|| effects.scale.map(number))
            .unwrap_or_else(|| "1".to_owned());
        let scale_y = effects
            .scale_y_expression
            .as_ref()
            .map(|expression| crate::generator::engine::expressions::expression(expression))
            .or_else(|| effects.scale_y.map(number))
            .or_else(|| {
                effects
                    .scale_expression
                    .as_ref()
                    .map(|expression| crate::generator::engine::expressions::expression(expression))
            })
            .or_else(|| effects.scale.map(number))
            .unwrap_or_else(|| "1".to_owned());
        append_modifier(
            out,
            depth,
            &format!("scaleEffect(x: {scale_x}, y: {scale_y})"),
        );
    } else if let Some(scale) = &effects.scale_expression {
        append_modifier(
            out,
            depth,
            &format!(
                "scaleEffect({})",
                crate::generator::engine::expressions::expression(scale)
            ),
        );
    } else if let Some(scale) = effects.scale {
        append_modifier(out, depth, &format!("scaleEffect({})", number(scale)));
    }
    if let Some(offset_y) = &effects.offset_y_expression {
        append_modifier(
            out,
            depth,
            &format!(
                "offset(y: {})",
                crate::generator::engine::expressions::expression(offset_y)
            ),
        );
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
    if let Some(glass) = &effects.glass {
        let shape_str = match glass.shape {
            nexa_ir::GlassShape::Circle => ".circle",
            nexa_ir::GlassShape::Capsule => ".capsule",
            nexa_ir::GlassShape::Rounded(r) => &format!(".rounded({})", r),
        };
        if let Some(glass_tint) = &glass.tint {
            append_modifier(
                out,
                depth,
                &format!(
                    "nexaGlass(tint: {}, shape: {})",
                    colors::expression(*glass_tint),
                    shape_str
                ),
            );
        } else {
            append_modifier(out, depth, &format!("nexaGlass(shape: {})", shape_str));
        }
    }
}

fn append_modifier(out: &mut SourceWriter, depth: usize, modifier: &str) {
    out.push('\n');
    indent(out, depth + 1);
    out.push('.');
    out.push_str(modifier);
}

#[cfg(test)]
mod tests {
    use super::{
        FormSectionRenderOptions, append_size_modifiers, append_visual_effects,
        render_form_section, render_layout,
    };
    use crate::generator::{components::RenderScope, features::Features};
    use nexa_codegen::SourceWriter;
    use nexa_ir::{
        AnimationSpec, Dimension, Expr, LayoutKind, Module, Node, TextStyle, ToolbarPlacement,
        ViewStyle,
    };

    #[test]
    fn screen_fraction_dimensions_use_the_edge_to_edge_viewport() {
        let style = ViewStyle {
            width: Some(Dimension::ScreenWidthFraction(0.9)),
            height: Some(Dimension::ScreenHeightFraction(0.12)),
            ..ViewStyle::default()
        };
        let mut output = SourceWriter::new();

        append_size_modifiers(&mut output, 0, &style, ".center");

        let generated = output.finish();
        assert!(
            generated.contains("nexaWindowMetrics.viewportSize.width * 0.9"),
            "{generated}"
        );
        assert!(
            generated.contains("nexaWindowMetrics.viewportSize.height * 0.12"),
            "{generated}"
        );
        assert!(!generated.contains("UIScreen.main.bounds"), "{generated}");
    }

    #[test]
    fn fixed_dimensions_are_not_expanded_by_flexible_bounds() {
        let style = ViewStyle {
            width: Some(Dimension::Fixed(240.0)),
            height: Some(Dimension::Fixed(72.0)),
            min_width: Some(Dimension::Fixed(220.0)),
            max_width: Some(Dimension::Fixed(260.0)),
            min_height: Some(Dimension::Fixed(64.0)),
            max_height: Some(Dimension::Fixed(80.0)),
            ..ViewStyle::default()
        };
        let mut output = SourceWriter::new();

        append_size_modifiers(&mut output, 0, &style, ".topLeading");

        let generated = output.finish();
        let bounds = generated
            .find(
                ".frame(minWidth: 220, maxWidth: 260, minHeight: 64, maxHeight: 80, alignment: .topLeading)",
            )
            .expect("emit min/max bounds");
        let fixed = generated
            .find(".frame(width: 240, height: 72, alignment: .topLeading)")
            .expect("emit explicit dimensions");
        assert!(bounds < fixed, "fixed dimensions must wrap flexible bounds");
    }

    #[test]
    fn size_frames_preserve_layout_alignment_and_column_top_origin() {
        let style = ViewStyle {
            width: Some(Dimension::Fixed(220.0)),
            max_height: Some(Dimension::Fixed(68.0)),
            ..ViewStyle::default()
        };
        let mut output = SourceWriter::new();

        append_size_modifiers(&mut output, 0, &style, ".topLeading");

        let generated = output.finish();
        assert!(generated.contains(
            ".frame(maxHeight: 68, alignment: .topLeading)\n    .frame(width: 220, alignment: .topLeading)"
        ));
    }

    #[test]
    fn bounded_layout_clips_children_after_sizing() {
        let style = ViewStyle {
            max_height: Some(Dimension::Fixed(68.0)),
            ..ViewStyle::default()
        };
        let mut output = SourceWriter::new();

        super::append_style(&mut output, 0, &style, ".top");

        let generated = output.finish();
        let frame = generated
            .find(".frame(maxHeight: 68, alignment: .top)")
            .expect("emit constrained frame");
        let clip = generated
            .find(".clipped()")
            .expect("clip overflowing children");
        assert!(frame < clip);
    }

    #[test]
    fn content_size_animation_wraps_size_before_background() {
        let style = ViewStyle {
            min_height: Some(Dimension::Fixed(36.0)),
            background: Some(nexa_ir::ColorValue::Static(nexa_ir::Color {
                red: 242,
                green: 242,
                blue: 247,
                alpha: 255,
            })),
            animation: Some(nexa_ir::AnimationSpec::EaseInOut),
            ..ViewStyle::default()
        };
        let mut output = SourceWriter::new();

        super::append_style(&mut output, 0, &style, ".center");

        let generated = output.finish();
        let minimum = generated.find(".frame(minHeight: 36").unwrap();
        let animation = generated
            .find(".modifier(NexaAnimateContentSizeModifier(animation: .timingCurve(0.4, 0.0, 0.2, 1.0, duration: 0.3)))")
            .unwrap();
        let background = generated.find(".background(").unwrap();
        assert!(minimum < animation && animation < background);
    }

    #[test]
    fn spring_content_animation_uses_shared_compose_physics() {
        let style = ViewStyle {
            animation: Some(AnimationSpec::Spring {
                response: 0.35,
                damping: 0.8,
            }),
            ..ViewStyle::default()
        };
        let mut output = SourceWriter::new();

        super::append_style(&mut output, 0, &style, ".center");

        assert!(output.finish().contains(
            ".interpolatingSpring(mass: 1, stiffness: 816.32654, damping: 45.714287, initialVelocity: 0)"
        ));
    }

    #[test]
    fn opacity_composites_the_complete_view_before_blending() {
        let mut output = SourceWriter::new();

        append_visual_effects(&mut output, 0, Some(0.45), &nexa_ir::ViewEffects::default());

        assert!(output.finish().contains("compositingGroup().opacity(0.45)"));
    }

    #[test]
    fn vertical_scale_overrides_only_the_vertical_transform_axis() {
        let effects = nexa_ir::ViewEffects {
            scale: Some(0.8),
            scale_y: Some(0.4),
            ..nexa_ir::ViewEffects::default()
        };
        let mut output = SourceWriter::new();

        append_visual_effects(&mut output, 0, None, &effects);

        assert!(output.finish().contains("scaleEffect(x: 0.8, y: 0.4)"));
    }

    #[test]
    fn animated_vertical_scale_reads_float_state_in_swiftui() {
        let effects = nexa_ir::ViewEffects {
            scale_y_expression: Some(Box::new(nexa_ir::Expr::AnimatedState(
                "squash".to_owned(),
                nexa_ir::Type::Numeric(nexa_ir::NumericType::Float64),
            ))),
            ..nexa_ir::ViewEffects::default()
        };
        let mut output = SourceWriter::new();

        append_visual_effects(&mut output, 0, None, &effects);

        let generated = output.finish();
        assert!(
            generated.contains("scaleEffect(x: 1, y: nexa_squash)"),
            "{generated}"
        );
    }

    #[test]
    fn layout_attaches_toolbar_to_the_container_instead_of_an_empty_child() {
        let module = Module {
            app_name: "ToolbarLayout".to_owned(),
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
        let children = [Node::Toolbar {
            placement: ToolbarPlacement::Trailing,
            children: vec![Node::Text {
                value: Expr::String("Toolbar action".to_owned()),
                style: TextStyle::default(),
            }],
        }];
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

        let generated = output.finish();
        assert!(generated.contains(
            "NexaColumnPrimitive(alignment: .center, textAlignment: .center, spacing: 0) {"
        ));
        assert!(generated.contains("ToolbarItem(placement: .topBarTrailing)"));
        assert!(!generated.contains("EmptyView()"));
    }

    #[test]
    fn bottom_sheets_attach_to_the_layout_background_without_spacing_gaps() {
        let module = Module {
            app_name: "BottomSheetLayout".to_owned(),
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
                value: Expr::String("Before sheet".to_owned()),
                style: TextStyle::default(),
            },
            Node::BottomSheet {
                component_ref: Expr::ComponentRefCreate {
                    namespace: "Nexa".to_owned(),
                    name: "BottomSheet".to_owned(),
                    identity: "sheet-ref".to_owned(),
                },
                content_sized: false,
                content_padding: 0.0,
                corner_radius: None,
                detents: Vec::new(),
                initial_detent: None,
                show_drag_indicator: true,
                allows_dismissal: true,
                allows_background_interaction: false,
                should_dismiss_on_back_press: true,
                background_color: None,
                scrim_color: None,
                on_dismiss: Vec::new(),
                children: Vec::new(),
            },
            Node::Text {
                value: Expr::String("After sheet".to_owned()),
                style: TextStyle::default(),
            },
        ];
        let mut output = SourceWriter::new();

        render_layout(
            LayoutKind::Column,
            12.0,
            &ViewStyle::default(),
            &children,
            &scope,
            0,
            &mut output,
        );

        let generated = output.finish();
        let background = generated
            .find(".background {\n    ZStack {")
            .expect("sheet anchors attach outside the spaced column content");
        let sheet = generated
            .find("NexaBottomSheetPrimitive(")
            .expect("sheet presentation anchor is still generated");
        assert!(!generated[..background].contains("NexaBottomSheetPrimitive("));
        assert!(sheet > background);
        assert!(generated[..background].contains("Before sheet"));
        assert!(generated[..background].contains("After sheet"));
    }

    #[test]
    fn stack_children_keep_later_overlays_above_native_list_hosts() {
        let module = Module {
            app_name: "StackHitTesting".to_owned(),
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
                value: Expr::String("Full-screen content".to_owned()),
                style: TextStyle::default(),
            },
            Node::Text {
                value: Expr::String("Overlay controls".to_owned()),
                style: TextStyle::default(),
            },
        ];
        let mut output = SourceWriter::new();

        render_layout(
            LayoutKind::Stack,
            0.0,
            &ViewStyle::default(),
            &children,
            &scope,
            0,
            &mut output,
        );

        let generated = output.finish();
        assert!(generated.contains("NexaStackLayer(order: 0) {"));
        assert!(generated.contains("NexaStackLayer(order: 1) {"));
        assert!(
            generated.find("NexaStackLayer(order: 0)").unwrap()
                < generated.find("NexaStackLayer(order: 1)").unwrap()
        );
    }

    #[test]
    fn form_section_uses_the_shared_row_minimum_and_vertical_insets() {
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
                value: Expr::String("Setting".to_owned()),
                style: TextStyle::default(),
            },
            Node::Button {
                label: Expr::String("Save settings".to_owned()),
                icon: None,
                loading: None,
                disabled: None,
                style: Some(nexa_ir::ButtonStyle::Plain),
                size: None,
                shape: None,
                tint: None,
                glass: false,
                accessibility_label: None,
                actions: Vec::new(),
            },
        ];
        let mut output = SourceWriter::new();

        render_form_section(
            FormSectionRenderOptions {
                title: None,
                footer: None,
                comment: None,
                show_dividers: true,
                is_first_section: false,
                background: None,
            },
            &children,
            &scope,
            0,
            &mut output,
        );

        let generated = output.finish();
        assert_eq!(
            generated
                .matches(
                    "NexaFormRowPrimitive(minHeight: 56, horizontalInset: 16, showDividers: true) {",
                )
                .count(),
            children.len()
        );
    }
}
