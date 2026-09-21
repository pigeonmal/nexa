use crate::Node;

/// Visits a node sequence and its nested children in preorder without building
/// an intermediate list.
pub fn walk_nodes(nodes: &[Node], visit: &mut impl FnMut(&Node)) {
    for node in nodes {
        visit(node);
        match node {
            Node::Layout { children, .. }
            | Node::Pressable { children, .. }
            | Node::NavigationLink { children, .. }
            | Node::KeyboardAware { children }
            | Node::FastList { children, .. } => walk_nodes(children, visit),
            Node::If {
                then_body,
                else_body,
                ..
            } => {
                walk_nodes(then_body, visit);
                if let Some(else_body) = else_body {
                    walk_nodes(else_body, visit);
                }
            }
            Node::Text { .. }
            | Node::Button { .. }
            | Node::TextInput { .. }
            | Node::Switch { .. }
            | Node::Image { .. }
            | Node::NavigationStack { .. }
            | Node::ComponentCall { .. } => {}
        }
    }
}
