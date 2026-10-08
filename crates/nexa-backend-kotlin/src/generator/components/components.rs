use nexa_codegen::SourceWriter;
use nexa_ir::{LayoutKind, Module, Node, ToolbarPlacement, ViewStyle};

use crate::generator::{
    accessibility, bottom_bar, controls, dialogs, features::Features, images, input, keyboard,
    layout, links, lists, navigation, refresh, sheets, system_icons, utils::indent,
};

use super::RenderScope;
use super::conditional;
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
            let mode = crate::generator::engine::expressions::expression(mode);
            let system_dark = format!("nexaSystemDarkTheme{}", out.next_id());
            let dark_scheme = nexa_codegen::design_system::kotlin_color_scheme(true);
            let light_scheme = nexa_codegen::design_system::kotlin_color_scheme(false);
            out.line_at(
                depth,
                format_args!("val {system_dark} = isSystemInDarkTheme()"),
            );
            out.line_at(
                depth,
                format_args!("MaterialTheme(colorScheme = remember({mode}, {system_dark}) {{"),
            );
            out.line_at(depth + 1, format_args!("when ({mode}) {{"));
            out.line_at(depth + 2, format_args!("\"dark\" -> {dark_scheme}"));
            out.line_at(depth + 2, format_args!("\"light\" -> {light_scheme}"));
            out.line_at(
                depth + 2,
                format_args!("else -> if ({system_dark}) {dark_scheme} else {light_scheme}"),
            );
            out.line_at(depth + 1, format_args!("}}"));
            out.line_at(
                depth,
                format_args!(
                    "}}, typography = MaterialTheme.typography, shapes = MaterialTheme.shapes) {{"
                ),
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
        Node::Form { children } => layout::render_form(children, &scope, depth, out),
        Node::FormSection {
            title,
            footer,
            children,
            ..
        } => layout::render_form_section(
            title.as_ref(),
            footer.as_ref(),
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
        } => {
            out.line_at(
                depth,
                format_args!(
                    "Column(modifier = Modifier.fillMaxSize().padding(32.dp), verticalArrangement = Arrangement.Center, horizontalAlignment = Alignment.CenterHorizontally) {{"
                ),
            );
            out.line_at(
                depth + 1,
                format_args!(
                    "Icon(imageVector = {}, contentDescription = null, modifier = Modifier.size(36.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)",
                    icon.material_reference()
                ),
            );
            out.line_at(
                depth + 1,
                format_args!("Spacer(modifier = Modifier.height(8.dp))"),
            );
            out.line_at(
                depth + 1,
                format_args!(
                    "Text({}, style = MaterialTheme.typography.titleMedium, textAlign = TextAlign.Center)",
                    crate::generator::engine::expressions::text_expression(title)
                ),
            );
            out.line_at(
                depth + 1,
                format_args!("Spacer(modifier = Modifier.height(8.dp))"),
            );
            out.line_at(
                depth + 1,
                format_args!(
                    "Text({}, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)",
                    crate::generator::engine::expressions::text_expression(description)
                ),
            );
            out.line_at(depth, format_args!("}}"));
        }
        Node::Spacer => out.line_at(
            depth,
            format_args!("Spacer(modifier = Modifier.weight(1f))"),
        ),
        Node::Divider { color, thickness } => out.line_at(
            depth,
            format_args!(
                "HorizontalDivider(color = {}, thickness = {}.dp)",
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
            ..
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
                font: *font,
                min_lines: *min_lines,
                max_lines: *max_lines,
                searchable: *searchable,
                actions,
                on_change: on_change.as_ref(),
            },
            depth,
            out,
        ),
        Node::Switch { state, label, .. } => controls::render_switch(state, label, depth, out),
        Node::Slider {
            state,
            animated,
            min,
            max,
            step,
        } => controls::render_slider(state, *animated, *min, *max, *step, depth, out),
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
            ..
        } => controls::render_picker(items, state, icon.as_ref(), label.as_ref(), depth, out),
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
            let (brush, colors) = match direction {
                nexa_ir::GradientDirection::TopToBottom => ("Brush.verticalGradient", "start_end"),
                nexa_ir::GradientDirection::BottomToTop => ("Brush.verticalGradient", "end_start"),
                nexa_ir::GradientDirection::LeadingToTrailing => {
                    ("Brush.horizontalGradient", "start_end")
                }
                nexa_ir::GradientDirection::TrailingToLeading => {
                    ("Brush.horizontalGradient", "end_start")
                }
            };
            let (first, second) = if colors == "start_end" {
                (*start_color, *end_color)
            } else {
                (*end_color, *start_color)
            };
            out.line_at(
                depth,
                format_args!(
                    "Box(modifier = Modifier.fillMaxWidth().height({}.dp).background({}(colors = listOf({}, {}))))",
                    crate::generator::engine::utils::number(*height),
                    brush,
                    crate::generator::engine::colors::expression(first),
                    crate::generator::engine::colors::expression(second),
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
            partial,
            large_only,
            title,
            children,
        } => sheets::render_bottom_sheet(
            state,
            *partial,
            *large_only,
            title.as_ref(),
            children,
            module,
            features,
            depth,
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
        } => {
            conditional::render_if(
                condition,
                then_body,
                else_body.as_deref(),
                *transition,
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
        } => {
            conditional::render_when(
                value,
                cases,
                else_body,
                *transition,
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
