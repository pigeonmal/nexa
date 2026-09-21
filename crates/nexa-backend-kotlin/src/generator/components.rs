use nexa_ir::{LayoutKind, Module, Node, ViewStyle};

use super::{
    colors, controls,
    expressions::text_expression,
    images, input, keyboard, layout, lists, navigation,
    utils::{indent, number},
};

pub(super) fn render_node(node: &Node, module: &Module, depth: usize, out: &mut String) {
    match node {
        Node::Layout {
            kind,
            spacing,
            style,
            children,
        } => layout::render_layout(*kind, *spacing, style, children, module, depth, out),
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
        } => controls::render_pressable(*disabled, children, actions, module, depth, out),
        Node::NavigationStack { root } => {
            navigation::render_navigation_stack(module, *root, depth, out);
        }
        Node::NavigationLink {
            destination,
            children,
        } => navigation::render_link(*destination, children, module, depth, out),
        Node::KeyboardAware { children } => {
            keyboard::render_keyboard_aware(children, module, depth, out)
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
                depth,
                out,
            );
        }
    }
}

pub(super) fn render_children(children: &[Node], module: &Module, depth: usize, out: &mut String) {
    match children {
        [] => {}
        [node] => render_node(node, module, depth, out),
        _ => layout::render_layout(
            LayoutKind::View,
            0.0,
            &ViewStyle::default(),
            children,
            module,
            depth,
            out,
        ),
    }
}
