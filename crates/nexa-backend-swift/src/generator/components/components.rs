use nexa_codegen::SourceWriter;
use nexa_ir::{LayoutKind, Module, Node, ViewStyle, ViewTransition};

use super::RenderScope;
use super::dialogs;
use crate::generator::{
    accessibility, bottom_bar, colors, controls,
    engine::features::Features,
    expressions::text_expression,
    images, input, keyboard, layout, links, lists, navigation, refresh, sheets, system_icons,
    utils::{indent, number},
};

pub(crate) fn render_node(
    node: &Node,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let scope = RenderScope { module, features };
    match node {
        Node::StatusBar { .. }
        | Node::Direction { .. }
        | Node::OnAppear { .. }
        | Node::OnDisappear { .. }
        | Node::OnActive { .. }
        | Node::OnInactive { .. }
        | Node::OnBackground { .. } => {}
        Node::Layout {
            kind,
            spacing,
            style,
            children,
        } => layout::render_layout(*kind, *spacing, style, children, &scope, depth, out),
        Node::Text { value, style } => {
            out.text_at(depth, format_args!("Text({})", text_expression(value)));
            if let Some(color) = style.color {
                out.push_str(&format!(
                    "\n{}.foregroundStyle({})",
                    "    ".repeat(depth + 1),
                    colors::expression(color)
                ));
            }
            if let Some(font_size) = style.font_size {
                out.push_str(&format!(
                    "\n{}.font(.system(size: {}))",
                    "    ".repeat(depth + 1),
                    number(font_size)
                ));
            }
            if let Some(font_weight) = style.font_weight {
                out.push_str(&format!(
                    "\n{}.fontWeight({})",
                    "    ".repeat(depth + 1),
                    swift_font_weight(font_weight)
                ));
            }
            if let Some(line_limit) = style.line_limit {
                out.push_str(&format!(
                    "\n{}.lineLimit({line_limit})",
                    "    ".repeat(depth + 1)
                ));
            }
            if let Some(line_height) = style.line_height {
                out.push_str(&format!(
                    "\n{}.lineSpacing({})",
                    "    ".repeat(depth + 1),
                    number(line_height)
                ));
            }
            if let Some(letter_spacing) = style.letter_spacing {
                out.push_str(&format!(
                    "\n{}.tracking({})",
                    "    ".repeat(depth + 1),
                    number(letter_spacing)
                ));
            }
            if style.selectable {
                out.push_str(&format!(
                    "\n{}.textSelection(.enabled)",
                    "    ".repeat(depth + 1)
                ));
            }
            if let Some(padding) = style.padding {
                out.push_str(&format!(
                    "\n{}.padding({})",
                    "    ".repeat(depth + 1),
                    number(padding)
                ));
            }
            layout::append_visual_effects(out, depth, style.opacity, &style.effects);
        }
        Node::Spacer => out.line_at(depth, format_args!("Spacer()")),
        Node::Divider { color, thickness } => {
            out.line_at(depth, format_args!("Divider()"));
            out.push_str(&format!(
                "\n{}.overlay({})\n{}.frame(height: {})",
                "    ".repeat(depth + 1),
                colors::expression(*color),
                "    ".repeat(depth + 1),
                number(*thickness)
            ));
        }
        Node::Button {
            label,
            icon,
            loading,
            disabled,
            actions,
        } => {
            controls::render_button(
                label,
                icon.as_deref(),
                loading.as_ref(),
                disabled.as_ref(),
                actions,
                depth,
                out,
            );
        }
        Node::TextInput {
            state,
            placeholder,
            keyboard,
            secure,
            multiline,
            autofill,
            return_key,
            autocorrect,
            capitalization,
            focused,
            max_length,
            actions,
        } => input::render_text_input(
            input::TextInputProps {
                state,
                placeholder,
                keyboard: *keyboard,
                secure: *secure,
                multiline: *multiline,
                autofill: *autofill,
                return_key: *return_key,
                autocorrect: *autocorrect,
                capitalization: *capitalization,
                focused: focused.as_deref(),
                max_length: *max_length,
                actions,
            },
            depth,
            out,
        ),
        Node::Switch { state, label } => controls::render_switch(state, label, depth, out),
        Node::Slider {
            state,
            min,
            max,
            step,
            ..
        } => controls::render_slider(state, *min, *max, *step, depth, out),
        Node::ProgressBar { progress } => controls::render_progress_bar(progress, depth, out),
        Node::ProgressRing { progress } => controls::render_progress_ring(progress, depth, out),
        Node::SegmentedControl { items, state } => {
            controls::render_segmented_control(items, state, depth, out)
        }
        Node::Picker { items, state } => controls::render_picker(items, state, depth, out),
        Node::Image {
            source,
            description,
            scale,
            placeholder,
            shared_element,
        } => images::render_image(
            source,
            description,
            *scale,
            placeholder.as_deref(),
            shared_element.as_ref(),
            depth,
            out,
        ),
        Node::SystemIcon {
            icon,
            description,
            size,
            tint,
        } => system_icons::render(*icon, description, *size, *tint, depth, out),
        Node::LinearGradient {
            start_color,
            end_color,
            direction,
            height,
        } => {
            let (start, end) = match direction {
                nexa_ir::GradientDirection::TopToBottom => (".top", ".bottom"),
                nexa_ir::GradientDirection::BottomToTop => (".bottom", ".top"),
                nexa_ir::GradientDirection::LeadingToTrailing => (".leading", ".trailing"),
                nexa_ir::GradientDirection::TrailingToLeading => (".trailing", ".leading"),
            };
            out.line_at(
                depth,
                format_args!(
                    "LinearGradient(colors: [{}, {}], startPoint: {}, endPoint: {}).frame(maxWidth: .infinity).frame(height: {}).allowsHitTesting(false)",
                    crate::generator::engine::colors::expression(*start_color),
                    crate::generator::engine::colors::expression(*end_color),
                    start,
                    end,
                    crate::generator::engine::utils::number(*height),
                ),
            );
        }
        Node::Pressable { .. } => controls::render_pressable(node, module, features, depth, out),
        Node::NavigationStack { root, arguments } => {
            navigation::render_navigation_stack(module, *root, arguments, features, depth, out);
        }
        Node::NavigationLink {
            destination,
            arguments,
            guard,
            children,
        } => navigation::render_link(
            *destination,
            arguments,
            guard.as_ref(),
            children,
            &scope,
            depth,
            out,
        ),
        Node::NavigationBack { label } => navigation::render_back(label, depth, out),
        Node::Link { url, children } => {
            links::render_link(url, children, module, features, depth, out)
        }
        Node::Accessibility {
            label,
            hint,
            role,
            children,
        } => accessibility::render_accessibility(
            label,
            hint.as_ref(),
            *role,
            children,
            &scope,
            depth,
            out,
        ),
        Node::KeyboardAware { dismiss, children } => {
            keyboard::render_keyboard_aware(*dismiss, children, module, features, depth, out)
        }
        Node::BottomSheet {
            state,
            partial,
            children,
        } => sheets::render_bottom_sheet(state, *partial, children, module, features, depth, out),
        Node::Dialog {
            state,
            title,
            message,
            children,
        } => dialogs::render_dialog(state, title, message, children, &scope, depth, out),
        Node::RefreshControl {
            state,
            children,
            actions,
        } => {
            refresh::render_refresh_control(state, children, actions, module, features, depth, out)
        }
        Node::AppBottomBar { state, tabs } => {
            bottom_bar::render_app_bottom_bar(state, tabs, module, features, depth, out)
        }
        Node::FastList { plan } => {
            lists::render_virtualized_list(plan, module, features, depth, out);
        }
        Node::If {
            condition,
            then_body,
            else_body,
            transition,
        } => {
            let content_depth = depth + usize::from(transition.is_some());
            if transition.is_some() {
                out.line_at(depth, format_args!("Group {{"));
            }
            out.line_at(
                content_depth,
                format_args!(
                    "if {} {{",
                    crate::generator::engine::expressions::expression(condition)
                ),
            );
            render_children(then_body, module, features, content_depth + 1, out);
            if let Some(else_body) = else_body {
                out.push('\n');
                indent(out, content_depth);
                out.push_str("} else {\n");
                render_children(else_body, module, features, content_depth + 1, out);
            }
            out.push('\n');
            indent(out, content_depth);
            out.push('}');
            if let Some(transition) = transition {
                out.push('\n');
                indent(out, depth);
                out.push('}');
                out.push_str(&format!(
                    ".transition({}).animation(.default, value: {})",
                    swift_transition(*transition),
                    crate::generator::engine::expressions::expression(condition)
                ));
            }
        }
        Node::When {
            value,
            cases,
            else_body,
            transition,
        } => {
            let content_depth = depth + usize::from(transition.is_some());
            if transition.is_some() {
                out.line_at(depth, format_args!("Group {{"));
            }
            out.line_at(
                content_depth,
                format_args!(
                    "switch {} {{",
                    crate::generator::engine::expressions::expression(value)
                ),
            );
            for case in cases {
                out.line_at(
                    content_depth + 1,
                    format_args!(
                        "case {}:",
                        crate::generator::engine::expressions::expression(&case.value)
                    ),
                );
                render_children(&case.body, module, features, content_depth + 2, out);
                out.push('\n');
            }
            indent(out, content_depth + 1);
            out.push_str("default:\n");
            render_children(else_body, module, features, content_depth + 2, out);
            out.push('\n');
            indent(out, content_depth);
            out.push('}');
            if let Some(transition) = transition {
                out.push('\n');
                indent(out, depth);
                out.push('}');
                out.push_str(&format!(
                    ".transition({}).animation(.default, value: {})",
                    swift_transition(*transition),
                    crate::generator::engine::expressions::expression(value)
                ));
            }
        }
        Node::Content => {
            indent(out, depth);
            out.push_str("nexaContent()")
        }
        Node::ComponentCall {
            name,
            arguments,
            children,
        } => {
            out.text_at(
                depth,
                format_args!(
                    "{}({})",
                    nexa_codegen::names::component_name(name),
                    arguments
                        .iter()
                        .map(
                            |(_, argument)| crate::generator::engine::expressions::expression(
                                argument
                            )
                        )
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
            if let Some(children) = children {
                out.push_str(" {\n");
                render_children(children, module, features, depth + 1, out);
                out.push('\n');
                indent(out, depth);
                out.push('}');
            }
        }
        Node::NativeComponentCall {
            name,
            arguments,
            children,
            event_handlers,
            ..
        } => {
            indent(out, depth);
            let mut rendered_arguments = arguments
                .iter()
                .map(|(argument_name, argument)| {
                    format!(
                        "{argument_name}: {}",
                        crate::generator::engine::expressions::expression(argument)
                    )
                })
                .collect::<Vec<_>>();
            rendered_arguments.extend(event_handlers.iter().map(|handler| {
                format!(
                    "{}: {}",
                    handler.property,
                    controls::render_event_closure(&handler.parameters, &handler.actions, depth)
                )
            }));
            out.push_str(&format!("{}({})", name, rendered_arguments.join(", ")));
            if let Some(children) = children {
                out.push_str(" {\n");
                render_children(children, module, features, depth + 1, out);
                out.push('\n');
                indent(out, depth);
                out.push('}');
            }
        }
    }
}

fn swift_transition(transition: ViewTransition) -> &'static str {
    match transition {
        ViewTransition::Fade => ".opacity",
        ViewTransition::SlideFromBottom => ".move(edge: .bottom)",
        ViewTransition::SlideFromLeft => ".move(edge: .leading)",
        ViewTransition::SlideFromRight => ".move(edge: .trailing)",
        ViewTransition::Scale => ".scale",
    }
}

fn swift_font_weight(weight: nexa_ir::FontWeight) -> &'static str {
    match weight {
        nexa_ir::FontWeight::Normal => ".regular",
        nexa_ir::FontWeight::Medium => ".medium",
        nexa_ir::FontWeight::Semibold => ".semibold",
        nexa_ir::FontWeight::Bold => ".bold",
    }
}

pub(crate) fn render_children(
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    match children {
        [] => {
            indent(out, depth);
            out.push_str("EmptyView()");
        }
        [node] => render_node(node, module, features, depth, out),
        _ => layout::render_layout(
            LayoutKind::Column,
            0.0,
            &ViewStyle::default(),
            children,
            &RenderScope { module, features },
            depth,
            out,
        ),
    }
}
