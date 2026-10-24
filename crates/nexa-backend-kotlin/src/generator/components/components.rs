use nexa_codegen::SourceWriter;
use nexa_ir::{LayoutKind, Module, Node, SystemIcon, ToolbarPlacement, ViewStyle};

use crate::generator::{
    accessibility, bottom_bar, bottom_sheet, controls, dialogs, features::Features, images, input,
    keyboard, layout, links, lists, navigation, refresh, system_icons, utils::indent,
};

use super::conditional;
use super::{RenderScope, appearance};
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
            spacing,
            style,
            children,
        } => layout::render_layout(*kind, *spacing, style, children, &scope, depth, out),
        Node::Form {
            show_dividers,
            background,
            children,
        } => layout::render_form(*show_dividers, *background, children, &scope, depth, out),
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
                first_section: false,
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
                SystemIcon::SfSymbol(_) | SystemIcon::MaterialSymbol(_) => "Icon".to_owned(),
            };
            out.line_at(
                depth,
                format_args!(
                    "NexaContentUnavailablePrimitive(title = {}, description = {}, icon = {}, iconDescription = {}, titleColor = {}, descriptionColor = {}, iconTint = {}, background = {})",
                    crate::generator::engine::expressions::text_expression(title),
                    description.as_ref().map(crate::generator::engine::expressions::text_expression).unwrap_or_else(|| "null".to_owned()),
                    icon.material_reference(),
                    crate::generator::utils::kotlin_string(&icon_description),
                    title_color.as_ref().map(crate::generator::colors::expression_for_color).unwrap_or_else(|| "null".to_owned()),
                    description_color.as_ref().map(crate::generator::colors::expression_for_color).unwrap_or_else(|| "null".to_owned()),
                    icon_tint.as_ref().map(crate::generator::colors::expression_for_color).unwrap_or_else(|| "null".to_owned()),
                    background.as_ref().map(crate::generator::colors::expression_for_color).unwrap_or_else(|| "null".to_owned()),
                ),
            );
        }
        Node::Spacer => out.line_at(
            depth,
            format_args!("Spacer(modifier = Modifier.weight(1f))"),
        ),
        Node::Divider { color, thickness } => out.line_at(
            depth,
            format_args!(
                "NexaDividerPrimitive(color = {}, thickness = {}.dp)",
                crate::generator::colors::expression(*color),
                super::super::engine::utils::number(*thickness)
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
            actions,
            ..
        } => {
            controls::render_button(
                label,
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
        Node::Switch { state, label, .. } => controls::render_switch(state, label, depth, out),
        Node::Slider {
            state,
            animated,
            min,
            max,
            step,
        } => controls::render_slider(state, *animated, *min, *max, *step, depth, out),
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
        Node::KeyboardAware { dismiss, children } => {
            keyboard::render_keyboard_aware(*dismiss, children, module, features, depth, out)
        }
        Node::BottomSheet {
            state,
            content_padding,
            detents,
            snap_points,
            initial_snap_point,
            selected_snap_point,
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
                state,
                content_padding: *content_padding,
                detents,
                snap_points,
                initial_snap_point: *initial_snap_point,
                selected_snap_point: selected_snap_point.as_deref(),
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
                else_body,
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

pub(crate) fn render_node_in_row(
    node: &Node,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    if matches!(node, Node::TextInput { .. }) {
        render_text_input_node(node, true, depth, out);
    } else {
        render_node(node, module, features, depth, out);
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
        min_lines,
        max_lines,
        searchable,
        actions,
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
            min_lines: *min_lines,
            max_lines: *max_lines,
            searchable: *searchable,
            horizontal_padding: None,
            weight_in_row,
            actions,
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
