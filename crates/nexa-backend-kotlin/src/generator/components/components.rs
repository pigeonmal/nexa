use nexa_ir::{LayoutKind, Module, Node, ViewStyle};

use crate::generator::{
    accessibility, bottom_bar, controls, features::Features, images, input, keyboard, layout,
    links, lists, navigation, refresh, sheets, utils::indent,
};

use crate::generator::components::text;

pub(crate) fn render_node(
    node: &Node,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
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
        } => layout::render_layout(
            *kind, *spacing, style, children, module, features, depth, out,
        ),
        Node::Text { value, style } => text::render(value, style, depth, out),
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
            autocorrect,
            capitalization,
            focused,
            max_length,
            actions,
        } => input::render_text_input(
            state,
            placeholder,
            *keyboard,
            *secure,
            *multiline,
            *autocorrect,
            *capitalization,
            focused.as_deref(),
            *max_length,
            actions,
            depth,
            out,
        ),
        Node::Switch { state, label } => controls::render_switch(state, label, depth, out),
        Node::Image {
            source,
            description,
            scale,
            placeholder,
        } => images::render_image(
            source,
            description,
            *scale,
            placeholder.as_deref(),
            depth,
            out,
        ),
        Node::Pressable {
            disabled,
            haptic,
            children,
            actions,
            long_press_actions,
        } => controls::render_pressable(
            disabled,
            *haptic,
            children,
            actions,
            long_press_actions,
            module,
            features,
            depth,
            out,
        ),
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
            module,
            features,
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
            module,
            features,
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
        Node::FastList {
            source,
            axis,
            item_extent,
            section,
            index,
            item,
            key,
            children,
            on_end_reached,
            on_scroll,
            scroll_position,
            sticky_header,
            section_header,
            refresh,
        } => {
            lists::render_virtualized_list(
                source,
                *axis,
                *item_extent,
                section.as_deref(),
                index,
                item.as_deref(),
                key.as_ref(),
                children,
                on_end_reached.as_deref(),
                on_scroll.as_deref(),
                scroll_position.as_deref(),
                sticky_header.as_deref(),
                section_header.as_deref(),
                refresh.as_ref(),
                module,
                features,
                depth,
                out,
            );
        }
        Node::If {
            condition,
            then_body,
            else_body,
        } => {
            indent(out, depth);
            out.push_str(&format!(
                "if ({}) {{\n",
                crate::generator::engine::expressions::expression(condition)
            ));
            render_children(then_body, module, features, depth + 1, out);
            if let Some(else_body) = else_body {
                out.push('\n');
                indent(out, depth);
                out.push_str("} else {\n");
                render_children(else_body, module, features, depth + 1, out);
            }
            out.push('\n');
            indent(out, depth);
            out.push('}');
        }
        Node::When {
            value,
            cases,
            else_body,
        } => {
            indent(out, depth);
            out.push_str(&format!(
                "when ({}) {{\n",
                crate::generator::engine::expressions::expression(value)
            ));
            for case in cases {
                indent(out, depth + 1);
                out.push_str(&format!(
                    "{} -> {{\n",
                    crate::generator::engine::expressions::expression(&case.value)
                ));
                render_children(&case.body, module, features, depth + 2, out);
                out.push('\n');
                indent(out, depth + 1);
                out.push_str("}\n");
            }
            indent(out, depth + 1);
            out.push_str("else -> {\n");
            render_children(else_body, module, features, depth + 2, out);
            out.push('\n');
            indent(out, depth + 1);
            out.push('}');
            out.push('\n');
            indent(out, depth);
            out.push('}');
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
    out: &mut String,
) {
    match children {
        [] => {}
        [node] => render_node(node, module, features, depth, out),
        _ => layout::render_layout(
            LayoutKind::Column,
            0.0,
            &ViewStyle::default(),
            children,
            module,
            features,
            depth,
            out,
        ),
    }
}
