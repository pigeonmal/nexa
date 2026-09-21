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
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::NavigationStack { .. } => false,
    }
}

pub(super) fn uses_adaptive_color(module: &Module) -> bool {
    module.body.iter().any(contains_adaptive_color)
        || module
            .screens
            .iter()
            .any(|screen| screen.body.iter().any(contains_adaptive_color))
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
    module.body.iter().any(contains_font_size)
        || module
            .screens
            .iter()
            .any(|screen| screen.body.iter().any(contains_font_size))
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
