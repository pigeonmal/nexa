use nexa_ir::{ColorValue, Component, Module, Node};

pub(super) fn uses_fast_list(module: &Module) -> bool {
    module.body.iter().any(contains_fast_list)
        || module
            .screens
            .iter()
            .any(|screen| screen.body.iter().any(contains_fast_list))
        || module
            .components
            .iter()
            .any(|component| component.body.iter().any(contains_fast_list))
}

fn contains_fast_list(node: &Node) -> bool {
    match node {
        Node::FastList { .. } => true,
        Node::If {
            then_body,
            else_body,
            ..
        } => {
            then_body.iter().any(contains_fast_list)
                || else_body
                    .as_ref()
                    .is_some_and(|body| body.iter().any(contains_fast_list))
        }
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::KeyboardAware { children } => children.iter().any(contains_fast_list),
        Node::Text { .. }
        | Node::ComponentCall { .. }
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
        | Node::NavigationStack { .. } => false,
    }
}

pub(super) fn app_uses_adaptive_color(module: &Module) -> bool {
    nodes_use_adaptive_color(&module.body)
        || module
            .screens
            .iter()
            .any(|screen| nodes_use_adaptive_color(&screen.body))
}

pub(super) fn component_uses_adaptive_color(component: &Component) -> bool {
    nodes_use_adaptive_color(&component.body)
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
    own_color_is_adaptive
        || match node {
            Node::If {
                then_body,
                else_body,
                ..
            } => {
                nodes_use_adaptive_color(then_body)
                    || else_body
                        .as_ref()
                        .is_some_and(|body| nodes_use_adaptive_color(body))
            }
            _ => node_children(node)
                .iter()
                .any(|child| contains_adaptive_color(child)),
        }
}

fn node_children(node: &Node) -> &[Node] {
    match node {
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::KeyboardAware { children }
        | Node::FastList { children, .. } => children,
        Node::If { .. } => &[],
        _ => &[],
    }
}
