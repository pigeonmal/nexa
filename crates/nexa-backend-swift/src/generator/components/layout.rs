use std::collections::HashSet;

use nexa_codegen::SourceWriter;
use nexa_ir::Expr;
use nexa_ir::{Alignment, AnimationSpec, LayoutKind, Module, Node, ViewStyle};

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
    } else {
        out.push_str(&format!(
            "{layout}(alignment: {alignment}, spacing: {}) {{\n",
            number(spacing)
        ));
    }
    let content_children = children
        .iter()
        .filter(|child| !matches!(child, Node::Toolbar { .. }))
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

pub(crate) fn render_form_section(
    title: Option<&Expr>,
    footer: Option<&Expr>,
    comment: Option<&str>,
    children: &[Node],
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(depth, format_args!("NexaFormSectionPrimitive("));
    out.line_at(depth + 1, format_args!("content: {{"));
    for (index, child) in children.iter().enumerate() {
        render_form_row(child, scope, depth + 2, out);
        if index + 1 < children.len() {
            out.push('\n');
        }
    }
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth + 1, format_args!("header: {{"));
    if let Some(title) = title {
        out.line_at(
            depth + 2,
            format_args!(
                "{}",
                crate::generator::expressions::localized_text_view(title, comment)
            ),
        );
    } else {
        out.line_at(depth + 2, format_args!("EmptyView()"));
    }
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth + 1, format_args!("footer: {{"));
    if let Some(footer) = footer {
        out.line_at(
            depth + 2,
            format_args!(
                "{}",
                crate::generator::expressions::localized_text_view(footer, comment)
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
    scope: &RenderScope<'_>,
    depth: usize,
    out: &mut SourceWriter,
) {
    out.line_at(
        depth,
        format_args!(
            "NexaFormRowPrimitive(minHeight: {}, horizontalInset: {}) {{",
            nexa_codegen::design_system::FORM_ROW_MIN_HEIGHT,
            nexa_codegen::design_system::FORM_ROW_HORIZONTAL_INSET,
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

/// Keep explicit dimensions outside flexible frames so SwiftUI does not expand
/// a fixed size to the available `maxWidth` or `maxHeight` proposal.
pub(crate) fn append_size_modifiers(
    out: &mut SourceWriter,
    depth: usize,
    style: &ViewStyle,
    frame_alignment: &str,
) {
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
        append_modifier(
            out,
            depth,
            &format!("frame({}, alignment: {frame_alignment})", bounds.join(", ")),
        );
    }
    if let (Some(width), Some(height)) = (style.width, style.height) {
        append_modifier(
            out,
            depth,
            &format!(
                "frame(width: {}, height: {}, alignment: {frame_alignment})",
                number(width),
                number(height)
            ),
        );
    } else if let Some(width) = style.width {
        append_modifier(
            out,
            depth,
            &format!(
                "frame(width: {}, alignment: {frame_alignment})",
                number(width)
            ),
        );
    } else if let Some(height) = style.height {
        append_modifier(
            out,
            depth,
            &format!(
                "frame(height: {}, alignment: {frame_alignment})",
                number(height)
            ),
        );
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
    use super::{append_size_modifiers, append_visual_effects, render_form_section, render_layout};
    use crate::generator::{components::RenderScope, features::Features};
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, LayoutKind, Module, Node, TextStyle, ToolbarPlacement, ViewStyle};

    #[test]
    fn fixed_dimensions_are_not_expanded_by_flexible_bounds() {
        let style = ViewStyle {
            width: Some(240.0),
            height: Some(72.0),
            min_width: Some(220.0),
            max_width: Some(260.0),
            min_height: Some(64.0),
            max_height: Some(80.0),
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
            width: Some(220.0),
            max_height: Some(68.0),
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
    fn bounded_layout_clips_children_before_padding() {
        let style = ViewStyle {
            max_height: Some(68.0),
            padding: Some(6.0),
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
        let padding = generated
            .find(".padding(6)")
            .expect("preserve outer padding");
        assert!(frame < clip && clip < padding);
    }

    #[test]
    fn opacity_composites_the_complete_view_before_blending() {
        let mut output = SourceWriter::new();

        append_visual_effects(&mut output, 0, Some(0.45), &nexa_ir::ViewEffects::default());

        assert!(output.finish().contains("compositingGroup().opacity(0.45)"));
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
                actions: Vec::new(),
            },
        ];
        let mut output = SourceWriter::new();

        render_form_section(None, None, None, &children, &scope, 0, &mut output);

        let generated = output.finish();
        assert_eq!(
            generated
                .matches("NexaFormRowPrimitive(minHeight: 56, horizontalInset: 16) {")
                .count(),
            children.len()
        );
    }
}
