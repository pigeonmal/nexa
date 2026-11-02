use nexa_codegen::SourceWriter;
use nexa_ir::adaptive_layout::{LayoutEdge, ScrollOwnerResolution};
use nexa_ir::{
    Dimension, EdgeInsets, Expr, LayoutKind, Module, Node, SystemIcon, ToolbarPlacement, ViewStyle,
};

use crate::generator::{
    accessibility, bottom_bar, bottom_sheet, controls, dialogs, features::Features, images, input,
    keyboard, layout, links, lists, navigation, refresh, system_icons, utils::indent,
};

use super::conditional;
use super::{RenderScope, adaptive_layout, appearance};
use crate::generator::components::split_view;
use crate::generator::components::text;

pub(crate) fn render_node(
    node: &Node,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let scope = RenderScope { module, features };
    match node {
        Node::ViewInsets {
            padding,
            margin,
            child,
        } => {
            if let Node::Form {
                show_dividers,
                background,
                children,
            } = child.as_ref()
            {
                let content_padding = padding.as_ref().map(kotlin_padding_values);
                let margin = margin
                    .as_ref()
                    .map(kotlin_padding_values)
                    .map(|values| format!("Modifier.padding({values})"));
                layout::render_form(
                    *show_dividers,
                    *background,
                    children,
                    Some(layout::FormInsets {
                        content_padding,
                        margin,
                    }),
                    &scope,
                    depth,
                    out,
                );
            } else {
                let padding = kotlin_insets("padding", padding.as_ref());
                let margin = kotlin_insets("margin", margin.as_ref());
                let scroll_content = matches!(
                    child.as_ref(),
                    Node::FastList { .. }
                        | Node::KeyboardAware { .. }
                        | Node::RefreshControl { .. }
                );
                out.line_at(
                    depth,
                    format_args!("NexaViewInsetsPrimitive({padding}, {margin}, scrollContent = {scroll_content}) {{"),
                );
                render_node(child, module, features, depth + 1, out);
                out.line_at(depth, format_args!("}}"));
            }
        }
        Node::SafeAreaInset {
            plan,
            content,
            children,
        } => {
            let edge = match plan.edge {
                LayoutEdge::Top => "NexaSafeAreaEdge.Top",
                LayoutEdge::Bottom => "NexaSafeAreaEdge.Bottom",
                LayoutEdge::Leading => "NexaSafeAreaEdge.Leading",
                LayoutEdge::Trailing => "NexaSafeAreaEdge.Trailing",
            };
            let accommodates_endpoint = matches!(
                plan.scroll_owner,
                ScrollOwnerResolution::Unique(_) | ScrollOwnerResolution::RouteHost
            );
            out.line_at(
                depth,
                format_args!(
                    "NexaSafeAreaInsetPrimitive(edge = {edge}, accommodatesEndpoint = {accommodates_endpoint}, content = {{"
                ),
            );
            render_children(children, module, features, depth + 1, out);
            out.line_at(depth, format_args!("}}, attachedContent = {{"));
            render_children(content, module, features, depth + 1, out);
            out.line_at(depth, format_args!("}})"));
        }
        Node::ScrollHeader {
            plan,
            header,
            children,
        } => adaptive_layout::render_scroll_header(
            *plan, header, children, module, features, depth, out,
        ),
        Node::Appearance { mode, children } => {
            appearance::render_theme_start(mode, depth, out);
            render_children(children, module, features, depth + 2, out);
            appearance::render_theme_end(depth, out);
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
            gap,
            style,
            children,
        } => layout::render_layout(*kind, *gap, style, children, &scope, depth, out),
        Node::Form {
            show_dividers,
            background,
            children,
        } => layout::render_form(
            *show_dividers,
            *background,
            children,
            None,
            &scope,
            depth,
            out,
        ),
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
            let horizontal = match placement {
                ToolbarPlacement::Leading => "Start",
                ToolbarPlacement::Trailing => "End",
            };
            out.line_at(depth, format_args!("Row(horizontalArrangement = Arrangement.{horizontal}, verticalAlignment = Alignment.CenterVertically) {{"));
            render_children(children, module, features, depth + 1, out);
            out.line_at(depth, format_args!("}}"));
        }
        Node::Text { value, style, .. } => text::render(value, style, depth, out),
        Node::ContentUnavailable {
            title,
            icon,
            description,
            title_color,
            description_color,
            icon_tint,
            background,
        } => {
            let icon_description = match icon {
                SystemIcon::Shared(name) => name
                    .split('_')
                    .filter(|part| !part.is_empty())
                    .map(|part| {
                        let mut chars = part.chars();
                        chars
                            .next()
                            .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
                SystemIcon::SfSymbol(_)
                | SystemIcon::MaterialSymbol(_)
                | SystemIcon::FontGlyph { .. } => "Icon".to_owned(),
            };
            out.line_at(
                depth,
                format_args!(
                    "NexaContentUnavailablePrimitive(title = {}, description = {}, icon = {}, iconDescription = {}, titleColor = {}, descriptionColor = {}, iconTint = {}, background = {})",
                    crate::generator::engine::expressions::text_expression(title),
                    description.as_ref().map(crate::generator::engine::expressions::text_expression).unwrap_or_else(|| "null".to_owned()),
                    system_icons::spec(icon),
                    crate::generator::utils::kotlin_string(&icon_description),
                    title_color.as_ref().map(crate::generator::colors::expression_for_color).unwrap_or_else(|| "null".to_owned()),
                    description_color.as_ref().map(crate::generator::colors::expression_for_color).unwrap_or_else(|| "null".to_owned()),
                    icon_tint.as_ref().map(crate::generator::colors::expression_for_color).unwrap_or_else(|| "null".to_owned()),
                    background.as_ref().map(crate::generator::colors::expression_for_color).unwrap_or_else(|| "null".to_owned()),
                ),
            );
        }
        Node::Spacer { width, height } => {
            let mut modifiers = Vec::new();
            let mut relative = Vec::new();
            for (axis, dimension) in [("width", *width), ("height", *height)] {
                let Some(dimension) = dimension else { continue };
                match dimension {
                    Dimension::Fixed(value) => modifiers.push(format!(
                        ".{axis}({}.dp)",
                        crate::generator::utils::number(value)
                    )),
                    Dimension::ScreenWidthFraction(fraction) if axis == "width" => {
                        modifiers.push(format!(
                            ".width((LocalNexaWindowMetrics.current.viewportWidthDp * {}f).dp)",
                            crate::generator::utils::number(fraction)
                        ))
                    }
                    Dimension::ScreenHeightFraction(fraction) if axis == "height" => modifiers
                        .push(format!(
                            ".height((LocalNexaWindowMetrics.current.viewportHeightDp * {}f).dp)",
                            crate::generator::utils::number(fraction)
                        )),
                    Dimension::Fill => modifiers.push(format!(
                        ".fillMax{}()",
                        if axis == "width" { "Width" } else { "Height" }
                    )),
                    Dimension::ParentFraction(fraction) => relative.push(format!(
                        "{axis} = {}f",
                        crate::generator::utils::number(fraction)
                    )),
                    _ => {}
                }
            }
            if !relative.is_empty() {
                modifiers.push(format!(
                    ".nexaParentRelativeDimension({})",
                    relative.join(", ")
                ));
            }
            let modifier = if width.is_none() && height.is_none() {
                "Modifier.weight(1f)".to_owned()
            } else {
                format!("Modifier{}", modifiers.join(""))
            };
            out.line_at(depth, format_args!("Spacer(modifier = {modifier})"));
        }
        Node::Divider { color, thickness } => out.line_at(
            depth,
            format_args!(
                "NexaDividerPrimitive(color = {}, thickness = {}.dp)",
                crate::generator::colors::expression(*color),
                super::super::engine::utils::number(*thickness)
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
            let native_shape = match shape {
                nexa_ir::ButtonShape::Capsule => {
                    "androidx.compose.foundation.shape.RoundedCornerShape(percent = 50)".to_owned()
                }
                nexa_ir::ButtonShape::Circle => {
                    "androidx.compose.foundation.shape.CircleShape".to_owned()
                }
                nexa_ir::ButtonShape::Rounded(radius) => format!(
                    "androidx.compose.foundation.shape.RoundedCornerShape({}.dp)",
                    crate::generator::utils::number(*radius)
                ),
            };
            let static_width = format!("{}.dp", crate::generator::utils::number(*width));
            let static_height = format!("{}.dp", crate::generator::utils::number(*height));
            let animated_width = match (width_expression, animation) {
                (Some(expression), Some(animation)) => {
                    let variable = format!("nexaShapeWidthState{}", out.next_id());
                    let target = format!(
                        "({}).dp.coerceAtLeast(0.dp)",
                        crate::generator::engine::expressions::expression(expression)
                    );
                    out.line_at(
                        depth,
                        format_args!(
                            "val {variable} = androidx.compose.animation.core.animateDpAsState(targetValue = {target}, animationSpec = {}, label = \"Nexa shape width\")",
                            super::controls::kotlin_animation(*animation),
                        ),
                    );
                    Some(variable)
                }
                _ => None,
            };
            let animated_height = match (height_expression, animation) {
                (Some(expression), Some(animation)) => {
                    let variable = format!("nexaShapeHeightState{}", out.next_id());
                    let target = format!(
                        "({}).dp.coerceAtLeast(0.dp)",
                        crate::generator::engine::expressions::expression(expression)
                    );
                    out.line_at(
                        depth,
                        format_args!(
                            "val {variable} = androidx.compose.animation.core.animateDpAsState(targetValue = {target}, animationSpec = {}, label = \"Nexa shape height\")",
                            super::controls::kotlin_animation(*animation),
                        ),
                    );
                    Some(variable)
                }
                _ => None,
            };

            if animated_width.is_some() || animated_height.is_some() {
                let width = animated_width.map_or_else(
                    || {
                        format!(
                            "({static_width}.roundToPx()).coerceIn(constraints.minWidth, constraints.maxWidth)"
                        )
                    },
                    |state| {
                        format!(
                            "({state}.value.roundToPx()).coerceIn(constraints.minWidth, constraints.maxWidth)"
                        )
                    },
                );
                let height = animated_height.map_or_else(
                    || {
                        format!(
                            "({static_height}.roundToPx()).coerceIn(constraints.minHeight, constraints.maxHeight)"
                        )
                    },
                    |state| {
                        format!(
                            "({state}.value.roundToPx()).coerceIn(constraints.minHeight, constraints.maxHeight)"
                        )
                    },
                );
                out.line_at(
                    depth,
                    format_args!(
                        "androidx.compose.ui.layout.Layout(content = {{}}, modifier = Modifier.background({}, shape = {})) {{ _, constraints ->",
                        crate::generator::colors::expression(*fill),
                        native_shape,
                    ),
                );
                out.line_at(depth + 1, format_args!("val widthPx = {width}"));
                out.line_at(depth + 1, format_args!("val heightPx = {height}"));
                out.line_at(depth + 1, format_args!("layout(widthPx, heightPx) {{}}"));
                out.line_at(depth, format_args!("}}"));
            } else {
                let width = width_expression.as_ref().map_or_else(
                    || static_width,
                    |expression| {
                        format!(
                            "({}).dp.coerceAtLeast(0.dp)",
                            crate::generator::engine::expressions::expression(expression)
                        )
                    },
                );
                let height = height_expression.as_ref().map_or_else(
                    || static_height,
                    |expression| {
                        format!(
                            "({}).dp.coerceAtLeast(0.dp)",
                            crate::generator::engine::expressions::expression(expression)
                        )
                    },
                );
                out.line_at(
                    depth,
                    format_args!(
                        "Spacer(modifier = Modifier.width({width}).height({height}).background({}, shape = {}))",
                        crate::generator::colors::expression(*fill),
                        native_shape,
                    ),
                );
            }
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
            actions,
            ..
        } => {
            controls::render_button(
                label,
                accessibility_label.as_deref(),
                icon.as_ref(),
                loading.as_ref(),
                disabled.as_ref(),
                *style,
                *size,
                *shape,
                tint.as_ref(),
                actions,
                false,
                false,
                depth,
                out,
            );
        }
        Node::TextInput { .. } => render_text_input_node(node, false, depth, out),
        Node::Switch {
            state,
            label,
            on_change,
        } => controls::render_switch(state, label, on_change.as_ref(), depth, out),
        Node::Slider {
            state,
            animated,
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
        } => controls::render_slider(
            state,
            *animated,
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
            ..
        } => controls::render_picker(
            items,
            item_icons,
            state,
            icon.as_ref(),
            label.as_ref(),
            tint.as_ref(),
            true,
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
            out.line_at(
                depth,
                format_args!(
                    "NexaLinearGradientPrimitive(startColor = {}, endColor = {}, horizontal = {}, reversed = {}, height = {}.dp)",
                    crate::generator::engine::colors::expression(*start_color),
                    crate::generator::engine::colors::expression(*end_color),
                    matches!(direction, nexa_ir::GradientDirection::LeadingToTrailing | nexa_ir::GradientDirection::TrailingToLeading),
                    matches!(direction, nexa_ir::GradientDirection::BottomToTop | nexa_ir::GradientDirection::TrailingToLeading),
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
        Node::NavigationBack { label, .. } => navigation::render_back(label, depth, out),
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
            ..
        } => dialogs::render_dialog(state, title, message, children, &scope, depth, out),
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
            bottom_bar::render_page_pager(state, pages, module, features, depth, out);
        }
        Node::FastList { plan } => {
            lists::render_virtualized_list(plan, module, features, depth, out);
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
                conditional::RenderScope {
                    module,
                    features,
                    depth,
                    out,
                },
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
                conditional::RenderScope {
                    module,
                    features,
                    depth,
                    out,
                },
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
            indent(out, depth);
            let mut rendered_arguments = arguments
                .iter()
                .map(|(_, argument)| crate::generator::engine::expressions::expression(argument))
                .collect::<Vec<_>>();
            if features.component_requires_system_theme(name) {
                rendered_arguments.push("nexaIsDarkTheme".to_owned());
            }
            if features.component_requires_navigation(name) {
                rendered_arguments.push("navController".to_owned());
            }
            if let Some(component_ref) = component_ref {
                rendered_arguments.push(crate::generator::engine::expressions::expression(
                    component_ref,
                ));
            }
            out.push_str(&format!(
                "{}({})",
                nexa_codegen::names::component_name(name),
                rendered_arguments.join(", ")
            ));
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
                        "{argument_name} = {}",
                        crate::generator::engine::expressions::expression(argument)
                    )
                })
                .collect::<Vec<_>>();
            rendered_arguments.extend(event_handlers.iter().map(|handler| {
                format!(
                    "{} = {}",
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

fn kotlin_insets(prefix: &str, insets: Option<&EdgeInsets>) -> String {
    let insets = insets.cloned().unwrap_or_default();
    let value = |expression: Option<&Box<Expr>>| {
        expression
            .map(|expression| {
                format!(
                    "({}).toFloat().dp",
                    crate::generator::engine::expressions::expression(expression)
                )
            })
            .unwrap_or_else(|| "0.dp".to_owned())
    };
    [
        ("Top", insets.top.as_ref()),
        ("Leading", insets.leading.as_ref()),
        ("Bottom", insets.bottom.as_ref()),
        ("Trailing", insets.trailing.as_ref()),
    ]
    .into_iter()
    .map(|(edge, expression)| format!("{prefix}{edge} = {}", value(expression)))
    .collect::<Vec<_>>()
    .join(", ")
}

fn kotlin_padding_values(insets: &EdgeInsets) -> String {
    let value = |expression: Option<&Box<Expr>>| {
        expression
            .map(|expression| {
                format!(
                    "androidx.compose.ui.unit.Dp(({}).toFloat())",
                    crate::generator::engine::expressions::expression(expression)
                )
            })
            .unwrap_or_else(|| "androidx.compose.ui.unit.Dp(0f)".to_owned())
    };
    format!(
        "androidx.compose.foundation.layout.PaddingValues(start = {}, top = {}, end = {}, bottom = {})",
        value(insets.leading.as_ref()),
        value(insets.top.as_ref()),
        value(insets.trailing.as_ref()),
        value(insets.bottom.as_ref()),
    )
}

pub(crate) fn render_node_in_row(
    node: &Node,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    match node {
        Node::TextInput { .. } => render_text_input_node(node, true, depth, out),
        Node::Layout {
            kind,
            gap,
            style,
            children,
        } if matches!(style.width, Some(nexa_ir::Dimension::Fill)) => {
            layout::render_row_child_layout(
                *kind,
                *gap,
                style,
                children,
                &RenderScope { module, features },
                depth,
                out,
            )
        }
        _ => render_node(node, module, features, depth, out),
    }
}

pub(crate) fn render_node_in_form_row(
    node: &Node,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    match node {
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
            actions,
            ..
        } => controls::render_button(
            label,
            accessibility_label.as_deref(),
            icon.as_ref(),
            loading.as_ref(),
            disabled.as_ref(),
            *style,
            *size,
            *shape,
            tint.as_ref(),
            actions,
            true,
            true,
            depth,
            out,
        ),
        _ => render_node(node, module, features, depth, out),
    }
}

fn render_text_input_node(node: &Node, weight_in_row: bool, depth: usize, out: &mut SourceWriter) {
    let Node::TextInput {
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
        min_lines,
        max_lines,
        searchable,
        actions,
        on_focus,
        on_change,
        ..
    } = node
    else {
        return;
    };

    input::render_text_input(
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
            font: *font,
            font_size: *font_size,
            font_family: font_family.as_deref(),
            text_color: *text_color,
            placeholder_color: *placeholder_color,
            min_lines: *min_lines,
            max_lines: *max_lines,
            searchable: *searchable,
            horizontal_padding: None,
            weight_in_row,
            actions,
            on_focus: on_focus.as_deref(),
            on_change: on_change.as_ref(),
        },
        depth,
        out,
    );
}

pub(crate) fn render_children(
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    match children {
        [] => {}
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
