use nexa_ir::{Module, Node};

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
        Node::Text(_)
        | Node::Button { .. }
        | Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
        | Node::NavigationStack { .. } => false,
    }
}
