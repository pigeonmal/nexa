use nexa_ir::{LayoutKind, Module, Node, ViewStyle};

use super::{
    accessibility, bottom_bar, colors, controls,
    expressions::text_expression,
    features::Features,
    images, input, keyboard, layout, links, lists, navigation, refresh, sheets,
    utils::{indent, number},
};

pub(super) fn render_node(
    node: &Node,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut String,
) {
    match node {
        Node::StatusBar { .. } | Node::Direction { .. } | Node::OnAppear { .. } => {}
        Node::Layout {
            kind,
            spacing,
            style,
            children,
        } => layout::render_layout(
            *kind, *spacing, style, children, module, features, depth, out,
        ),
        Node::Text { value, style } => {
            indent(out, depth);
            out.push_str(&format!("Text({}", text_expression(value)));
            if let Some(color) = style.color {
                out.push_str(&format!(", color = {}", colors::expression(color)));
            }
            if let Some(font_size) = style.font_size {
                out.push_str(&format!(", fontSize = {}.sp", number(font_size)));
            }
            out.push(')');
        }
        Node::Button { label, actions } => {
            controls::render_button(label, actions, depth, out);
        }
        Node::TextInput {
            state,
            placeholder,
            keyboard,
            secure,
            multiline,
            autocorrect,
            capitalization,
        } => input::render_text_input(
            state,
            placeholder,
            *keyboard,
            *secure,
            *multiline,
            *autocorrect,
            *capitalization,
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
            children,
            actions,
        } => controls::render_pressable(*disabled, children, actions, module, features, depth, out),
        Node::NavigationStack { root } => {
            navigation::render_navigation_stack(module, *root, features, depth, out);
        }
        Node::NavigationLink {
            destination,
            children,
        } => navigation::render_link(*destination, children, module, features, depth, out),
        Node::Link { url, children } => {
            links::render_link(url, children, module, features, depth, out)
        }
        Node::Accessibility {
            label,
            role,
            children,
        } => accessibility::render_accessibility(
            label, *role, children, module, features, depth, out,
        ),
        Node::KeyboardAware { children } => {
            keyboard::render_keyboard_aware(children, module, features, depth, out)
        }
        Node::BottomSheet { state, children } => {
            sheets::render_bottom_sheet(state, children, module, features, depth, out)
        }
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
            index,
            item,
            children,
        } => {
            lists::render_virtualized_list(
                source,
                index,
                item.as_deref(),
                children,
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
                super::expressions::expression(condition)
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
        Node::ComponentCall { name, arguments } => {
            indent(out, depth);
            let mut rendered_arguments = arguments
                .iter()
                .map(|(_, argument)| super::expressions::expression(argument))
                .collect::<Vec<_>>();
            if features.component_requires_system_theme(name) {
                rendered_arguments.push("nexaIsDarkTheme".to_owned());
            }
            out.push_str(&format!(
                "{}({})",
                nexa_codegen::names::component_name(name),
                rendered_arguments.join(", ")
            ));
        }
    }
}

pub(super) fn render_children(
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
