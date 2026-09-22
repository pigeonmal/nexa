use nexa_ir::{LayoutKind, Module, Node, ViewStyle};

use super::{
    accessibility, bottom_bar, colors, controls,
    expressions::text_expression,
    images, input, keyboard, layout, links, lists, navigation, refresh, sheets,
    utils::{indent, number},
};

pub(super) fn render_node(node: &Node, module: &Module, depth: usize, out: &mut String) {
    match node {
        Node::StatusBar { .. }
        | Node::Direction { .. }
        | Node::OnAppear { .. }
        | Node::OnDisappear { .. } => {}
        Node::Layout {
            kind,
            spacing,
            style,
            children,
        } => layout::render_layout(*kind, *spacing, style, children, module, depth, out),
        Node::Text { value, style } => {
            indent(out, depth);
            out.push_str(&format!("Text({})", text_expression(value)));
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
        }
        Node::Button {
            label,
            loading,
            actions,
        } => {
            controls::render_button(label, loading.as_ref(), actions, depth, out);
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
        Node::Link { url, children } => links::render_link(url, children, module, depth, out),
        Node::Accessibility {
            label,
            role,
            children,
        } => accessibility::render_accessibility(label, *role, children, module, depth, out),
        Node::KeyboardAware { children } => {
            keyboard::render_keyboard_aware(children, module, depth, out)
        }
        Node::BottomSheet { state, children } => {
            sheets::render_bottom_sheet(state, children, module, depth, out)
        }
        Node::RefreshControl {
            state,
            children,
            actions,
        } => refresh::render_refresh_control(state, children, actions, module, depth, out),
        Node::AppBottomBar { state, tabs } => {
            bottom_bar::render_app_bottom_bar(state, tabs, module, depth, out)
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
        Node::If {
            condition,
            then_body,
            else_body,
        } => {
            indent(out, depth);
            out.push_str(&format!(
                "if {} {{\n",
                super::expressions::expression(condition)
            ));
            render_children(then_body, module, depth + 1, out);
            if let Some(else_body) = else_body {
                out.push('\n');
                indent(out, depth);
                out.push_str("} else {\n");
                render_children(else_body, module, depth + 1, out);
            }
            out.push('\n');
            indent(out, depth);
            out.push('}');
        }
        Node::ComponentCall { name, arguments } => {
            indent(out, depth);
            out.push_str(&format!(
                "{}({})",
                nexa_codegen::names::component_name(name),
                arguments
                    .iter()
                    .map(|(_, argument)| super::expressions::expression(argument))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
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

pub(super) fn render_children(children: &[Node], module: &Module, depth: usize, out: &mut String) {
    match children {
        [] => {
            indent(out, depth);
            out.push_str("EmptyView()");
        }
        [node] => render_node(node, module, depth, out),
        _ => layout::render_layout(
            LayoutKind::Column,
            0.0,
            &ViewStyle::default(),
            children,
            module,
            depth,
            out,
        ),
    }
}
