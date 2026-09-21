use nexa_ir::Node;

pub(super) fn contains_image(node: &Node) -> bool {
    match node {
        Node::Image { .. } => true,
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::KeyboardAware { children, .. }
        | Node::FastList { children, .. } => children.iter().any(contains_image),
        Node::Text(_)
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
        | Node::Text(_)
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
        | Node::Text(_)
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
        | Node::Text(_)
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
        | Node::Text(_)
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::NavigationStack { .. } => false,
    }
}
