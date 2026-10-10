use nexa_codegen::SourceWriter;
use nexa_ir::{LayoutKind, Module, Node, ToolbarPlacement, ViewStyle, ViewTransition};

use super::RenderScope;
use super::{content_unavailable, dialogs};
use crate::generator::components::split_view;
use crate::generator::{
    accessibility, bottom_bar, colors, controls,
    engine::features::Features,
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
        Node::Appearance { mode, children } => {
            out.line_at(depth, format_args!("Group {{"));
            render_children(children, module, features, depth + 1, out);
            out.line_at(depth, format_args!("}}"));
            let mode = crate::generator::engine::expressions::expression(mode);
            out.line_at(depth, format_args!(".preferredColorScheme({mode} == \"dark\" ? .dark : {mode} == \"light\" ? .light : nil)"));
        }
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
        Node::Form { children } => {
            out.line_at(depth, format_args!("NexaFormPrimitive {{"));
            for (index, child) in children.iter().enumerate() {
                if matches!(child, Node::FormSection { .. }) {
                    render_node(child, module, features, depth + 1, out);
                } else {
                    layout::render_form_row(child, &scope, depth + 1, out);
                }
                if index + 1 < children.len() {
                    out.push('\n');
                }
            }
            out.line_at(depth, format_args!("}}"));
        }
        Node::FormSection {
            title,
            footer,
            children,
        } => layout::render_form_section(
            title.as_ref(),
            footer.as_ref(),
            None,
            children,
            &scope,
            depth,
            out,
        ),
        Node::Toolbar {
            placement,
            children,
        } => {
            let placement = match placement {
                ToolbarPlacement::Leading => ".navigationBarLeading",
                ToolbarPlacement::Trailing => ".navigationBarTrailing",
            };
            out.line_at(depth, format_args!("EmptyView()"));
            out.line_at(depth, format_args!(".toolbar {{"));
            for child in children {
                out.line_at(
                    depth + 1,
                    format_args!("ToolbarItem(placement: {placement}) {{"),
                );
                render_node(child, module, features, depth + 2, out);
                out.line_at(depth + 1, format_args!("}}"));
            }
            out.line_at(depth, format_args!("}}"));
        }
        Node::Text { value, style } => {
            render_text(value, style, depth, out);
            if let Some(padding) = style.padding {
                out.push_str(&format!(
                    "\n{}.padding({})",
                    "    ".repeat(depth + 1),
                    number(padding)
                ));
            }
            layout::append_visual_effects(out, depth, style.opacity, &style.effects);
        }
        Node::ContentUnavailable {
            title,
            icon,
            description,
        } => content_unavailable::render(title, icon, description, depth, out),
        Node::Spacer => out.line_at(depth, format_args!("Spacer()")),
        Node::Divider { color, thickness } => out.line_at(
            depth,
            format_args!(
                "NexaDividerPrimitive(color: {}, thickness: {})",
                colors::expression(*color),
                number(*thickness),
            ),
        ),
        Node::Button {
            label,
            icon,
            loading,
            disabled,
            style,
            size,
            shape,
            tint,
            glass,
            actions,
            ..
        } => {
            controls::render_button(
                label,
                None,
                icon.as_ref(),
                loading.as_ref(),
                disabled.as_ref(),
                *style,
                *size,
                *shape,
                tint.as_ref(),
                *glass,
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
            font,
            min_lines,
            max_lines,
            searchable,
            actions,
            on_change,
            comment,
            ..
        } => input::render_text_input(
            input::TextInputProps {
                state,
                placeholder,
                comment: comment.as_deref(),
                keyboard: *keyboard,
                secure: *secure,
                multiline: *multiline,
                autofill: *autofill,
                return_key: *return_key,
                autocorrect: *autocorrect,
                capitalization: *capitalization,
                focused: focused.as_deref(),
                max_length: *max_length,
                font: *font,
                min_lines: *min_lines,
                max_lines: *max_lines,
                searchable: *searchable,
                horizontal_padding: None,
                actions,
                on_change: on_change.as_ref(),
            },
            depth,
            out,
        ),
        Node::Switch { state, label } => controls::render_switch(state, label, None, depth, out),
        Node::Slider {
            state,
            min,
            max,
            step,
            ..
        } => controls::render_slider(state, *min, *max, *step, depth, out),
        Node::ProgressBar { progress } => controls::render_progress_bar(progress, depth, out),
        Node::ProgressRing { progress } => controls::render_progress_ring(progress, depth, out),
        Node::SegmentedControl { items, state, .. } => {
            controls::render_segmented_control(items, state, depth, out)
        }
        Node::Picker {
            items,
            state,
            icon,
            label,
            tint,
        } => controls::render_picker(
            items,
            state,
            controls::PickerOptions {
                icon: icon.as_ref(),
                label: label.as_ref(),
                tint: tint.as_ref(),
                comment: None,
            },
            depth,
            out,
        ),
        Node::DatePicker {
            timestamp_state,
            has_time_state,
        } => controls::render_date_picker(timestamp_state, has_time_state, depth, out),
        Node::Image {
            source,
            description,
            scale,
            placeholder,
            max_height,
            shared_element,
        } => images::render_image(
            source,
            description,
            *scale,
            placeholder.as_deref(),
            *max_height,
            shared_element.as_ref(),
            depth,
            out,
        ),
        Node::SystemIcon {
            icon,
            description,
            size,
            tint,
        } => system_icons::render(icon, description, *size, tint, depth, out),
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
                    "NexaLinearGradientPrimitive(colors: [{}, {}], startPoint: {}, endPoint: {}, height: {})",
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
        Node::NavigationSplitView {
            detail_visible,
            sidebar,
            detail,
        } => {
            split_view::render(detail_visible, sidebar, detail, &scope, depth, out);
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
        Node::NavigationBack { label } => navigation::render_back(label, None, depth, out),
        Node::Link { url, children } => {
            links::render_link(url, children, module, features, depth, out)
        }
        Node::Accessibility {
            label,
            hint,
            value,
            role,
            children,
        } => accessibility::render_accessibility(
            label,
            hint.as_ref(),
            value.as_ref(),
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
            large_only,
            ..
        } => sheets::render_bottom_sheet(state, *partial, *large_only, depth, out),
        Node::Dialog {
            state,
            title,
            message,
            children,
        } => dialogs::render_dialog(state, title, message, None, children, &scope, depth, out),
        Node::ConfirmationDialog {
            state,
            title,
            children,
        } => dialogs::render_confirmation_dialog(state, title, children, &scope, depth, out),
        Node::RefreshControl {
            state,
            children,
            actions,
        } => {
            refresh::render_refresh_control(state, children, actions, module, features, depth, out)
        }
        Node::AppBottomBar { state, tint, tabs } => bottom_bar::render_app_bottom_bar(
            state,
            tint.as_ref(),
            tabs,
            module,
            features,
            depth,
            out,
        ),
        Node::PagePager { state, pages } => {
            bottom_bar::render_page_pager(state, pages, depth, out);
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

fn render_text(
    value: &nexa_ir::Expr,
    style: &nexa_ir::TextStyle,
    depth: usize,
    out: &mut SourceWriter,
) {
    let alignment = style
        .alignment
        .map(|alignment| match alignment {
            nexa_ir::TextAlignment::Leading => ".leading",
            nexa_ir::TextAlignment::Center => ".center",
            nexa_ir::TextAlignment::Trailing => ".trailing",
        })
        .unwrap_or("nil");
    let font = if let Some(size) = style.font_size {
        format!(".system(size: {})", number(size))
    } else {
        style
            .font_style
            .map(swift_text_font_style)
            .unwrap_or("nil")
            .to_owned()
    };
    let color = style
        .color
        .map(colors::expression)
        .unwrap_or_else(|| "nil".to_owned());
    let weight = style.font_weight.map(swift_font_weight).unwrap_or("nil");
    let line_limit = style
        .line_limit
        .map(|value| value.to_string())
        .unwrap_or("nil".to_owned());
    let line_spacing = style.line_height.map(number).unwrap_or("nil".to_owned());
    let tracking = style.letter_spacing.map(number).unwrap_or("nil".to_owned());
    let text = crate::generator::expressions::localized_text_view(value, None);

    out.line_at(depth, format_args!("NexaTextPrimitive("));
    out.line_at(depth + 1, format_args!("text: {text},"));
    out.line_at(depth + 1, format_args!("alignment: {alignment},"));
    out.line_at(depth + 1, format_args!("color: {color},"));
    out.line_at(depth + 1, format_args!("font: {font},"));
    out.line_at(depth + 1, format_args!("weight: {weight},"));
    out.line_at(depth + 1, format_args!("lineLimit: {line_limit},"));
    out.line_at(depth + 1, format_args!("lineSpacing: {line_spacing},"));
    out.line_at(depth + 1, format_args!("tracking: {tracking},"));
    out.line_at(
        depth + 1,
        format_args!("strikethrough: {},", style.strikethrough),
    );
    out.line_at(depth + 1, format_args!("selectable: {}", style.selectable));
    out.line_at(depth, format_args!(")"));
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

fn swift_text_font_style(style: nexa_ir::TextFontStyle) -> &'static str {
    match style {
        nexa_ir::TextFontStyle::LargeTitle => ".largeTitle",
        nexa_ir::TextFontStyle::Title => ".title",
        nexa_ir::TextFontStyle::Title2 => ".title2",
        nexa_ir::TextFontStyle::Title3 => ".title3",
        nexa_ir::TextFontStyle::Headline => ".headline",
        nexa_ir::TextFontStyle::Subheadline => ".subheadline",
        nexa_ir::TextFontStyle::Body => ".body",
        nexa_ir::TextFontStyle::Callout => ".callout",
        nexa_ir::TextFontStyle::Footnote => ".footnote",
        nexa_ir::TextFontStyle::Caption => ".caption",
        nexa_ir::TextFontStyle::Caption2 => ".caption2",
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
