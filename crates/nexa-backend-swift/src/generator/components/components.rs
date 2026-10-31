use nexa_codegen::SourceWriter;
use nexa_ir::adaptive_layout::LayoutEdge;
use nexa_ir::{LayoutKind, Module, Node, ToolbarPlacement, ViewStyle};

use super::RenderScope;
use super::{adaptive_layout, conditional, content_unavailable, dialogs};
use crate::generator::components::split_view;
use crate::generator::{
    accessibility, bottom_bar, bottom_sheet, colors, controls,
    engine::features::Features,
    images, input, keyboard, layout, links, lists, navigation, refresh, system_icons,
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
        Node::SafeAreaPadding { plan, children } => {
            let edges = plan
                .edges
                .iter()
                .map(|edge| match edge {
                    LayoutEdge::Top => ".top",
                    LayoutEdge::Bottom => ".bottom",
                    LayoutEdge::Leading => ".leading",
                    LayoutEdge::Trailing => ".trailing",
                })
                .collect::<Vec<_>>()
                .join(", ");
            out.line_at(
                depth,
                format_args!(
                    "NexaSafeAreaPaddingPrimitive(edges: [{edges}], spacing: {}) {{",
                    number(plan.spacing)
                ),
            );
            render_children(children, module, features, depth + 1, out);
            out.line_at(depth, format_args!("}}"));
        }
        Node::SafeAreaInset {
            plan,
            content,
            children,
        } => {
            let edge = match plan.edge {
                LayoutEdge::Top => ".top",
                LayoutEdge::Bottom => ".bottom",
                LayoutEdge::Leading => ".leading",
                LayoutEdge::Trailing => ".trailing",
            };
            let accommodates_endpoint = matches!(
                plan.scroll_owner,
                nexa_ir::adaptive_layout::ScrollOwnerResolution::Unique(_)
                    | nexa_ir::adaptive_layout::ScrollOwnerResolution::RouteHost
            );
            out.line_at(
                depth,
                format_args!(
                    "NexaSafeAreaInsetPrimitive(edge: {edge}, spacing: {}, accommodatesEndpoint: {accommodates_endpoint}) {{",
                    number(plan.spacing)
                ),
            );
            render_children(children, module, features, depth + 1, out);
            out.line_at(depth, format_args!("}} attachedContent: {{"));
            render_children(content, module, features, depth + 1, out);
            out.line_at(depth, format_args!("}}"));
        }
        Node::ScrollHeader {
            plan,
            header,
            children,
        } => adaptive_layout::render_scroll_header(
            *plan, header, children, module, features, depth, out,
        ),
        Node::Appearance { mode, children } => {
            let mode = crate::generator::engine::expressions::expression(mode);
            out.line_at(
                depth,
                format_args!("NexaAppearancePrimitive(mode: {mode}) {{"),
            );
            render_children(children, module, features, depth + 1, out);
            out.line_at(depth, format_args!("}}"));
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
        Node::Form {
            show_dividers,
            background,
            children,
        } => {
            let background = background
                .map(crate::generator::engine::colors::expression)
                .unwrap_or_else(|| "nil".to_owned());
            out.line_at(
                depth,
                format_args!("NexaFormPrimitive(background: {background}) {{"),
            );
            for (index, child) in children.iter().enumerate() {
                match child {
                    Node::FormSection {
                        title,
                        footer,
                        show_dividers: section_show_dividers,
                        background,
                        children,
                    } => layout::render_form_section(
                        layout::FormSectionRenderOptions {
                            title: title.as_ref(),
                            footer: footer.as_ref(),
                            comment: None,
                            show_dividers: section_show_dividers.unwrap_or(*show_dividers),
                            background: *background,
                        },
                        children,
                        &scope,
                        depth + 1,
                        out,
                    ),
                    _ => layout::render_form_row(child, *show_dividers, &scope, depth + 1, out),
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
            show_dividers,
            background,
            children,
        } => layout::render_form_section(
            layout::FormSectionRenderOptions {
                title: title.as_ref(),
                footer: footer.as_ref(),
                comment: None,
                show_dividers: show_dividers.unwrap_or(true),
                background: *background,
            },
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
                ToolbarPlacement::Leading => ".topBarLeading",
                ToolbarPlacement::Trailing => ".topBarTrailing",
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
            title_color,
            description_color,
            icon_tint,
            background,
        } => content_unavailable::render(
            title,
            icon,
            description.as_ref(),
            content_unavailable::ColorOverrides {
                title: title_color.as_ref(),
                description: description_color.as_ref(),
                icon: icon_tint.as_ref(),
                background: background.as_ref(),
            },
            depth,
            out,
        ),
        Node::Spacer { width, height } => {
            let minimum = (*height).or(*width).unwrap_or(0.0);
            out.line_at(
                depth,
                format_args!("Spacer(minLength: {})", number(minimum)),
            );
            if width.is_some() || height.is_some() {
                let width = width.map_or_else(|| "nil".to_owned(), number);
                let height = height.map_or_else(|| "nil".to_owned(), number);
                out.push_str(&format!(".frame(width: {width}, height: {height})"));
            }
        }
        Node::Divider { color, thickness } => out.line_at(
            depth,
            format_args!(
                "NexaDividerPrimitive(color: {}, thickness: {})",
                colors::expression(*color),
                number(*thickness),
            ),
        ),
        Node::Shape {
            shape,
            width,
            height,
            fill,
            width_expression,
            height_expression,
            animation,
        } => {
            let shape = match shape {
                nexa_ir::ButtonShape::Capsule => "Capsule()".to_owned(),
                nexa_ir::ButtonShape::Circle => "Circle()".to_owned(),
                nexa_ir::ButtonShape::Rounded(radius) => {
                    format!("RoundedRectangle(cornerRadius: {})", number(*radius))
                }
            };
            let width_value = width_expression.as_ref().map_or_else(
                || number(*width),
                |expression| {
                    format!(
                        "max(0, CGFloat({}))",
                        crate::generator::engine::expressions::expression(expression)
                    )
                },
            );
            let height_value = height_expression.as_ref().map_or_else(
                || number(*height),
                |expression| {
                    format!(
                        "max(0, CGFloat({}))",
                        crate::generator::engine::expressions::expression(expression)
                    )
                },
            );
            out.line_at(
                depth,
                format_args!(
                    "{}.fill({}).frame(width: {}, height: {}){}",
                    shape,
                    colors::expression(*fill),
                    width_value,
                    height_value,
                    animation.map_or_else(String::new, |animation| {
                        let spec = controls::swift_animation(animation);
                        let width_animation =
                            width_expression
                                .as_ref()
                                .map_or_else(String::new, |expression| {
                                    format!(
                                        ".animation({spec}, value: {})",
                                        crate::generator::engine::expressions::expression(
                                            expression
                                        )
                                    )
                                });
                        let height_animation =
                            height_expression
                                .as_ref()
                                .map_or_else(String::new, |expression| {
                                    format!(
                                        ".animation({spec}, value: {})",
                                        crate::generator::engine::expressions::expression(
                                            expression
                                        )
                                    )
                                });
                        format!("{width_animation}{height_animation}")
                    }),
                ),
            );
        }
        Node::Button {
            label,
            accessibility_label,
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
                accessibility_label.as_deref(),
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
            font_size,
            font_family,
            text_color,
            placeholder_color,
            clearable,
            min_lines,
            max_lines,
            searchable,
            actions,
            on_focus,
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
                font_size: *font_size,
                font_family: font_family.as_deref(),
                text_color: *text_color,
                placeholder_color: *placeholder_color,
                clearable: *clearable,
                min_lines: *min_lines,
                max_lines: *max_lines,
                searchable: *searchable,
                horizontal_padding: None,
                actions,
                on_focus: on_focus.as_deref(),
                on_change: on_change.as_ref(),
            },
            depth,
            out,
        ),
        Node::Switch {
            state,
            label,
            on_change,
        } => controls::render_switch(state, label, on_change.as_ref(), depth, out),
        Node::Slider {
            state,
            min,
            max,
            step,
            tint,
            track_color,
            thumb_color,
            track_height,
            thumb_size,
            hit_slop,
            on_change,
            on_editing_changed,
            ..
        } => controls::render_slider(
            state,
            min,
            max,
            *step,
            tint.as_ref(),
            track_color.as_ref(),
            thumb_color.as_ref(),
            *track_height,
            *thumb_size,
            *hit_slop,
            on_change.as_ref(),
            on_editing_changed.as_ref(),
            depth,
            out,
        ),
        Node::ProgressBar { progress, style } => {
            controls::render_progress_bar(progress.as_ref(), style.as_deref(), depth, out)
        }
        Node::ProgressRing { progress, style } => {
            controls::render_progress_ring(progress.as_ref(), style.as_deref(), depth, out)
        }
        Node::SegmentedControl {
            items,
            state,
            animation,
            tint,
            background,
            text_color,
            selected_text_color,
            font_size,
            font_weight,
            height,
            corner_radius,
            border_color,
            border_width,
        } => controls::render_segmented_control(
            items,
            state,
            *animation,
            controls::SegmentedControlStyle {
                tint: tint.as_ref(),
                background: background.as_ref(),
                text_color: text_color.as_ref(),
                selected_text_color: selected_text_color.as_ref(),
                font_size: *font_size,
                font_weight: *font_weight,
                height: *height,
                corner_radius: *corner_radius,
                border_color: border_color.as_ref(),
                border_width: *border_width,
            },
            depth,
            out,
        ),
        Node::Picker {
            items,
            state,
            icon,
            item_icons,
            label,
            tint,
        } => controls::render_picker(
            items,
            state,
            controls::PickerOptions {
                icon: icon.as_ref(),
                item_icons,
                label: label.as_ref(),
                tint: tint.as_ref(),
                comment: None,
            },
            depth,
            out,
        ),
        Node::PickerDate { .. } => controls::render_picker_date(node, depth, out),
        Node::Image {
            source,
            description,
            scale,
            placeholder,
            show_loading_indicator,
            max_height,
            fill_max_size,
            shared_element,
        } => images::render_image(
            source,
            description,
            *scale,
            placeholder.as_deref(),
            *show_loading_indicator,
            *max_height,
            *fill_max_size,
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
        Node::FontIcon {
            font_family,
            glyph,
            description,
            size,
            tint,
        } => system_icons::render_font(font_family, glyph, description, size, tint, depth, out),
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
        Node::NavigationStack {
            root,
            arguments,
            show_navigation_bar,
            title_mode,
            title_alignment,
        } => {
            navigation::render_navigation_stack(
                module,
                *root,
                arguments,
                navigation::NavigationTitleOptions {
                    mode: *title_mode,
                    alignment: *title_alignment,
                    show_native_bar: *show_navigation_bar,
                },
                features,
                depth,
                out,
            );
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
        Node::KeyboardAware {
            dismiss,
            behavior,
            children,
        } => keyboard::render_keyboard_aware(
            *dismiss, *behavior, children, module, features, depth, out,
        ),
        Node::BottomSheet {
            component_ref,
            content_sized,
            content_padding,
            corner_radius,
            detents,
            initial_detent,
            show_drag_indicator,
            allows_dismissal,
            allows_background_interaction,
            should_dismiss_on_back_press,
            background_color,
            scrim_color,
            on_dismiss,
            children,
        } => bottom_sheet::render_bottom_sheet(
            bottom_sheet::BottomSheetRenderArgs {
                component_ref,
                content_sized: *content_sized,
                content_padding: *content_padding,
                corner_radius: *corner_radius,
                detents,
                initial_detent: *initial_detent,
                show_drag_indicator: *show_drag_indicator,
                allows_dismissal: *allows_dismissal,
                allows_background_interaction: *allows_background_interaction,
                should_dismiss_on_back_press: *should_dismiss_on_back_press,
                background_color: background_color.as_ref(),
                scrim_color: scrim_color.as_ref(),
                on_dismiss,
                children,
                module,
                features,
                depth,
            },
            out,
        ),
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
        Node::AppBottomBar {
            state,
            tint,
            tabs,
            custom_bar,
        } => bottom_bar::render_app_bottom_bar(
            state,
            tint.as_ref(),
            tabs,
            custom_bar.as_deref(),
            module,
            features,
            depth,
            out,
        ),
        Node::PagePager { state, pages } => {
            bottom_bar::render_page_pager(state, pages, depth, out);
        }
        Node::FastList { plan } => {
            if plan
                .on_end_reached()
                .is_some_and(nexa_ir::walk::actions_contain_task_launch)
            {
                out.line_at(depth, format_args!("NexaTaskScopeView {{ nexaTaskScope in"));
                lists::render_virtualized_list(plan, module, features, depth + 1, out);
                out.line_at(depth, format_args!("}}"));
            } else {
                lists::render_virtualized_list(plan, module, features, depth, out);
            }
        }
        Node::If {
            condition,
            then_body,
            else_body,
            transition,
            transition_animation,
        } => {
            conditional::render_if(
                condition,
                then_body,
                else_body.as_deref(),
                *transition,
                *transition_animation,
                conditional::ConditionalRenderContext {
                    module,
                    features,
                    depth,
                },
                out,
            );
        }
        Node::When {
            value,
            cases,
            else_body,
            transition,
            transition_animation,
        } => {
            conditional::render_when(
                value,
                cases,
                else_body.as_deref(),
                *transition,
                *transition_animation,
                conditional::ConditionalRenderContext {
                    module,
                    features,
                    depth,
                },
                out,
            );
        }
        Node::Content => {
            indent(out, depth);
            out.push_str("nexaContent()")
        }
        Node::ComponentCall {
            name,
            arguments,
            component_ref,
            children,
        } => {
            let mut rendered_arguments = arguments
                .iter()
                .map(|(_, argument)| crate::generator::engine::expressions::expression(argument))
                .collect::<Vec<_>>();
            if let Some(component_ref) = component_ref {
                rendered_arguments.push(format!(
                    "nexaComponentRef: {}",
                    crate::generator::engine::expressions::expression(component_ref)
                ));
            }
            out.text_at(
                depth,
                format_args!(
                    "{}({})",
                    nexa_codegen::names::component_name(name),
                    rendered_arguments.join(", ")
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
    let font_design = style.font_design.map(swift_text_font_design);
    let custom_font_weight = style
        .font_weight
        .or_else(|| {
            style
                .font_style
                .map(nexa_codegen::design_system::text_metrics)
                .map(|metrics| metrics.weight)
        })
        .unwrap_or(nexa_ir::FontWeight::Normal);
    let font = if let Some(family) = style.font_family.as_deref() {
        let face = nexa_codegen::names::font_face_name(family, custom_font_weight);
        let size = style
            .font_size
            .or_else(|| {
                style
                    .font_style
                    .map(nexa_codegen::design_system::text_metrics)
                    .map(|metrics| metrics.size as f32)
            })
            .unwrap_or(nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE as f32);
        let relative_to = style
            .font_style
            .map(swift_text_font_style)
            .unwrap_or(".body");
        format!(
            ".custom({}, size: {}, relativeTo: {relative_to})",
            crate::generator::engine::utils::swift_string(&face),
            number(size)
        )
    } else {
        match (style.font_size, style.font_style, font_design) {
            (Some(size), _, Some(design)) => {
                format!(".system(size: {}, design: {design})", number(size))
            }
            (Some(size), _, None) => format!(".system(size: {})", number(size)),
            (None, Some(font_style), Some(design)) => {
                format!(
                    ".system({}, design: {design})",
                    swift_text_font_style(font_style)
                )
            }
            (None, Some(font_style), None) => swift_text_font_style(font_style).to_owned(),
            (None, None, Some(design)) => format!(".system(.body, design: {design})"),
            (None, None, None) => String::new(),
        }
    };
    let base_font_size = style.font_size.unwrap_or_else(|| {
        style
            .font_style
            .map(|font_style| nexa_codegen::design_system::text_metrics(font_style).size as f32)
            .unwrap_or(nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE as f32)
    });
    let text = if style.reveal.is_some() {
        crate::generator::engine::expressions::expression(value)
    } else {
        crate::generator::expressions::localized_text_view(value, None)
    };
    let primitive = if style.reveal.is_some() {
        "NexaTextRevealPrimitive("
    } else {
        "NexaTextPrimitive("
    };
    out.line_at(depth, format_args!("{primitive}"));
    append_text_argument(out, depth + 1, format_args!("text: {text}"));
    if let Some(reveal) = &style.reveal {
        let expression = crate::generator::engine::expressions::expression;
        let segment_field = |name: &str| nexa_codegen::names::struct_field_name(name);
        append_text_argument(
            out,
            depth + 1,
            format_args!("segments: {}", expression(&reveal.segments)),
        );
        append_text_argument(
            out,
            depth + 1,
            format_args!("textOf: {{ $0.{} }}", segment_field("text")),
        );
        append_text_argument(
            out,
            depth + 1,
            format_args!(
                "startTimeOf: {{ Double($0.{}) }}",
                segment_field("startTimeSeconds")
            ),
        );
        append_text_argument(
            out,
            depth + 1,
            format_args!(
                "endTimeOf: {{ Double($0.{}) }}",
                segment_field("endTimeSeconds")
            ),
        );
        match &reveal.segment_type {
            nexa_ir::Type::Struct { fields, .. } => {
                match fields.iter().find(|(name, _)| name == "hasTrailingSpace") {
                    Some((name, nexa_ir::Type::Optional(_))) => append_text_argument(
                        out,
                        depth + 1,
                        format_args!(
                            "hasTrailingSpaceOf: {{ $0.{} ?? true }}",
                            segment_field(name)
                        ),
                    ),
                    Some((name, _)) => append_text_argument(
                        out,
                        depth + 1,
                        format_args!("hasTrailingSpaceOf: {{ $0.{} }}", segment_field(name)),
                    ),
                    None => append_text_argument(
                        out,
                        depth + 1,
                        format_args!("hasTrailingSpaceOf: {{ _ in true }}"),
                    ),
                }
            }
            _ => append_text_argument(
                out,
                depth + 1,
                format_args!("hasTrailingSpaceOf: {{ _ in true }}"),
            ),
        }
        let font_weight = style
            .font_weight
            .or_else(|| {
                style
                    .font_style
                    .map(nexa_codegen::design_system::text_metrics)
                    .map(|metrics| metrics.weight)
            })
            .unwrap_or(nexa_ir::FontWeight::Normal);
        let font_size = style
            .font_size
            .or_else(|| {
                style
                    .font_style
                    .map(nexa_codegen::design_system::text_metrics)
                    .map(|metrics| metrics.size as f32)
            })
            .unwrap_or(nexa_codegen::design_system::DEFAULT_BODY_FONT_SIZE as f32);
        let font_name = style
            .font_family
            .as_deref()
            .map(|family| {
                crate::generator::engine::utils::swift_string(&nexa_codegen::names::font_face_name(
                    family,
                    font_weight,
                ))
            })
            .unwrap_or_else(|| "nil".to_owned());
        let alignment = match style.alignment {
            Some(nexa_ir::TextAlignment::Center) => "Center",
            Some(nexa_ir::TextAlignment::Trailing) => "Trailing",
            Some(nexa_ir::TextAlignment::Leading) => "Leading",
            None => "Natural",
        };
        let design = match style.font_design {
            Some(nexa_ir::TextFontDesign::Serif) => "Serif",
            Some(nexa_ir::TextFontDesign::Rounded) => "Rounded",
            Some(nexa_ir::TextFontDesign::Monospaced) => "Monospaced",
            _ => "Default",
        };
        let color = style
            .color
            .map(colors::expression)
            .unwrap_or_else(|| "Color.primary".to_owned());
        let line_limit = style.line_limit.unwrap_or(0);
        append_text_argument(
            out,
            depth + 1,
            format_args!(
                "position: Double({}), active: {}, playing: {}, enabled: {}, seekRevision: Int64({}), dimOpacity: {}, leadTime: {}, targetOffset: {}, smoothingDuration: {}, fontName: {font_name}, fontSize: {}, fontWeight: {}, fontDesign: \"{design}\", color: UIColor({color}), alignment: \"{alignment}\", lineHeight: {}, letterSpacing: {}, maxLines: {line_limit}, italic: {}, underline: {}, strikethrough: {}",
                expression(&reveal.position),
                expression(&reveal.active),
                expression(&reveal.playing),
                expression(&reveal.enabled),
                expression(&reveal.seek_revision),
                number(reveal.dim_opacity),
                number(reveal.lead_time),
                number(reveal.target_offset),
                number(reveal.smoothing_duration),
                number(font_size),
                swift_font_weight(font_weight),
                number(style.line_height.unwrap_or(0.0)),
                number(style.letter_spacing.unwrap_or(0.0)),
                style.italic,
                style.underline,
                style.strikethrough,
            ),
        );
    }
    let mut style_modifiers = Vec::new();
    if let Some(alignment) = style.alignment {
        let alignment = match alignment {
            nexa_ir::TextAlignment::Leading => ".leading",
            nexa_ir::TextAlignment::Center => ".center",
            nexa_ir::TextAlignment::Trailing => ".trailing",
        };
        style_modifiers.push(format!(".withAlignment({alignment})"));
    }
    if let Some(color) = style.color {
        style_modifiers.push(format!(".withColor({})", colors::expression(color)));
    }
    let has_explicit_font = style.font_size.is_some()
        || style.font_style.is_some()
        || style.font_family.is_some()
        || font_design.is_some_and(|design| design != ".default");
    if has_explicit_font {
        style_modifiers.push(format!(".withFont({font})"));
    }
    let semantic_weight = if style.font_family.is_some() {
        style
            .font_style
            .map(nexa_codegen::design_system::text_metrics)
            .map(|metrics| metrics.weight)
            .filter(|weight| *weight != nexa_ir::FontWeight::Normal)
    } else {
        style
            .font_style
            .filter(|_| style.font_size.is_some())
            .map(nexa_codegen::design_system::text_metrics)
            .map(|metrics| metrics.weight)
            .filter(|weight| *weight != nexa_ir::FontWeight::Normal)
    };
    if let Some(weight) = style.font_weight.or(semantic_weight) {
        style_modifiers.push(format!(".withWeight({})", swift_font_weight(weight)));
    }
    if let Some(line_limit) = style.line_limit {
        style_modifiers.push(format!(".withLineLimit({line_limit})"));
    }
    if let Some(line_height) = style.line_height {
        let factor = number(line_height / base_font_size);
        style_modifiers.push(format!(
            ".withLineHeight(factor: {factor}, fontSize: {})",
            number(base_font_size)
        ));
    }
    if let Some(tracking) = style.letter_spacing.filter(|tracking| *tracking != 0.0) {
        style_modifiers.push(format!(".withTracking({})", number(tracking)));
    }
    if style.strikethrough {
        style_modifiers.push(".withStrikethrough()".to_owned());
    }
    if style.italic {
        style_modifiers.push(".withItalic()".to_owned());
    }
    if style.underline {
        style_modifiers.push(".withUnderline()".to_owned());
    }
    if style.selectable {
        style_modifiers.push(".withSelectable()".to_owned());
    }
    // `NexaTextRevealPrimitive` receives its supported text attributes as
    // typed arguments above; it has no `NexaTextStyle` wrapper parameter.
    if !style_modifiers.is_empty() && style.reveal.is_none() {
        append_text_argument(
            out,
            depth + 1,
            format_args!("style: NexaTextStyle(){}", style_modifiers.join("")),
        );
    }
    out.line_at(depth, format_args!(")"));
}

fn append_text_argument(out: &mut SourceWriter, depth: usize, argument: std::fmt::Arguments<'_>) {
    out.line_at(depth, format_args!("{argument},"));
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

fn swift_text_font_design(design: nexa_ir::TextFontDesign) -> &'static str {
    match design {
        nexa_ir::TextFontDesign::Default => ".default",
        nexa_ir::TextFontDesign::Serif => ".serif",
        nexa_ir::TextFontDesign::Rounded => ".rounded",
        nexa_ir::TextFontDesign::Monospaced => ".monospaced",
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

#[cfg(test)]
mod text_tests {
    use super::render_text;
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Expr, TextFontStyle, TextStyle};

    #[test]
    fn explicit_text_line_height_is_passed_as_a_font_relative_factor() {
        let mut output = SourceWriter::new();

        render_text(
            &Expr::String("Two lines".to_owned()),
            &TextStyle {
                font_size: Some(16.0),
                line_height: Some(22.0),
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        assert!(output.contains(".withLineHeight(factor: 1.375, fontSize: 16)"));
        assert!(!output.contains("lineSpacing:"));
    }

    #[test]
    fn semantic_text_line_height_uses_the_shared_font_metrics() {
        let mut output = SourceWriter::new();

        render_text(
            &Expr::String("Body text".to_owned()),
            &TextStyle {
                font_style: Some(TextFontStyle::Body),
                line_height: Some(22.0),
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        assert!(output.contains(".withLineHeight(factor: 1.2941177, fontSize: 17)"));
    }

    #[test]
    fn text_emphasis_is_passed_to_the_shared_native_primitive() {
        let mut output = SourceWriter::new();
        render_text(
            &Expr::String("Emphasis".to_owned()),
            &TextStyle {
                strikethrough: true,
                italic: true,
                underline: true,
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        let generated = output.finish();
        assert!(generated.contains(".withStrikethrough()"));
        assert!(generated.contains(".withItalic()"));
        assert!(generated.contains(".withUnderline()"));
    }

    #[test]
    fn default_text_style_uses_shared_native_primitive_defaults() {
        let mut output = SourceWriter::new();
        render_text(
            &Expr::String("Default".to_owned()),
            &TextStyle::default(),
            0,
            &mut output,
        );

        let generated = output.finish();
        assert!(
            generated.contains("NexaTextPrimitive(\n    text: Text(\"Default\"),\n)"),
            "{generated:?}"
        );
        assert!(!generated.contains("alignment:"));
        assert!(!generated.contains("font:"));
        assert!(!generated.contains("style:"));
    }

    #[test]
    fn explicit_text_parameters_override_shared_native_primitive_defaults() {
        let mut output = SourceWriter::new();
        render_text(
            &Expr::String("Title".to_owned()),
            &TextStyle {
                font_size: Some(23.0),
                font_weight: Some(nexa_ir::FontWeight::Bold),
                line_limit: Some(2),
                line_height: Some(30.0),
                letter_spacing: Some(0.5),
                strikethrough: true,
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        let generated = output.finish();
        assert!(generated.contains(".withFont(.system(size: 23))"));
        assert!(generated.contains(".withWeight(.bold)"));
        assert!(generated.contains(".withLineLimit(2)"));
        assert!(generated.contains(".withLineHeight(factor: 1.3043479, fontSize: 23)"));
        assert!(generated.contains(".withTracking(0.5)"));
        assert!(generated.contains(".withStrikethrough()"));
        assert!(!generated.contains(".withItalic()"));
    }

    #[test]
    fn explicit_font_size_keeps_semantic_font_style_weight() {
        let mut output = SourceWriter::new();
        render_text(
            &Expr::String("Headline".to_owned()),
            &TextStyle {
                font_size: Some(24.0),
                font_style: Some(TextFontStyle::Headline),
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        assert!(output.finish().contains(".withWeight(.semibold)"));
    }

    #[test]
    fn explicit_font_weight_overrides_semantic_font_style_weight() {
        let mut output = SourceWriter::new();
        render_text(
            &Expr::String("Regular headline".to_owned()),
            &TextStyle {
                font_size: Some(24.0),
                font_style: Some(TextFontStyle::Headline),
                font_weight: Some(nexa_ir::FontWeight::Normal),
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        assert!(output.finish().contains(".withWeight(.regular)"));
    }

    #[test]
    fn text_font_design_is_passed_to_the_shared_native_primitive() {
        let mut output = SourceWriter::new();
        render_text(
            &Expr::String("Monospaced".to_owned()),
            &TextStyle {
                font_design: Some(nexa_ir::TextFontDesign::Monospaced),
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        assert!(
            output
                .finish()
                .contains(".withFont(.system(.body, design: .monospaced))")
        );
    }

    #[test]
    fn custom_text_font_family_uses_registered_family_with_dynamic_type_role() {
        let mut output = SourceWriter::new();
        render_text(
            &Expr::String("Mazica".to_owned()),
            &TextStyle {
                font_family: Some("Gilroy-Regular".to_owned()),
                font_style: Some(nexa_ir::TextFontStyle::Headline),
                ..TextStyle::default()
            },
            0,
            &mut output,
        );

        let generated = output.finish();
        assert!(
            generated.contains(".custom(\"Gilroy-Semibold\", size: 17, relativeTo: .headline)")
        );
        assert!(generated.contains(".withFont("));
    }
}
