use std::collections::HashSet;

use nexa_ir::{ColorValue, Module, Node};

pub(super) fn contains_image(node: &Node) -> bool {
    match node {
        Node::Image { .. } => true,
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::KeyboardAware { children, .. }
        | Node::FastList { children, .. } => children.iter().any(contains_image),
        Node::Text { .. }
        | Node::ComponentCall { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::NavigationStack { .. } => false,
    }
}

pub(super) fn contains_placeholder(node: &Node) -> bool {
    match node {
        Node::Image {
            placeholder: Some(_),
            ..
        } => true,
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::KeyboardAware { children, .. }
        | Node::FastList { children, .. } => children.iter().any(contains_placeholder),
        Node::Image { .. }
        | Node::Text { .. }
        | Node::ComponentCall { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::NavigationStack { .. } => false,
    }
}

pub(super) fn contains_navigation_link(node: &Node) -> bool {
    match node {
        Node::NavigationLink { .. } => true,
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::KeyboardAware { children, .. }
        | Node::FastList { children, .. } => children.iter().any(contains_navigation_link),
        Node::Image { .. }
        | Node::Text { .. }
        | Node::ComponentCall { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::NavigationStack { .. } => false,
    }
}

pub(super) fn contains_list(node: &Node) -> bool {
    match node {
        Node::FastList { .. } => true,
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::KeyboardAware { children, .. } => children.iter().any(contains_list),
        Node::Image { .. }
        | Node::Text { .. }
        | Node::ComponentCall { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::NavigationStack { .. } => false,
    }
}

pub(super) fn contains_keyboard_aware(node: &Node) -> bool {
    match node {
        Node::KeyboardAware { .. } => true,
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::FastList { children, .. } => children.iter().any(contains_keyboard_aware),
        Node::Image { .. }
        | Node::Text { .. }
        | Node::ComponentCall { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::NavigationStack { .. } => false,
    }
}

pub(super) fn uses_adaptive_color(module: &Module) -> bool {
    nodes_use_adaptive_color(&module.body)
        || module
            .screens
            .iter()
            .any(|screen| nodes_use_adaptive_color(&screen.body))
        || module
            .components
            .iter()
            .any(|component| nodes_use_adaptive_color(&component.body))
}

pub(super) fn component_requires_system_theme(module: &Module, name: &str) -> bool {
    fn visit(module: &Module, name: &str, visited: &mut HashSet<String>) -> bool {
        if !visited.insert(name.to_owned()) {
            return false;
        }
        let Some(component) = module
            .components
            .iter()
            .find(|component| component.name == name)
        else {
            return false;
        };
        if nodes_use_adaptive_color(&component.body) {
            return true;
        }
        let mut calls = Vec::new();
        for node in &component.body {
            collect_component_calls(node, &mut calls);
        }
        calls.iter().any(|child| visit(module, child, visited))
    }

    visit(module, name, &mut HashSet::new())
}

pub(super) fn nodes_use_adaptive_color(nodes: &[Node]) -> bool {
    nodes.iter().any(contains_adaptive_color)
}

fn contains_adaptive_color(node: &Node) -> bool {
    let own_color_is_adaptive = match node {
        Node::Layout { style, .. } => style.background,
        Node::Text { style, .. } => style.color,
        _ => None,
    }
    .is_some_and(|color| matches!(color, ColorValue::Adaptive { .. }));
    own_color_is_adaptive || children(node).iter().any(contains_adaptive_color)
}

pub(super) fn uses_font_size(module: &Module) -> bool {
    nodes_use_font_size(&module.body)
        || module
            .screens
            .iter()
            .any(|screen| nodes_use_font_size(&screen.body))
        || module
            .components
            .iter()
            .any(|component| nodes_use_font_size(&component.body))
}

pub(super) fn nodes_use_font_size(nodes: &[Node]) -> bool {
    nodes.iter().any(contains_font_size)
}

fn contains_font_size(node: &Node) -> bool {
    matches!(node, Node::Text { style, .. } if style.font_size.is_some())
        || children(node).iter().any(contains_font_size)
}

fn children(node: &Node) -> &[Node] {
    match node {
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::KeyboardAware { children }
        | Node::FastList { children, .. } => children,
        _ => &[],
    }
}

fn collect_component_calls(node: &Node, calls: &mut Vec<String>) {
    match node {
        Node::ComponentCall { name, .. } => calls.push(name.clone()),
        _ => {
            for child in children(node) {
                collect_component_calls(child, calls);
            }
        }
    }
}
