use nexa_ir::{ColorValue, Module, Node};

pub(super) fn uses_fast_list(module: &Module) -> bool {
    module.body.iter().any(contains_fast_list)
        || module
            .screens
            .iter()
            .any(|screen| screen.body.iter().any(contains_fast_list))
}

fn contains_fast_list(node: &Node) -> bool {
    match node {
        Node::FastList { .. } => true,
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::KeyboardAware { children } => children.iter().any(contains_fast_list),
        Node::Text { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
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
    own_color_is_adaptive
        || node_children(node)
            .iter()
            .any(|child| contains_adaptive_color(child))
}

fn node_children(node: &Node) -> &[Node] {
    match node {
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::KeyboardAware { children }
        | Node::FastList { children, .. } => children,
        _ => &[],
    }
}
